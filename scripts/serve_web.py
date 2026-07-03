"""Static server for mobile/build/web with no-store headers so the
Cloudflare edge never pins a stale Flutter bundle."""
import http.server
import os
import sys


class NoStoreHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        super().end_headers()


if __name__ == '__main__':
    os.chdir(sys.argv[1])
    http.server.ThreadingHTTPServer(('', 8788), NoStoreHandler).serve_forever()
