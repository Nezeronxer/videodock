import os
import time
from pathlib import Path

os.environ.setdefault('QT_QPA_PLATFORM', 'offscreen')

from PySide6.QtWidgets import QApplication

from videodock.core import Store
from videodock.ui import MainWindow


def wait_for(app, predicate, timeout=25):
    until = time.monotonic() + timeout
    while time.monotonic() < until:
        app.processEvents()
        if predicate():
            return
        time.sleep(0.02)
    raise AssertionError('Qt condition timed out')


def test_add_download_history_and_retry(media_server, tmp_path):
    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    window.folder.setText(str(tmp_path / 'Медиа'))
    window.urls.setPlainText(media_server)
    window.add_button.click()
    wait_for(app, lambda: store.jobs() and store.jobs()[0]['state'] == 'done')
    job = store.jobs()[0]
    assert window.cards[job['id']].open_button.isEnabled()
    assert job['filepath'].endswith('.mp4')
    window.retry(job['id'])
    wait_for(app, lambda: store.jobs()[0]['state'] == 'done')
    assert len(store.jobs()) == 1
    assert len(list((tmp_path / 'Медиа').glob('*.mp4'))) == 1
    window.close()
    app.processEvents()
    store.close()


def test_invalid_link_is_reported_without_creating_job(tmp_path):
    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    window.urls.setPlainText('file:///etc/passwd')
    window.add_button.click()
    assert store.jobs() == []
    assert window.notice.text()
    window.close()
    app.processEvents()
    store.close()


def test_cancel_queued_job_and_remove_only_history(tmp_path):
    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    job = store.add('https://example.com/video', window.current_settings())
    window.refresh()
    window.cancel(job['id'])
    assert store.job(job['id'])['state'] == 'cancelled'
    window.remove(job['id'])
    assert store.jobs() == []
    window.close()
    app.processEvents()
    store.close()


def test_cancel_active_worker_and_continue_queue(media_server, tmp_path):
    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    window.folder.setText(str(tmp_path / 'media'))
    window.urls.setPlainText(media_server.replace('sample.mp4', 'slow.mp4') + '\n' + media_server)
    window.add_button.click()
    first_id = min(job['id'] for job in store.jobs())
    wait_for(app, lambda: store.job(first_id)['percent'] > 0)
    pid = window.process.processId()
    window.cancel(first_id)
    wait_for(app, lambda: store.job(first_id)['state'] == 'cancelled')
    wait_for(app, lambda: store.jobs()[0]['state'] == 'done')
    if os.name == 'posix':
        import pytest
        with pytest.raises(ProcessLookupError):
            os.kill(pid, 0)
    window.close()
    app.processEvents()
    store.close()


def test_remote_titles_are_plain_text(tmp_path):
    from PySide6.QtCore import Qt

    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    job = store.add('https://example.com/video', window.current_settings())
    text = '<img src="file:///etc/passwd">'
    store.update(job['id'], title=text)
    window.refresh()
    assert window.cards[job['id']].title.text() == text
    assert window.cards[job['id']].title.textFormat() == Qt.TextFormat.PlainText
    window.close()
    app.processEvents()
    store.close()


def test_cancel_kills_descendant_that_ignores_sigterm(tmp_path):
    import signal
    import sys

    import pytest
    from PySide6.QtCore import QProcess

    if sys.platform != 'linux':
        pytest.skip('Linux process-group regression test')
    app = QApplication.instance() or QApplication([])
    store = Store(tmp_path / 'state.sqlite')
    window = MainWindow(store)
    job = store.add('https://example.com/video', window.current_settings())
    child_code = 'import signal,time; signal.signal(signal.SIGTERM,signal.SIG_IGN); print("ready",flush=True); time.sleep(60)'
    worker_code = ('import os,subprocess,sys,time; os.setsid(); '
                   'p=subprocess.Popen([sys.executable,"-c",sys.argv[1]],stdout=subprocess.PIPE,text=True); '
                   'p.stdout.readline(); print(p.pid,flush=True); time.sleep(60)')
    process = QProcess(window)
    process.start(sys.executable, ['-c', worker_code, child_code])
    assert process.waitForStarted(3000)
    assert process.waitForReadyRead(3000)
    child = int(bytes(process.readAllStandardOutput()).strip())
    parent = process.processId()
    assert os.getpgid(child) == parent
    window.process, window.active_id = process, job['id']
    process.finished.connect(lambda code, status: window.finished(process, code))

    def child_finished():
        path = Path(f'/proc/{child}/stat')
        return not path.exists() or path.read_text().rsplit(')', 1)[1].split()[0] == 'Z'

    try:
        window.cancel(job['id'])
        wait_for(app, lambda: window.process is None, timeout=4)
        wait_for(app, child_finished, timeout=3)
        assert store.job(job['id'])['state'] == 'cancelled'
    finally:
        try:
            os.kill(child, signal.SIGKILL)
        except ProcessLookupError:
            pass
        window.close()
        app.processEvents()
        store.close()
