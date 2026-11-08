#!/usr/bin/env python3
"""Plain-HTTP OTA host for the 0086 brick drill (`ota-dev` builds only).

Serves ROOT (an `overhead-release sign` output directory holding `ota/...`)
on 0.0.0.0:PORT with HTTP/1.0 and an exact Content-Length, like the real
hosts. `--cut-after N` closes the connection after N body bytes of any
`/ota/images/` file, to drill a download cut off midway. Every request is
logged with its outcome. Standard library only; never a release host.

    python3 firmware/ota_dev_server.py ROOT [--port 8765] [--cut-after N]
"""
import argparse
import http.server
import os
import sys
import time


def handler(root, cut_after):
    class Handler(http.server.BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.0"

        def log_message(self, fmt, *args):
            sys.stdout.write("%s %s %s\n" % (time.strftime("%H:%M:%S"), self.client_address[0], fmt % args))
            sys.stdout.flush()

        def do_GET(self):
            rel = os.path.normpath(self.path.lstrip("/"))
            path = os.path.join(root, rel)
            if rel.startswith("..") or not os.path.isfile(path):
                self.send_error(404)
                return
            with open(path, "rb") as f:
                body = f.read()
            self.send_response(200)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Content-Type", "application/octet-stream")
            self.end_headers()
            if cut_after is not None and rel.startswith("ota/images/"):
                self.wfile.write(body[:cut_after])
                self.wfile.flush()
                self.log_message("CUT %s after %d of %d bytes", rel, cut_after, len(body))
                self.close_connection = True
                return
            step = 16 * 1024
            for i in range(0, len(body), step):
                self.wfile.write(body[i : i + step])
            self.log_message("SENT %s %d bytes", rel, len(body))

    return Handler


def main():
    p = argparse.ArgumentParser()
    p.add_argument("root")
    p.add_argument("--port", type=int, default=8765)
    p.add_argument("--cut-after", type=int)
    a = p.parse_args()
    root = os.path.abspath(a.root)
    server = http.server.ThreadingHTTPServer(("0.0.0.0", a.port), handler(root, a.cut_after))
    print(f"serving {root} on 0.0.0.0:{a.port} cut_after={a.cut_after}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
