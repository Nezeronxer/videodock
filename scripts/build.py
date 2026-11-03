"""Build on the target OS; no cross-compilation or system changes."""
import os
import subprocess
import sys
from pathlib import Path


def main():
    root = Path(__file__).resolve().parents[1]
    cache = Path(os.environ.get('LOCALAPPDATA', Path.home() / '.cache')) / 'videodock-build'
    cache.mkdir(parents=True, exist_ok=True)
    output = cache / 'dist'
    common = [sys.executable, '-m', 'PyInstaller', '--noconfirm', '--clean',
              '--distpath', str(output), '--workpath', str(cache / 'work'),
              '--specpath', str(cache), '--paths', str(root)]
    subprocess.run([*common, '--name', 'VideoDockWorker', '--onedir', '--console',
                    '--collect-all', 'yt_dlp', '--collect-all', 'yt_dlp_ejs',
                    str(root / 'scripts/worker_entry.py')], check=True, cwd=root)
    subprocess.run([*common, '--name', 'VideoDock', '--onedir', '--windowed',
                    '--exclude-module', 'yt_dlp', '--exclude-module', 'yt_dlp_ejs',
                    '--add-data', f'{output / "VideoDockWorker"}{os.pathsep}worker/VideoDockWorker',
                    str(root / 'scripts/gui_entry.py')], check=True, cwd=root)
    print(f'Сборка: {output / ("VideoDock.app" if sys.platform == "darwin" else "VideoDock")}')
    print('FFmpeg и Node.js устанавливаются отдельно. Для YouTube нужен JavaScript runtime.')


if __name__ == '__main__':
    main()
