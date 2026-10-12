"""Serves the private demo review page (out/review/) to your phone on the
home network. Never for publishing.

    python3 tools/review_server.py start    # in the background; prints the URL
    python3 tools/review_server.py stop
    python3 tools/review_server.py status

It listens on this Mac's Wi-Fi/Ethernet address only (not on all
interfaces), answers only clients with private network addresses (home
network, this Mac), serves only out/review/, and supports byte ranges, which
iPhone Safari needs to play videos. Development only."""

import http.server
import ipaddress
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "out" / "review"
PID = SITE / ".server.pid"
LOG = SITE / ".server.log"
PORT = 8766


def lan_address():
    """This Mac's address on the home network (Wi-Fi first)."""
    for interface in ("en0", "en1", "en2"):
        out = subprocess.run(["ipconfig", "getifaddr", interface], capture_output=True, text=True)
        address = out.stdout.strip()
        if address and ipaddress.ip_address(address).is_private:
            return address
    sys.exit("No private network address found (is Wi-Fi on?)")


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map,
                      ".mp4": "video/mp4", ".vtt": "text/vtt", ".json": "application/json"}

    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(SITE), **kwargs)

    def log_message(self, fmt, *args):
        with open(LOG, "a") as log:
            log.write(f"{time.strftime('%H:%M:%S')} {self.client_address[0]} {fmt % args}\n")

    def allowed(self):
        address = ipaddress.ip_address(self.client_address[0])
        if not (address.is_private or address.is_loopback):
            self.send_error(403)
            return False
        # Hidden files (the pid file, the log) stay private.
        if any(part.startswith(".") for part in self.path.split("?")[0].split("/") if part):
            self.send_error(404)
            return False
        return True

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        self.send_header("Accept-Ranges", "bytes")
        super().end_headers()

    def do_HEAD(self):
        if self.allowed():
            super().do_HEAD()

    def do_GET(self):
        if not self.allowed():
            return
        path = Path(self.translate_path(self.path))
        match = re.fullmatch(r"bytes=(\d*)-(\d*)", self.headers.get("Range", ""))
        if not (match and path.is_file()):
            return super().do_GET()
        size = path.stat().st_size
        start = int(match[1]) if match[1] else max(0, size - int(match[2] or 0))
        end = int(match[2]) if match[1] and match[2] else size - 1
        end = min(end, size - 1)
        if start > end:
            self.send_response(416)
            self.send_header("Content-Range", f"bytes */{size}")
            self.end_headers()
            return
        self.send_response(206)
        self.send_header("Content-Type", self.guess_type(str(path)))
        self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.send_header("Content-Length", str(end - start + 1))
        self.end_headers()
        with open(path, "rb") as f:
            f.seek(start)
            remaining = end - start + 1
            while remaining:
                chunk = f.read(min(1 << 20, remaining))
                if not chunk:
                    break
                self.wfile.write(chunk)
                remaining -= len(chunk)

    def do_POST(self):
        self.send_error(405)

    do_PUT = do_DELETE = do_POST


def running():
    try:
        pid = int(PID.read_text())
        os.kill(pid, 0)
        return pid
    except (FileNotFoundError, ValueError, ProcessLookupError, PermissionError):
        return None


def main():
    command = sys.argv[1] if len(sys.argv) > 1 else "status"
    SITE.mkdir(parents=True, exist_ok=True)
    if command == "serve":
        address = sys.argv[2]
        http.server.ThreadingHTTPServer((address, PORT), Handler).serve_forever()
    elif command == "start":
        if running():
            print(f"Already running: http://{lan_address()}:{PORT}/")
            return
        address = lan_address()
        proc = subprocess.Popen([sys.executable, __file__, "serve", address], start_new_session=True,
                                stdout=subprocess.DEVNULL, stderr=open(LOG, "a"))
        PID.write_text(str(proc.pid))
        time.sleep(0.5)
        if proc.poll() is not None:
            sys.exit(f"The server did not start; see {LOG}")
        print(f"Review page: http://{address}:{PORT}/")
    elif command == "stop":
        pid = running()
        if pid:
            os.kill(pid, signal.SIGTERM)
            print("Stopped.")
        else:
            print("Not running.")
        PID.unlink(missing_ok=True)
    else:
        pid = running()
        print(f"Running (pid {pid}): http://{lan_address()}:{PORT}/" if pid else "Not running.")


if __name__ == "__main__":
    main()
