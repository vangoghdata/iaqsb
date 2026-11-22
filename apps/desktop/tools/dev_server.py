#!/usr/bin/env python3
"""Servidor de desarrollo para probar la interfaz de IAQSB en un navegador, sin Tauri.

Sirve la carpeta dist/ y responde a POST /api/call ejecutando la CLI:
    iaqsb --json llamar <op>   (los argumentos van por stdin)

Uso:
    cargo build -p iaqsb
    npm run build
    python tools/dev_server.py [ruta/a/iaqsb] [puerto]
"""
import http.server
import json
import os
import subprocess
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "dist")
CLI = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "target", "debug", "iaqsb")
PORT = int(sys.argv[2]) if len(sys.argv) > 2 else 1421


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=ROOT, **k)

    def do_POST(self):
        if self.path != "/api/call":
            self.send_error(404)
            return
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        # la CLI lee y escribe UTF-8; sin encoding, Windows usaría cp1252 y dañaría las tildes
        p = subprocess.run([CLI, "--json", "llamar", body["op"]], input=json.dumps(body.get("args", {})),
                           capture_output=True, text=True, encoding="utf-8")
        out = p.stdout.encode("utf-8") or json.dumps({"ok": False, "error": {"message": p.stderr}}).encode("utf-8")
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

    def log_message(self, *a):
        pass


if __name__ == "__main__":
    print(f"IAQSB (navegador) en http://127.0.0.1:{PORT}")
    http.server.ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
