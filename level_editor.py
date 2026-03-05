#!/usr/bin/env python3
"""Minimal local server for the PICO-8 level editor.
Serves level_editor.html and provides /level GET/POST endpoints
for reading/writing level_data.json."""

import os, sys, json, webbrowser
from http.server import HTTPServer, SimpleHTTPRequestHandler

DIR = os.path.dirname(os.path.abspath(__file__))
LEVEL_JSON = os.path.join(DIR, "level_data.json")
HTML = os.path.join(DIR, "level_editor.html")
PORT = 8080

class Handler(SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/" or self.path == "/index.html":
            self.send_response(200)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
            self.end_headers()
            with open(HTML, "rb") as f:
                self.wfile.write(f.read())
        elif self.path == "/level":
            if not os.path.exists(LEVEL_JSON):
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{}')
                return
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            with open(LEVEL_JSON, "rb") as f:
                self.wfile.write(f.read())
        else:
            self.send_error(404)

    def do_POST(self):
        if self.path == "/level":
            length = int(self.headers.get("Content-Length", 0))
            body = self.rfile.read(length)
            # Validate JSON
            try:
                json.loads(body)
            except json.JSONDecodeError:
                self.send_response(400)
                self.send_header("Content-Type", "text/plain")
                self.end_headers()
                self.wfile.write(b"Invalid JSON")
                return
            with open(LEVEL_JSON, "wb") as f:
                f.write(body)
            self.send_response(200)
            self.send_header("Content-Type", "text/plain")
            self.end_headers()
            self.wfile.write(b"ok")
            print(f"  Level saved ({len(body)} bytes)")
        else:
            self.send_error(404)

    def log_message(self, fmt, *args):
        print(f"  {args[0]}")

if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else PORT
    server = HTTPServer(("127.0.0.1", port), Handler)
    url = f"http://localhost:{port}"
    print(f"Level editor running at {url}")
    print(f"Level data: {LEVEL_JSON}")
    print("Press Ctrl+C to stop.\n")
    webbrowser.open(url)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nStopped.")
