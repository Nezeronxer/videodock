"""Install a user-level launcher. Source and virtualenv remain separately updateable."""
import os
import plistlib
import shlex
import sys
from pathlib import Path

from run import environment


def install():
    python = environment()
    if sys.platform == 'linux':
        applications = Path.home() / '.local/share/applications'
        applications.mkdir(parents=True, exist_ok=True)
        icon = Path.home() / '.local/share/icons/hicolor/128x128/apps/videodock.png'
        icon.parent.mkdir(parents=True, exist_ok=True)
        import subprocess
        subprocess.run([str(python), '-c',
                        'from PySide6.QtWidgets import QApplication; from videodock.ui import app_icon; '
                        'import sys; app=QApplication([]); app_icon().pixmap(128,128).save(sys.argv[1])', str(icon)],
                       check=True, env={**os.environ, 'QT_QPA_PLATFORM': 'offscreen'})
        executable = str(python).replace('\\', '\\\\').replace('"', '\\"').replace('%', '%%')
        (applications / 'videodock.desktop').write_text(
            '[Desktop Entry]\nType=Application\nName=VideoDock\nComment=Загрузчик видео и аудио\n'
            f'Exec="{executable}" -m videodock\nIcon=videodock\nTerminal=false\nCategories=AudioVideo;Network;\n',
            encoding='utf-8')
        print('VideoDock добавлен в меню приложений Linux.')
    elif sys.platform == 'darwin':
        app = Path.home() / 'Applications/VideoDock.app/Contents'
        executable = app / 'MacOS/VideoDock'
        executable.parent.mkdir(parents=True, exist_ok=True)
        executable.write_text(f'#!/bin/sh\nexport PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"\nexec {shlex.quote(str(python))} -m videodock "$@"\n')
        executable.chmod(0o755)
        with (app / 'Info.plist').open('wb') as file:
            plistlib.dump({'CFBundleExecutable': 'VideoDock', 'CFBundleIdentifier': 'com.nezeronxer.videodock',
                           'CFBundleName': 'VideoDock', 'CFBundleVersion': '0.1.0',
                           'CFBundlePackageType': 'APPL', 'NSHighResolutionCapable': True}, file)
        print('VideoDock добавлен в ~/Applications (macOS).')
    else:
        print('Окружение готово. Запуск: python scripts/run.py. Переносимая сборка: python scripts/build.py.')


if __name__ == '__main__':
    install()
