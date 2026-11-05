#!/usr/bin/env python3
"""Offline black-box checks: invalid proof requests must never start the UI/profile."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import signal
import socket
import threading


def main():
    executable = str(Path(sys.argv[1]).resolve())
    request = {"version": 1, "request_id": "12345678-1234-1234-1234-123456789abc",
               "video_id": "aqz-KE-bpKQ", "proxy": None}
    invalid = [b"{}\n", b"x" * (32 * 1024 + 1),
               json.dumps({**request, "video_id": "../bad"}).encode(),
               json.dumps({**request, "request_id": "bad\nvalue"}).encode(),
               json.dumps({**request, "proxy": "socks5h://user:private@localhost:8080"}).encode(),
               json.dumps({**request, "proxy": "https://localhost:8080"}).encode(),
               json.dumps({**request, "proxy": "http://localhost:8080/path"}).encode(),
               json.dumps({**request, "unexpected": True}).encode()]
    with tempfile.TemporaryDirectory(prefix="videodock-proof-contract-") as directory:
        profile = Path(directory) / "must-not-exist"
        environment = {**os.environ, "QT_QPA_PLATFORM": "offscreen", "VIDEODOCK_DATA_DIR": str(profile)}
        for data in invalid:
            result = subprocess.run([executable, "--youtube-proof"], input=data,
                                    capture_output=True, env=environment, timeout=5)
            reply = json.loads(result.stdout)
            assert reply["version"] == 1 and reply["ok"] is False and reply["stage"] == "validation"
            assert len(result.stdout) <= 32 * 1024
            assert not profile.exists(), "worker opened the normal application profile"
            assert b"private" not in result.stdout + result.stderr
    print(f"proof worker offline contract: {len(invalid)} passed")

    if "--proxy" in sys.argv[2:]:
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            listener.listen()
            listener.settimeout(0.1)
            stopped = threading.Event()
            requests = []

            def reject_proxy():
                while not stopped.is_set():
                    try:
                        connection, _ = listener.accept()
                    except socket.timeout:
                        continue
                    with connection:
                        connection.settimeout(1)
                        try:
                            data = connection.recv(8192)
                            requests.append(data.startswith(b"CONNECT www.youtube.com:443 "))
                            connection.sendall(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                        except OSError:
                            pass

            thread = threading.Thread(target=reject_proxy)
            thread.start()
            environment = {**os.environ, "QT_QPA_PLATFORM": "offscreen"}
            routed = {**request, "proxy": f"http://127.0.0.1:{listener.getsockname()[1]}"}
            try:
                result = subprocess.run([executable, "--youtube-proof"], input=json.dumps(routed).encode(),
                                        capture_output=True, env=environment, timeout=15)
                assert result.returncode == 0 and result.stdout, (
                    f"Packaged proof worker exited without a response: code={result.returncode}, "
                    f"stdout_bytes={len(result.stdout)}, stderr_bytes={len(result.stderr)}")
                reply = json.loads(result.stdout)
                assert requests and any(requests), "worker did not use the selected HTTP proxy"
                assert reply["ok"] is False and reply["stage"] == "homepage"
            finally:
                stopped.set()
                thread.join()
        print("proof worker selected-proxy refusal: passed")

    if "--startup" in sys.argv[2:]:
        with tempfile.TemporaryDirectory(prefix="videodock-proxy-startup-") as directory:
            environment = {**os.environ, "QT_QPA_PLATFORM": "offscreen", "QT_QUICK_BACKEND": "software",
                           "VIDEODOCK_DATA_DIR": str(Path(directory) / "profile"),
                           "https_proxy": "unsupported://invalid", "HTTPS_PROXY": "unsupported://invalid"}
            process = subprocess.Popen([executable], env=environment, stdout=subprocess.DEVNULL,
                                       stderr=subprocess.DEVNULL, start_new_session=os.name != "nt")
            try:
                time.sleep(3)
                assert process.poll() is None, "bad automatic proxy prevented the settings UI from opening"
            finally:
                if os.name != "nt":
                    try:
                        os.killpg(process.pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                else:
                    subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
                process.wait()
        print("bad automatic proxy: application remains available for settings")


if __name__ == "__main__":
    main()
