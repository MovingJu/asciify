import { join } from "path";

const ROOT = import.meta.dir;
const PORT = 8080;

const MIME: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript",
  ".css": "text/css",
  ".wasm": "application/wasm",
  ".json": "application/json",
};

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    const path = url.pathname === "/" ? "/index.html" : url.pathname;
    const fsPath = join(ROOT, path);

    const file = Bun.file(fsPath);
    if (await file.exists()) {
      const ext = "." + fsPath.split(".").pop();
      return new Response(file, { headers: { "Content-Type": MIME[ext] ?? "application/octet-stream" } });
    }

    return new Response("Not Found", { status: 404 });
  },
});

console.log(`🖼  asciify web on :${PORT}`);
