"""Check installer staging without requiring a Mac or changing user applications."""
import os
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile

project = Path(__file__).resolve().parents[2]
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    tools, home, target = (root / name for name in ('tools', 'home', 'build'))
    for path in (tools, home, target / 'release'):
        path.mkdir(parents=True)
    stale = target / 'release/videodock-desktop'
    stale.write_text('old binary')
    stale.chmod(0o755)
    app = root / 'staged/VideoDock.app'
    scripts = {
        'uname': '#!/bin/sh\necho Darwin\n',
        'cargo': '#!/bin/sh\nprintf "fresh binary" > "$CARGO_TARGET_DIR/release/videodock-desktop"\n',
        'ffmpeg': '#!/bin/sh\necho bundled-ffmpeg\n',
        'ffprobe': '#!/bin/sh\necho bundled-ffprobe\n',
        'macdeployqt': '#!/bin/sh\nmkdir -p "$1/Contents/Frameworks"\n: > "$1/Contents/Frameworks/libdependency.dylib"\nprintf "%s\\n" "$@" > "$PACKAGE_CALLS"\n',
        'plutil': '#!/bin/sh\nexit 0\n',
        'codesign': '#!/bin/sh\nprintf "%s\\n" "$*" >> "$PACKAGE_SIGN_CALLS"\n',
        'install_name_tool': '#!/bin/sh\nexit 0\n',
        'otool': '#!/bin/sh\nprintf "%s:\\n\\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\\n" "$2"\n',
    }
    for name, code in scripts.items():
        path = tools / name
        path.write_text(code)
        path.chmod(0o755)
    env = dict(os.environ, HOME=str(home), CARGO_TARGET_DIR=str(target), VIDEODOCK_MAC_APP=str(app), PACKAGE_CALLS=str(root / 'calls'), PACKAGE_SIGN_CALLS=str(root / 'sign-calls'), PATH=str(tools) + os.pathsep + os.environ['PATH'])
    subprocess.run(['sh', str(project / 'scripts/rust/install.sh')], env=env, check=True, capture_output=True)
    assert app.is_dir(), 'Installer ignored its isolated staging destination'
    assert not (home / 'Applications').exists(), 'Verification modified the installed application location'
    assert '-codesign=VideoDock Self-Signed' in (root / 'calls').read_text(), 'Qt deployment uses an unrelated or ad-hoc identity'
    assert '--sign VideoDock Self-Signed' in (root / 'sign-calls').read_text(), 'Bundle does not use its own named certificate'
    assert '--sign - ' not in (root / 'sign-calls').read_text(), 'Ad-hoc re-signing replaced the stable identity'
    assert (app / 'Contents/MacOS/VideoDock').read_text() == 'fresh binary', 'Installer reused an old build'
    for name in ('ffmpeg', 'ffprobe'):
        assert (app / 'Contents/MacOS' / name).is_file(), f'{name} was not bundled'
        assert '-executable=' + str(app / 'Contents/MacOS' / name) in (root / 'calls').read_text()
    with (app / 'Contents/Info.plist').open('rb') as source:
        assert plistlib.load(source)['CFBundleExecutable'] == 'VideoDock'
    for name in ('GPL-3.0.txt', 'Qt-LGPL-3.0.txt', 'BgUtils-MIT.txt', 'googlevideo-MIT.txt', 'sabr-rs-MIT.txt', 'ffmpeg-license.txt', 'ffmpeg-buildconf.txt'):
        assert (app / 'Contents/Resources/licenses' / name).is_file(), f'Missing license: {name}'
    assert (app / 'Contents/Resources/THIRD-PARTY-NOTICES.md').is_file()
    (tools / 'ffmpeg').write_text('#!/bin/sh\ncase "$1" in -L|-buildconf) exit 0;; *) exit 134;; esac\n')
    result = subprocess.run(['sh', str(project / 'scripts/rust/install.sh')], env=env, capture_output=True)
    assert result.returncode != 0, 'Installer accepted a bundled media executable that cannot start'
    (tools / 'ffmpeg').write_text(scripts['ffmpeg'])
    (tools / 'otool').write_text('#!/bin/sh\nprintf "%s:\\n\\t/opt/homebrew/lib/unbundled.dylib (compatibility version 1.0.0)\\n" "$2"\n')
    result = subprocess.run(['sh', str(project / 'scripts/rust/install.sh')], env=env, capture_output=True)
    assert result.returncode != 0 and b'Unbundled library dependency' in result.stderr, 'External library dependency was accepted'
    env['VIDEODOCK_MAC_APP'] = str(root / 'not-an-app')
    result = subprocess.run(['sh', str(project / 'scripts/rust/install.sh')], env=env, capture_output=True)
    assert result.returncode != 0 and b'absolute .app destination' in result.stderr, 'Invalid staging destination accepted'
    diagnostics = root / 'diagnostics'
    subprocess.run([sys.executable, str(project / 'scripts/rust/diagnose-macos-worker.py'),
                    str(root / 'missing-binary'), str(diagnostics)], check=True)
    assert not list(diagnostics.iterdir()), 'Missing bundle unexpectedly started the debugger'
    no_identity = {key: value for key, value in os.environ.items()
                   if key not in ('VIDEODOCK_MACOS_CERTIFICATE', 'VIDEODOCK_MACOS_CERT_PASSWORD')}
    no_identity['RUNNER_TEMP'] = str(root)
    result = subprocess.run([sys.executable, str(project / 'scripts/rust/import-macos-signing.py')],
                            env=no_identity, capture_output=True)
    assert result.returncode != 0 and b'identity: missing' in result.stderr
    assert not (root / 'videodock-signing.keychain-db').exists(), 'Missing credentials created a keychain'
print('macOS packaging contract passed: current build, isolated destination, media tools, Qt deployment and plist')
