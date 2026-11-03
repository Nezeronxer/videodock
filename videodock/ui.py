import json
import math
import os
import signal
import sys
from pathlib import Path

from PySide6.QtCore import QLockFile, QProcess, QStandardPaths, Qt, QTimer, QUrl
from PySide6.QtGui import QColor, QDesktopServices, QFont, QIcon, QPainter, QPixmap
from PySide6.QtWidgets import (
    QApplication,
    QCheckBox,
    QComboBox,
    QFileDialog,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QMainWindow,
    QMessageBox,
    QPlainTextEdit,
    QProgressBar,
    QPushButton,
    QScrollArea,
    QSizePolicy,
    QVBoxLayout,
    QWidget,
)

from . import __version__
from .core import Settings, Store, validate_url

STATUS = {'queued': 'В очереди', 'downloading': 'Загружается', 'processing': 'Обработка медиа',
          'done': 'Готово', 'cancelled': 'Отменено', 'error': 'Ошибка', 'interrupted': 'Прервано'}
ACTIVE = {'queued', 'downloading', 'processing'}
STYLE = '''
QWidget { background:#f5f5f0; color:#222b25; font-size:14px; }
QMainWindow { background:#f5f5f0; }
QFrame#sidebar { background:#e9ece3; border-right:1px solid #dce0d6; }
QFrame#sidebar QLabel { background:transparent; }
QLabel#brand { font-size:25px; font-weight:750; letter-spacing:-1px; }
QLabel#eyebrow { color:#6c796b; font-size:11px; font-weight:650; letter-spacing:2px; }
QLabel#heading { font-size:30px; font-weight:720; letter-spacing:-1px; }
QLabel#muted { color:#737e73; }
QLabel#section { font-size:18px; font-weight:650; }
QFrame#panel, QFrame#card { background:#fff; border:1px solid #dde2d8; border-radius:12px; }
QFrame#panel QLabel, QFrame#card QLabel { background:transparent; }
QPlainTextEdit, QLineEdit, QComboBox {
 background:#fafbf8; border:1px solid #d9dfd3; border-radius:7px; padding:9px; selection-background-color:#c0dd8c;
}
QPlainTextEdit:focus, QLineEdit:focus, QComboBox:focus { border:1px solid #5c8249; }
QComboBox::drop-down { border:none; width:24px; }
QComboBox QAbstractItemView { background:white; selection-background-color:#dfeacb; }
QPushButton { background:#edf0e8; border:1px solid transparent; border-radius:7px; padding:9px 13px; font-weight:550; }
QPushButton:hover { background:#dfe6d7; }
QPushButton:focus { border:1px solid #61804c; }
QPushButton:disabled { color:#98a091; background:#f0f2ec; }
QPushButton#primary { background:#285b3c; color:white; padding:12px 20px; }
QPushButton#primary:hover { background:#1c472e; }
QPushButton#nav { text-align:left; background:transparent; padding:12px; }
QPushButton#nav:checked { background:#d9e3cb; color:#244a2a; }
QPushButton#small { padding:6px 10px; font-size:12px; }
QProgressBar { border:none; border-radius:3px; background:#e9eee3; max-height:5px; min-height:5px; }
QProgressBar::chunk { background:#6b9257; border-radius:3px; }
QCheckBox { background:transparent; spacing:7px; }
QScrollArea { border:none; background:transparent; }
QScrollBar:vertical { width:9px; background:transparent; }
QScrollBar::handle:vertical { background:#ced5c6; border-radius:4px; min-height:30px; }
QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical { height:0; }
'''


def label(text='', name='', wrap=False):
    widget = QLabel(text)
    widget.setTextFormat(Qt.TextFormat.PlainText)
    widget.setObjectName(name)
    widget.setWordWrap(wrap)
    return widget


def button(text, callback, name=''):
    widget = QPushButton(text)
    widget.setObjectName(name)
    widget.clicked.connect(callback)
    return widget


