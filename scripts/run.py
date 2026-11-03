"""Run from source; keep the environment outside synchronized project files."""
import hashlib
import os
import shutil
import subprocess
import sys
import venv
from pathlib import Path


def environment():
    if sys.version_info < (3, 12):  # noqa: UP036 - bootstrap can run before package installation.
        raise SystemExit('VideoDock требует Python 3.12 или новее.')
    root = Path(__file__).resolve().parents[1]
    if sys.platform == 'win32':
        base = Path(os.environ.get('LOCALAPPDATA', Path.home() / 'AppData/Local')) / 'VideoDock'
    elif sys.platform == 'darwin':
        base = Path.home() / 'Library/Application Support/VideoDock'
    else:
        base = Path(os.environ.get('XDG_DATA_HOME', Path.home() / '.local/share')) / 'videodock'
    env_path = base / 'venv'
    python = env_path / ('Scripts/python.exe' if os.name == 'nt' else 'bin/python')
    stamp = base / 'source-install.txt'
    digest = hashlib.sha256((root / 'pyproject.toml').read_bytes()).hexdigest()
    signature = f'{root}\n{digest}\n'
    if not python.exists():
        venv.EnvBuilder(with_pip=True).create(env_path)
    if not stamp.exists() or stamp.read_text() != signature:
        uv = shutil.which('uv')
        command = [uv, 'pip', 'install', '--python', str(python)] if uv else [str(python), '-m', 'pip', 'install']
        subprocess.run([*command, '-e', str(root)], check=True)
        stamp.write_text(signature)
    return python


if __name__ == '__main__':
    raise SystemExit(subprocess.call([str(environment()), '-m', 'videodock', *sys.argv[1:]]))
