"""Capture a native backtrace using a synthetic request and a closed localhost proxy."""
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1])
output = Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)
if not binary.is_file():
    raise SystemExit(0)
with tempfile.TemporaryDirectory(prefix='videodock-worker-debug-') as directory:
    request = Path(directory) / 'request.json'
    request.write_text(json.dumps({'version': 1, 'request_id': '12345678-1234-1234-1234-123456789abc',
                                   'video_id': 'aqz-KE-bpKQ', 'proxy': 'http://127.0.0.1:9'}))
    with (output / 'worker-backtrace.txt').open('w') as log:
        child = subprocess.Popen(['lldb', '--batch', '-o', f'settings set target.input-path "{request}"',
                                  '-o', 'run', '-o', 'thread backtrace all', '--', str(binary), '--youtube-proof'],
                                 env={**os.environ, 'QT_QPA_PLATFORM': 'cocoa'},
                                 stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        try:
            child.wait(timeout=60)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
            log.write('\nDebugger deadline exceeded.\n')