def app_icon():
    pixmap = QPixmap(128, 128)
    pixmap.fill(Qt.GlobalColor.transparent)
    painter = QPainter(pixmap)
    painter.setRenderHint(QPainter.RenderHint.Antialiasing)
    painter.setBrush(QColor('#285b3c'))
    painter.setPen(Qt.PenStyle.NoPen)
    painter.drawRoundedRect(0, 0, 128, 128, 30, 30)
    painter.setPen(QColor('#e2efb7'))
    painter.setFont(QFont('sans-serif', 62, QFont.Weight.Bold))
    painter.drawText(pixmap.rect(), Qt.AlignmentFlag.AlignCenter, '↓')
    painter.end()
    return QIcon(pixmap)


class JobCard(QFrame):
    def __init__(self, job, window):
        super().__init__()
        self.setObjectName('card')
        self.job_id = job['id']
        layout = QVBoxLayout(self)
        layout.setContentsMargins(17, 14, 17, 14)
        row = QHBoxLayout()
        self.title = label(wrap=True)
        self.title.setStyleSheet('font-size:15px;font-weight:650;')
        row.addWidget(self.title, 1)
        self.badge = label(name='muted')
        row.addWidget(self.badge)
        layout.addLayout(row)
        self.detail = label(wrap=True, name='muted')
        layout.addWidget(self.detail)
        self.progress = QProgressBar()
        self.progress.setRange(0, 100)
        self.progress.setTextVisible(False)
        layout.addWidget(self.progress)
        actions = QHBoxLayout()
        self.state = label(name='muted')
        actions.addWidget(self.state, 1)
        self.open_button = button('Открыть', lambda: window.open_job(self.job_id), 'small')
        self.folder_button = button('Папка', lambda: window.open_job(self.job_id, folder=True), 'small')
        self.cancel_button = button('Отменить', lambda: window.cancel(self.job_id), 'small')
        self.retry_button = button('Повторить', lambda: window.retry(self.job_id), 'small')
        self.remove_button = button('Убрать', lambda: window.remove(self.job_id), 'small')
        for widget in (self.open_button, self.folder_button, self.cancel_button, self.retry_button, self.remove_button):
            actions.addWidget(widget)
        layout.addLayout(actions)
        self.update_job(job)

    def update_job(self, job):
        state = job['state']
        self.title.setText(job['title'][:300])
        self.badge.setText('MP3' if job['options']['mode'] == 'audio' else 'ВИДЕО')
        self.detail.setText(job['detail'] or job['url'])
        self.state.setText(STATUS[state])
        self.progress.setValue(round(job['percent']))
        self.progress.setVisible(state in ACTIVE)
        self.cancel_button.setVisible(state in ACTIVE)
        self.retry_button.setVisible(state not in ACTIVE)
        self.remove_button.setVisible(state not in ACTIVE)
        self.open_button.setVisible(state == 'done')
        self.open_button.setEnabled(bool(job['filepath']) and Path(job['filepath']).is_file())
        self.folder_button.setVisible(state == 'done')


