import functools
import http.server
import os
import subprocess
import sys
import threading
import time

import pytest


@pytest.fixture
def media_server(tmp_path):
    media = tmp_path / 'source'
    media.mkdir()
    subprocess.run(['ffmpeg', '-v', 'error', '-f', 'lavfi', '-i', 'testsrc=size=320x180:rate=15',
                    '-f', 'lavfi', '-i', 'sine=frequency=440', '-t', '2', '-c:v', 'libx264',
                    '-pix_fmt', 'yuv420p', '-c:a', 'aac', str(media / 'sample.mp4')], check=True)
    (media / 'second.mp4').write_bytes((media / 'sample.mp4').read_bytes())
    (media / 'captions-en.vtt').write_text('WEBVTT\n\n00:00.000 --> 00:01.500\nTest subtitle\n')
    (media / 'playlist.html').write_text('''<html><head><title>Test playlist</title></head><body>
        <video src="sample.mp4"><track kind="subtitles" src="captions-en.vtt" srclang="en"></video>
        <video src="second.mp4"></video></body></html>''')

    class Handler(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *args):
            pass

        def do_HEAD(self):
            if self.path != '/slow.mp4':
                return super().do_HEAD()
            self.send_response(200)
            self.send_header('Content-Type', 'video/mp4')
            self.send_header('Content-Length', '10000000')
            self.end_headers()

        def do_GET(self):
            if self.path != '/slow.mp4':
                return super().do_GET()
            self.send_response(200)
            self.send_header('Content-Type', 'video/mp4')
            self.send_header('Content-Length', '10000000')
            self.end_headers()
            try:
                while True:
                    self.wfile.write(b'\x00' * 1024)
                    self.wfile.flush()
                    time.sleep(0.05)
            except (BrokenPipeError, ConnectionResetError):
                pass

    handler = functools.partial(Handler, directory=str(media))
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f'http://127.0.0.1:{server.server_port}/sample.mp4'
    server.shutdown()
    server.server_close()
    thread.join()


@pytest.fixture
def worker_command():
    binary = os.environ.get('VIDEODOCK_TEST_WORKER')
    return [binary] if binary else [sys.executable, '-m', 'videodock', '--worker']
