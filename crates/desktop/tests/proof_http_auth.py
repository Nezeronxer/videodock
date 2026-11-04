#!/usr/bin/env python3
"""Opt-in live proof through a bounded, temporary authenticated CONNECT fixture.

The fixture dials every remote hostname through the supplied existing SOCKS5
exit; it never resolves remote names locally or creates a direct remote socket.
"""
import base64
import json
import os
from pathlib import Path
import select
import socket
import socketserver
import subprocess
import sys
import threading
import time
import signal


def exact(connection, size):
    chunks = bytearray()
    while len(chunks) < size:
        value = connection.recv(size - len(chunks))
        if not value:
            raise OSError("fixture upstream closed")
        chunks.extend(value)
    return bytes(chunks)


def main():
    executable = str(Path(sys.argv[1]).resolve())
    upstream_host, upstream_port = sys.argv[2].rsplit(":", 1)
    expected = b"Basic " + base64.b64encode(b"proof@fixture:synthetic:fixture")
    stopped = threading.Event()
    counts = {"challenge": 0, "authorized": 0, "interpreter": 0}

    def run_worker(request, timeout):
        process = subprocess.Popen([executable, "--youtube-proof"], stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=os.environ,
                                   start_new_session=os.name != "nt")
        try:
            output, error = process.communicate(json.dumps(request).encode(), timeout=timeout)
        except subprocess.TimeoutExpired:
            if os.name != "nt":
                os.killpg(process.pid, signal.SIGKILL)
            else:
                subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"],
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=False)
            process.wait()
            raise AssertionError("proof authentication retry did not terminate within its fixture budget") from None
        assert process.returncode == 0 and not error and b"synthetic" not in output
        return json.loads(output)

    class Handler(socketserver.BaseRequestHandler):
        def handle(self):
            self.request.settimeout(5)
            try:
                header = bytearray()
                while b"\r\n\r\n" not in header and len(header) <= 16 * 1024:
                    value = self.request.recv(4096)
                    if not value:
                        return
                    header.extend(value)
                if len(header) > 16 * 1024:
                    return
                lines = bytes(header).split(b"\r\n")
                method, authority, _ = lines[0].split(b" ", 2)
                host, port = authority.decode("ascii").rsplit(":", 1)
                if method != b"CONNECT" or port != "443" or not any(
                        host == domain or host.endswith("." + domain) for domain in
                        ["youtube.com", "google.com", "ytimg.com", "gstatic.com", "googleapis.com", "googleusercontent.com"]):
                    return
                authorization = next((line.split(b":", 1)[1].strip() for line in lines[1:]
                                      if line.lower().startswith(b"proxy-authorization:")), b"")
                if authorization != expected:
                    counts["challenge"] += 1
                    self.request.sendall(b'HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm="fixture"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n')
                    return
                counts["authorized"] += 1
                counts["interpreter"] += host == "www.google.com"
                with socket.create_connection((upstream_host, int(upstream_port)), timeout=5) as tunnel:
                    tunnel.sendall(b"\x05\x01\x00")
                    if exact(tunnel, 2) != b"\x05\x00":
                        raise OSError("fixture SOCKS method")
                    target = host.encode("ascii")
                    tunnel.sendall(b"\x05\x01\x00\x03" + bytes([len(target)]) + target + (443).to_bytes(2, "big"))
                    reply = exact(tunnel, 4)
                    if reply[:2] != b"\x05\x00":
                        raise OSError("fixture SOCKS connection")
                    length = {1: 4, 4: 16}.get(reply[3])
                    if reply[3] == 3:
                        length = exact(tunnel, 1)[0]
                    if length is None:
                        raise OSError("fixture SOCKS address")
                    exact(tunnel, length + 2)
                    self.request.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    total = 0
                    deadline = time.monotonic() + 40
                    while not stopped.is_set() and time.monotonic() < deadline:
                        ready, _, _ = select.select([self.request, tunnel], [], [], 0.2)
                        for source in ready:
                            data = source.recv(64 * 1024)
                            if not data:
                                return
                            total += len(data)
                            if total > 32 * 1024 * 1024:
                                return
                            (tunnel if source is self.request else self.request).sendall(data)
            except (OSError, ValueError):
                return

    class Server(socketserver.ThreadingTCPServer):
        allow_reuse_address = True
        def handle_error(self, request, client_address):
            pass

    with Server(("127.0.0.1", 0), Handler) as server:
        thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.1})
        thread.start()
        request = {"version": 1, "request_id": "12345678-1234-1234-1234-123456789abc",
                   "video_id": "aqz-KE-bpKQ", "dns_bootstrap": [],
                   "proxy": f"http://proof%40fixture:synthetic%3Afixture@127.0.0.1:{server.server_address[1]}"}
        try:
            response = run_worker(request, 50)
            assert response.get("ok") is True, response.get("stage", "worker failed")
            assert all(value > 0 for value in counts.values()), "missing proxy challenge or authenticated browser/interpreter traffic"
            print("authenticated HTTP proof: passed", counts)
            wrong = {**request, "proxy": request["proxy"].replace("synthetic%3Afixture", "wrong")}
            started = time.monotonic()
            response = run_worker(wrong, 15)
            assert response.get("ok") is False and response.get("stage") in {"network", "homepage"}
            print("wrong HTTP proxy password: bounded failure", round(time.monotonic() - started, 2))
        finally:
            stopped.set()
            server.shutdown()
            thread.join()


if __name__ == "__main__":
    main()