class MainWindow(QMainWindow):
    def __init__(self, store):
        super().__init__()
        self.store = store
        self.cards = {}
        self.filter = 'all'
        self.process = None
        self.active_id = None
        self.cancelled = False
        self.closing = False
        self.buffer = b''
        self.error_message = ''
        self.got_done = False
        self.setWindowTitle(f'VideoDock {__version__}')
        self.setWindowIcon(app_icon())
        self.resize(1080, 800)
        self.setMinimumSize(860, 680)
        self.setStyleSheet(STYLE)
        root = QWidget()
        self.setCentralWidget(root)
        outer = QHBoxLayout(root)
        outer.setContentsMargins(0, 0, 0, 0)
        sidebar = QFrame()
        sidebar.setObjectName('sidebar')
        sidebar.setFixedWidth(184)
        nav = QVBoxLayout(sidebar)
        nav.setContentsMargins(18, 28, 18, 24)
        nav.addWidget(label('VideoDock', 'brand'))
        nav.addWidget(label('ТВОЯ МЕДИАТЕКА', 'eyebrow'))
        nav.addSpacing(36)
        self.nav_buttons = {}
        for key, text in [('all', 'Все загрузки'), ('active', 'В процессе'), ('done', 'Готовые'), ('error', 'Нужны действия')]:
            widget = button(text, lambda checked=False, key=key: self.set_filter(key), 'nav')
            widget.setCheckable(True)
            widget.setChecked(key == 'all')
            self.nav_buttons[key] = widget
            nav.addWidget(widget)
        nav.addStretch()
        nav.addWidget(label('Видео и музыка.\nВ выбранной папке.', 'muted', True))
        nav.addSpacing(14)
        nav.addWidget(label(f'VERSION {__version__}', 'eyebrow'))
        outer.addWidget(sidebar)
        content = QWidget()
        body = QVBoxLayout(content)
        body.setContentsMargins(30, 27, 30, 20)
        body.setSpacing(14)
        body.addWidget(label('СОХРАНИ ТО, ЧТО ВАЖНО', 'eyebrow'))
        body.addWidget(label('Ссылку сюда. Видео - к тебе.', 'heading'))
        body.addWidget(label('Добавь видео или плейлист. Выбери качество. Остальное - в очереди.', 'muted', True))
        panel = QFrame()
        panel.setObjectName('panel')
        form = QVBoxLayout(panel)
        form.setContentsMargins(20, 18, 20, 18)
        form.setSpacing(10)
        title_row = QHBoxLayout()
        title_row.addWidget(label('Новая загрузка', 'section'), 1)
        title_row.addWidget(button('Вставить', self.paste, 'small'))
        form.addLayout(title_row)
        self.urls = QPlainTextEdit()
        self.urls.setPlaceholderText('https://…\nКаждая ссылка - с новой строки')
        self.urls.setAccessibleName('Ссылки для загрузки')
        self.urls.setMaximumHeight(78)
        form.addWidget(self.urls)
        options = QHBoxLayout()
        self.mode = QComboBox()
        self.mode.setAccessibleName('Формат')
        self.mode.addItem('Видео', 'video')
        self.mode.addItem('Аудио · MP3', 'audio')
        self.quality = QComboBox()
        self.quality.setAccessibleName('Качество')
        for value, text in [(0, 'Лучшее доступное'), (2160, '4K · 2160p'), (1440, '2K · 1440p'),
                            (1080, 'Full HD · 1080p'), (720, 'HD · 720p'), (480, '480p')]:
            self.quality.addItem(text, value)
        options.addWidget(self.mode, 1)
        options.addWidget(self.quality, 1)
        form.addLayout(options)
        folders = QHBoxLayout()
        self.folder = QLineEdit()
        self.folder.setAccessibleName('Папка загрузок')
        folders.addWidget(self.folder, 1)
        folders.addWidget(button('Выбрать папку', self.choose_folder, 'small'))
        form.addLayout(folders)
        self.advanced = QWidget()
        advanced_layout = QVBoxLayout(self.advanced)
        advanced_layout.setContentsMargins(0, 4, 0, 4)
        self.playlist = QCheckBox('Загружать весь плейлист')
        self.subtitles = QCheckBox('Субтитры RU / EN, если доступны')
        checks = QHBoxLayout()
        checks.addWidget(self.playlist)
        checks.addWidget(self.subtitles)
        advanced_layout.addLayout(checks)
        self.proxy = QLineEdit()
        self.proxy.setPlaceholderText('Прокси (необязательно): socks5://127.0.0.1:1080')
        self.proxy.setAccessibleName('Адрес прокси')
        advanced_layout.addWidget(self.proxy)
        form.addWidget(self.advanced)
        self.advanced.hide()
        bottom = QHBoxLayout()
        self.more = button('Дополнительно', lambda: self.advanced.setVisible(not self.advanced.isVisible()), 'small')
        bottom.addWidget(self.more)
        bottom.addStretch()
        self.add_button = button('Добавить в очередь', self.add_requested, 'primary')
        bottom.addWidget(self.add_button)
        form.addLayout(bottom)
        body.addWidget(panel)
        self.notice = label(wrap=True)
        self.notice.setStyleSheet('color:#a24c36;')
        self.notice.hide()
        body.addWidget(self.notice)
        queue_heading = QHBoxLayout()
        queue_heading.addWidget(label('Загрузки', 'section'), 1)
        self.counter = label(name='muted')
        queue_heading.addWidget(self.counter)
        body.addLayout(queue_heading)
        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        holder = QWidget()
        self.list_layout = QVBoxLayout(holder)
        self.list_layout.setContentsMargins(0, 0, 5, 0)
        self.list_layout.setSpacing(10)
        self.empty = label('Здесь появятся твои загрузки.\nДобавь первую ссылку выше.', 'muted', True)
        self.empty.setAlignment(Qt.AlignmentFlag.AlignCenter)
        self.empty.setMinimumHeight(110)
        self.list_layout.addWidget(self.empty)
        self.list_layout.addStretch()
        scroll.setWidget(holder)
        scroll.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        body.addWidget(scroll, 1)
        outer.addWidget(content, 1)
        settings = store.settings()
        self.folder.setText(settings.folder)
        self.mode.setCurrentIndex(self.mode.findData(settings.mode))
        self.quality.setCurrentIndex(self.quality.findData(settings.quality))
        self.playlist.setChecked(settings.playlist)
        self.subtitles.setChecked(settings.subtitles)
        self.proxy.setText(settings.proxy)
        self.mode.currentIndexChanged.connect(lambda: self.quality.setEnabled(self.mode.currentData() == 'video'))
        self.quality.setEnabled(settings.mode == 'video')
        self.refresh()

    def current_settings(self):
        return Settings(folder=self.folder.text().strip(), mode=self.mode.currentData(),
                        quality=self.quality.currentData(), playlist=self.playlist.isChecked(),
                        subtitles=self.subtitles.isChecked(), proxy=self.proxy.text().strip()).validate()

    def show_error(self, text):
        self.notice.setText(str(text))
        self.notice.show()

    def paste(self):
        self.urls.setPlainText(QApplication.clipboard().text())
        self.urls.setFocus()

    def choose_folder(self):
        folder = QFileDialog.getExistingDirectory(self, 'Папка загрузок', self.folder.text())
        if folder:
            self.folder.setText(folder)

    def add_requested(self):
        try:
            settings = self.current_settings()
            links = [validate_url(line) for line in self.urls.toPlainText().splitlines() if line.strip()]
            if not links:
                raise ValueError('Добавь хотя бы одну ссылку.')
            if len(links) > 100:
                raise ValueError('За один раз можно добавить до 100 ссылок.')
            self.store.save_settings(settings)
            for url in dict.fromkeys(links):
                self.store.add(url, settings)
            self.urls.clear()
            self.notice.hide()
            self.set_filter('all')
            self.start_next()
        except (ValueError, OSError) as exc:
            self.show_error(exc)

    def set_filter(self, value):
        self.filter = value
        for key, widget in self.nav_buttons.items():
            widget.setChecked(key == value)
        self.refresh()

    def refresh(self):
        jobs = self.store.jobs()
        ids = {job['id'] for job in jobs}
        for job_id in set(self.cards) - ids:
            self.cards.pop(job_id).deleteLater()
        visible = 0
        for job in reversed(jobs):
            if job['id'] not in self.cards:
                card = JobCard(job, self)
                self.cards[job['id']] = card
                self.list_layout.insertWidget(0, card)
            card = self.cards[job['id']]
            card.update_job(job)
            state = job['state']
            show = (self.filter == 'all' or (self.filter == 'active' and state in ACTIVE)
                    or self.filter == state or (self.filter == 'error' and state in ('error', 'interrupted', 'cancelled')))
            card.setVisible(show)
            visible += show
        self.empty.setVisible(visible == 0)
        self.counter.setText(f'{len(jobs)} всего · {sum(j["state"] == "done" for j in jobs)} готово')

    def start_next(self):
        if self.process is not None or self.closing:
            return
        queued = [job for job in reversed(self.store.jobs()) if job['state'] == 'queued']
        if not queued:
            return
        job = queued[0]
        self.active_id = job['id']
        self.cancelled, self.got_done = False, False
        self.buffer, self.error_message = b'', ''
        self.store.update(job['id'], state='downloading', detail='Получаю данные о медиа…', percent=0)
        process = QProcess(self)
        self.process = process
        process.setWorkingDirectory(str(Path(__file__).resolve().parent.parent))
        process.readyReadStandardOutput.connect(self.read_output)
        process.readyReadStandardError.connect(lambda: process.readAllStandardError())
        process.finished.connect(lambda code, status: self.finished(process, code))
        process.errorOccurred.connect(lambda error: self.failed_start(process, error))
        if getattr(sys, 'frozen', False):
            name = 'VideoDockWorker.exe' if os.name == 'nt' else 'VideoDockWorker'
            executable = Path(sys._MEIPASS) / 'worker' / 'VideoDockWorker' / name
            args = []
        else:
            executable = sys.executable
            args = ['-m', 'videodock', '--worker']
        process.start(str(executable), args)
        process.write(json.dumps({'url': job['url'], 'settings': job['options']}).encode())
        process.closeWriteChannel()
        self.refresh()

    def read_output(self):
        if self.process is None:
            return
        self.buffer += bytes(self.process.readAllStandardOutput())
        if len(self.buffer) > 1_048_576:
            self.error_message = 'Слишком большой ответ движка загрузки.'
            self.stop_process()
            return
        while b'\n' in self.buffer:
            line, self.buffer = self.buffer.split(b'\n', 1)
            try:
                event = json.loads(line)
                self.handle_event(event)
            except (ValueError, TypeError, KeyError):
                self.error_message = 'Движок вернул некорректный ответ.'
        self.refresh()

    def handle_event(self, event):
        kind = event['event']
        job_id = self.active_id
        if job_id is None:
            return
        if kind == 'progress':
            percent = float(event['percent'])
            speed = float(event['speed'])
            if not math.isfinite(percent) or not math.isfinite(speed):
                raise ValueError('Invalid progress')
            speed_text = f'{speed / 1048576:.1f} МБ/с' if speed >= 1048576 else f'{speed / 1024:.0f} КБ/с'
            eta = event.get('eta')
            detail = f'{percent:.0f}% · {speed_text}'
            if isinstance(eta, (int, float)) and math.isfinite(eta):
                detail += f' · осталось {int(eta) // 60}:{int(eta) % 60:02}'
            if event.get('playlist_index'):
                detail = f'Элемент {event["playlist_index"]} · {detail}'
            self.store.update(job_id, state='downloading', percent=max(0, min(100, percent)),
                              title=str(event['title'])[:300], detail=detail)
        elif kind == 'processing':
            values = {'state': 'processing', 'detail': 'Собираю готовый файл…'}
            if event.get('title'):
                values['title'] = str(event['title'])[:300]
            self.store.update(job_id, **values)
        elif kind == 'file':
            self.store.update(job_id, filepath=event['path'])
        elif kind == 'done':
            self.got_done = True
            self.store.update(job_id, detail=f'Сохранено файлов: {event["count"]}', percent=100)
        elif kind == 'error':
            self.error_message = str(event['message'])[:1500]

    def failed_start(self, process, error):
        if error == QProcess.ProcessError.FailedToStart:
            self.error_message = 'Не удалось запустить движок загрузки.'
            self.finished(process, 1)

    def finished(self, process, code):
        if self.process is not process:
            return
        self.read_output()
        stop_group = process.property('stopGroup')
        if stop_group:
            try:
                # The worker may exit before a child that ignored SIGTERM.
                os.killpg(stop_group, signal.SIGKILL)
            except ProcessLookupError:
                pass
        job_id = self.active_id
        if self.cancelled:
            self.store.update(job_id, state='cancelled', detail='Остановлено. Повтор продолжит загрузку, если сервер поддерживает.')
        elif code == 0 and self.got_done:
            self.store.update(job_id, state='done', percent=100)
        else:
            self.store.update(job_id, state='error', detail=self.error_message or 'Загрузка неожиданно завершилась. Нажми «Повторить».')
        self.process, self.active_id = None, None
        process.deleteLater()
        self.refresh()
        QTimer.singleShot(0, self.start_next)

    def stop_process(self):
        process = self.process
        if process is None or process.state() == QProcess.ProcessState.NotRunning:
            return
        pid = process.processId()
        if os.name == 'posix':
            try:
                if os.getpgid(pid) == pid:
                    process.setProperty('stopGroup', pid)
                    os.killpg(pid, signal.SIGTERM)
                else:
                    process.terminate()
            except ProcessLookupError:
                pass
        else:
            # taskkill addresses the owned process tree, including FFmpeg.
            killer = QProcess(self)
            killer.start('taskkill', ['/PID', str(pid), '/T', '/F'])
            killer.waitForFinished(3000)
            if process.state() != QProcess.ProcessState.NotRunning:
                process.kill()
        QTimer.singleShot(1500, lambda: self.force_stop(process))

    def force_stop(self, process):
        if self.process is not process or process.state() == QProcess.ProcessState.NotRunning:
            return
        pid = process.processId()
        try:
            if os.name == 'posix' and os.getpgid(pid) == pid:
                os.killpg(pid, signal.SIGKILL)
            else:
                process.kill()
        except ProcessLookupError:
            pass

    def cancel(self, job_id):
        if job_id == self.active_id:
            self.cancelled = True
            self.stop_process()
        elif self.store.job(job_id)['state'] == 'queued':
            self.store.update(job_id, state='cancelled', detail='Убрано из очереди.')
        self.refresh()

    def retry(self, job_id):
        if self.store.job(job_id)['state'] not in ACTIVE:
            self.store.update(job_id, state='queued', detail='', percent=0)
            self.refresh()
            self.start_next()

    def remove(self, job_id):
        if self.store.job(job_id)['state'] not in ACTIVE:
            self.store.remove(job_id)
            self.refresh()

    def open_job(self, job_id, folder=False):
        job = self.store.job(job_id)
        path = Path(job['filepath']).resolve()
        root = Path(job['options']['folder']).resolve()
        if not job['filepath'] or not path.is_relative_to(root):
            self.show_error('Не удалось определить путь к сохранённому файлу.')
            return
        target = path.parent if folder else path
        if not target.exists():
            self.show_error('Файл перемещён или удалён. Проверь папку загрузок.')
        elif not QDesktopServices.openUrl(QUrl.fromLocalFile(str(target))):
            self.show_error('Системное приложение не смогло открыть файл.')

    def closeEvent(self, event):
        if self.process is not None:
            dialog = QMessageBox(self)
            dialog.setWindowTitle('Загрузка ещё идёт')
            dialog.setText('Остановить загрузку и закрыть VideoDock? Частичный файл останется для повтора.')
            stop = dialog.addButton('Остановить и закрыть', QMessageBox.ButtonRole.AcceptRole)
            keep = dialog.addButton('Продолжить загрузку', QMessageBox.ButtonRole.RejectRole)
            dialog.setDefaultButton(keep)
            dialog.exec()
            if dialog.clickedButton() != stop:
                event.ignore()
                return
            self.closing = True
            self.cancelled = True
            process = self.process
            self.stop_process()
            if not process.waitForFinished(1800):
                self.force_stop(process)
                process.waitForFinished(3000)
        self.closing = True
        try:
            self.store.save_settings(self.current_settings())
        except ValueError:
            pass
        event.accept()


def launch():
    app = QApplication(sys.argv)
    app.setApplicationName('VideoDock')
    app.setOrganizationName('Nezeronxer')
    app.setWindowIcon(app_icon())
    app.setStyle('Fusion')
    data = Path(QStandardPaths.writableLocation(QStandardPaths.StandardLocation.AppLocalDataLocation))
    data.mkdir(parents=True, exist_ok=True)
    lock = QLockFile(str(data / 'instance.lock'))
    if not lock.tryLock(100):
        QMessageBox.information(None, 'VideoDock', 'VideoDock уже запущен. Найди его окно на рабочем столе.')
        return 0
    try:
        store = Store(data / 'state.sqlite')
        window = MainWindow(store)
    except Exception as exc:
        QMessageBox.critical(None, 'Не удалось открыть VideoDock', f'Проверь каталог данных приложения:\n{data}\n\n{exc}')
        return 1
    window.show()
    result = app.exec()
    store.close()
    lock.unlock()
    return result
