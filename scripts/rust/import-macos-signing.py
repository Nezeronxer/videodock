"""Import the application's stable signing identity into an ephemeral CI keychain."""
import base64
import os
from pathlib import Path
import shlex
import subprocess
import sys
import tempfile

certificate = os.environ.get('VIDEODOCK_MACOS_CERTIFICATE')
password = os.environ.get('VIDEODOCK_MACOS_CERT_PASSWORD')
if not certificate or not password:
    raise SystemExit('Mac signing identity: missing')
if sys.platform != 'darwin':
    raise SystemExit('Mac signing identity requires macOS')

def security(stage, *arguments):
    result = subprocess.run(['security', *arguments], capture_output=True)
    if result.returncode:
        raise SystemExit(f'Mac signing identity: {stage} failed')
    return result.stdout.decode('utf-8')

keychain = Path(os.environ['RUNNER_TEMP']) / 'videodock-signing.keychain-db'
if keychain.exists():
    raise SystemExit('Refusing to replace an existing signing keychain')
with tempfile.TemporaryDirectory(prefix='videodock-cert-') as directory:
    p12 = Path(directory) / 'identity.p12'
    p12.write_bytes(base64.b64decode(certificate, validate=True))
    p12.chmod(0o600)
    previous = shlex.split(security('search list', 'list-keychains', '-d', 'user'))
    security('create', 'create-keychain', '-p', password, str(keychain))
    security('timeout', 'set-keychain-settings', '-lut', '3600', str(keychain))
    security('unlock', 'unlock-keychain', '-p', password, str(keychain))
    security('import', 'import', str(p12), '-k', str(keychain), '-P', password, '-T', '/usr/bin/codesign')
    security('access', 'set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', password, str(keychain))
    security('search list', 'list-keychains', '-d', 'user', '-s', str(keychain), *previous)
    security('certificate', 'find-certificate', '-c', 'VideoDock Self-Signed', str(keychain))
print('Mac signing identity: ready')
