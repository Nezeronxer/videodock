"""Make deployed libraries work from both the main executable and nested Qt helpers."""
import os
from pathlib import Path
import subprocess
import sys


def bundled_dependency(dependency, frameworks):
    if dependency.startswith('@executable_path/../Frameworks/'):
        relative = dependency.removeprefix('@executable_path/../Frameworks/')
    elif dependency.startswith('@rpath/'):
        relative = dependency.removeprefix('@rpath/')
    elif dependency.startswith(('/usr/local/', '/opt/homebrew/', '/Users/')):
        parts = Path(dependency).parts
        start = next((i for i, part in enumerate(parts) if part.endswith('.framework')), None)
        relative = str(Path(*parts[start:])) if start is not None else parts[-1]
    else:
        return None
    candidate = frameworks / relative
    if not candidate.is_file():
        raise RuntimeError(f'Missing bundled library: {relative}')
    return candidate


def repair(app, identity):
    frameworks = app / 'Contents/Frameworks'
    magic = {b'\xcf\xfa\xed\xfe', b'\xce\xfa\xed\xfe', b'\xca\xfe\xba\xbe', b'\xca\xfe\xba\xbf'}
    changed = []
    for binary in sorted(app.rglob('*')):
        if binary.is_symlink() or not binary.is_file():
            continue
        with binary.open('rb') as source:
            if source.read(4) not in magic:
                continue
        listing = subprocess.check_output(['otool', '-L', str(binary)], text=True)
        own_ids = subprocess.check_output(['otool', '-D', str(binary)], text=True).splitlines()[1:]
        arguments = []
        for line in listing.splitlines()[1:]:
            dependency = line.strip().split(' (compatibility version', 1)[0]
            if not dependency or dependency in own_ids:
                continue
            target = bundled_dependency(dependency, frameworks)
            if target is not None:
                # @executable_path is the nested helper's directory, not the main app's.
                replacement = '@loader_path/' + os.path.relpath(target, binary.parent)
                arguments.extend(['-change', dependency, replacement])
        if arguments:
            subprocess.run(['install_name_tool', *arguments, str(binary)], check=True)
            changed.append(binary)
    metadata = '--preserve-metadata=entitlements,requirements,flags,runtime'
    for binary in changed:
        subprocess.run(['codesign', '--force', '--sign', identity, metadata, str(binary)], check=True)
    # Rebuild enclosing resource seals after changing a nested library/helper.
    bundles = [p for p in app.rglob('*') if not p.is_symlink() and p.is_dir()
               and p.suffix in ('.framework', '.app')]
    for bundle in sorted(bundles, key=lambda p: len(p.parts), reverse=True) + [app]:
        subprocess.run(['codesign', '--force', '--sign', identity, metadata, str(bundle)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)


if __name__ == '__main__':
    repair(Path(sys.argv[1]), sys.argv[2])
