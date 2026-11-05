"""Inspect only the failed application bundle; never collect user/browser profiles."""
import json
import os
from pathlib import Path
import subprocess
import sys

binary = Path(sys.argv[1])
output = Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)
if not binary.is_file():
    raise SystemExit(0)
bundle = binary.resolve().parents[2]
if bundle.suffix != '.app' or binary.resolve().parent.name != 'MacOS':
    raise SystemExit('Expected an application bundle executable')

def capture(name, arguments, seconds=30):
    with (output / name).open('w') as log:
        try:
            result = subprocess.run(arguments, stdout=log, stderr=subprocess.STDOUT, timeout=seconds)
            log.write(f'\nExit status: {result.returncode}\n')
        except subprocess.TimeoutExpired:
            log.write('\nDiagnostic deadline exceeded.\n')

files = sorted(str(p.relative_to(bundle)) for p in bundle.rglob('*') if p.is_file())
(output / 'bundle-files.json').write_text(json.dumps(files, indent=2))
helpers = [p for p in bundle.rglob('QtWebEngineProcess') if p.is_file()]
for index, helper in enumerate(helpers):
    capture(f'helper-{index}-libraries.txt', ['otool', '-L', str(helper)])
    capture(f'helper-{index}-entitlements.txt', ['codesign', '-d', '--entitlements', ':-', str(helper)])
    capture(f'helper-{index}-signature.txt', ['codesign', '--verify', '--deep', '--strict', str(helper)])
capture('webengine-system-errors.txt', ['log', 'show', '--last', '3m', '--style', 'compact',
        '--predicate', 'process == "QtWebEngineProcess" OR eventMessage CONTAINS "QtWebEngineProcess"'])
# An app archive contains distributable code/resources only, not the sibling CI keychain.
archive = output / 'VideoDock-diagnostic.zip'
try:
    subprocess.run(['ditto', '-c', '-k', '--sequesterRsrc', '--keepParent', str(bundle), str(archive)],
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=180, check=False)
except subprocess.TimeoutExpired:
    pass
