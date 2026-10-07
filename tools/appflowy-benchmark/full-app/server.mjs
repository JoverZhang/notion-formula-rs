import http from 'node:http';
import https from 'node:https';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

const mimeTypes = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.png': 'image/png', '.ico': 'image/x-icon', '.woff2': 'font/woff2', '.woff': 'font/woff', '.ttf': 'font/ttf' };

function targetFor(url, backend, gotrue) {
  if (url.pathname.startsWith('/gotrue/')) {
    return new URL(gotrue.href.replace(/\/$/, '') + url.pathname.slice('/gotrue'.length) + url.search);
  }
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/ws/')) {
    return new URL(backend.href.replace(/\/$/, '') + url.pathname + url.search);
  }
  return null;
}

function transport(target) { return target.protocol === 'https:' ? https : http; }

// Serve both complete builds on the SAME loopback origin, switching only when
// all browser contexts of the preceding variant are closed. API/auth/WS calls
// reach the actual configured services, as a production reverse proxy does.
export async function serveFullApp({ backendUrl, gotrueUrl, port = 0 }) {
  const backend = new URL(backendUrl);
  const gotrue = new URL(gotrueUrl ?? `${backendUrl.replace(/\/$/, '')}/gotrue`);
  let dist;
  const sockets = new Set();
  const server = http.createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      const target = targetFor(url, backend, gotrue);
      if (target) {
        const upstream = transport(target).request(target, { method: request.method, headers: { ...request.headers, host: target.host } }, incoming => {
          response.writeHead(incoming.statusCode, incoming.headers);
          incoming.pipe(response);
        });
        upstream.on('error', () => { if (!response.headersSent) response.writeHead(502); response.end('Backend unavailable'); });
        request.pipe(upstream);
        return;
      }
      if (!dist) { response.writeHead(503); response.end(); return; }
      const relative = decodeURIComponent(url.pathname).replace(/^\/+/, '');
      const resolved = path.resolve(dist, relative);
      if (resolved !== dist && !resolved.startsWith(dist + path.sep)) { response.writeHead(403); response.end(); return; }
      let filePath = path.extname(relative) ? resolved : path.join(dist, 'index.html');
      let contents;
      try { contents = await readFile(filePath); }
      catch {
        if (path.extname(relative)) { response.writeHead(404); response.end(); return; }
        filePath = path.join(dist, 'index.html');
        contents = await readFile(filePath);
      }
      if (filePath.endsWith('index.html')) {
        const origin = `http://127.0.0.1:${server.address().port}`;
        const config = { APPFLOWY_BASE_URL: origin, APPFLOWY_GOTRUE_BASE_URL: `${origin}/gotrue`, APPFLOWY_WS_BASE_URL: origin.replace('http:', 'ws:') + '/ws/v2' };
        contents = Buffer.from(contents.toString().replace('<head>', `<head><script>window.__APP_CONFIG__=${JSON.stringify(config)};</script>`));
      }
      response.writeHead(200, { 'content-type': mimeTypes[path.extname(filePath)] ?? 'application/octet-stream', 'cache-control': filePath.endsWith('index.html') ? 'no-store' : 'public, max-age=31536000, immutable' });
      response.end(contents);
    } catch { response.writeHead(500); response.end('Full app server error'); }
  });
  server.on('connection', socket => { sockets.add(socket); socket.once('close', () => sockets.delete(socket)); });
  server.on('upgrade', (request, socket, head) => {
    const target = targetFor(new URL(request.url, 'http://localhost'), backend, gotrue);
    if (!target) { socket.destroy(); return; }
    const upstream = transport(target).request(target, { headers: { ...request.headers, host: target.host } });
    upstream.on('upgrade', (response, remote, remoteHead) => {
      socket.write(`HTTP/1.1 ${response.statusCode} ${response.statusMessage}\r\n` + response.rawHeaders.reduce((all, value, index) => all + (index % 2 ? `${value}\r\n` : `${value}: `), '') + '\r\n');
      if (remoteHead.length) socket.write(remoteHead);
      if (head.length) remote.write(head);
      remote.pipe(socket).pipe(remote);
      socket.once('close', () => remote.destroy());
      remote.once('error', () => socket.destroy());
    });
    upstream.on('response', () => socket.destroy());
    upstream.on('error', () => socket.destroy());
    upstream.end();
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); });
  return {
    origin: `http://127.0.0.1:${server.address().port}`,
    setVariant(directory) { dist = path.resolve(directory); },
    async close() { for (const socket of sockets) socket.destroy(); await new Promise(resolve => server.close(resolve)); },
  };
}
