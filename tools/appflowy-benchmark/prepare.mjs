import { spawn, execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createWriteStream } from 'node:fs';
import { cp, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import http from 'node:http';
import path from 'node:path';
import { promisify } from 'node:util';

const execute = promisify(execFile);
const pins = JSON.parse(await readFile(new URL('./revisions.json', import.meta.url), 'utf8'));
const hash = content => createHash('sha256').update(content).digest('hex');
const readJson = file => readFile(file, 'utf8').then(JSON.parse).catch(() => null);

// Preparation rejects drift before timing: wrong/dirty source, a different
// package manager or SDK pin, partial builds, and stale generated harnesses.
async function output(command, args, cwd) {
  const result = await execute(command, args, { cwd, maxBuffer: 8 * 1024 * 1024 });
  return result.stdout.trim();
}

async function logged(command, args, cwd, logPath, env = process.env) {
  const log = createWriteStream(logPath, { mode: 0o600 });
  try {
    await new Promise((resolve, reject) => {
      const child = spawn(command, args, { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
      child.stdout.pipe(log, { end: false });
      child.stderr.pipe(log, { end: false });
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`${command} failed (${code}); inspect ${logPath}`)));
    });
  } finally { await new Promise(resolve => log.end(resolve)); }
}

export async function directoryHash(directory) {
  const digests = [];
  async function visit(relative) {
    const entries = await readdir(path.join(directory, relative), { withFileTypes: true });
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name, 'en'))) {
      const file = path.join(relative, entry.name);
      if (entry.isDirectory()) await visit(file);
      else if (entry.isFile()) digests.push([file, hash(await readFile(path.join(directory, file)))]);
      else throw new Error(`Unexpected non-file build artifact: ${file}`);
    }
  }
  await visit('');
  if (!digests.length) throw new Error(`Empty build directory: ${directory}`);
  return hash(JSON.stringify(digests));
}

async function verifySource(root, revision) {
  if (await output('git', ['rev-parse', 'HEAD'], root) !== revision) throw new Error(`Wrong cached revision: ${root}`);
  if (await output('git', ['status', '--porcelain', '--untracked-files=no'], root)) throw new Error(`Dirty cached source: ${root}`);
}

export async function prepareVariants({ cacheDir }) {
  await mkdir(cacheDir, { recursive: true });
  const variants = [];
  for (const pin of pins.variants) {
    if (!/^[a-f0-9]{40}$/.test(pin.revision) || !['legacy', 'native'].includes(pin.id)) throw new Error('Invalid source pin');
    const root = path.join(cacheDir, pin.id);
    await mkdir(root, { recursive: true });
    const existing = await output('git', ['rev-parse', '--show-toplevel'], root).catch(() => null);
    if (existing && path.resolve(existing) !== path.resolve(root)) throw new Error('Cache must be outside another Git working tree');
    if (!existing) {
      console.log(`Fetch ${pin.id} ${pin.revision}`);
      await output('git', ['init', '--quiet'], root);
      await output('git', ['remote', 'add', 'origin', pins.repository], root);
      await logged('git', ['fetch', '--depth=1', 'origin', pin.revision], root, path.join(cacheDir, `fetch-${pin.id}.log`));
      await output('git', ['checkout', '--quiet', '--detach', pin.revision], root);
    }
    await verifySource(root, pin.revision);
    const manifest = await readJson(path.join(root, 'package.json'));
    const packageManager = await output('pnpm', ['--version'], root);
    if (manifest.packageManager?.split('+')[0] !== `pnpm@${packageManager}`)
      throw new Error(`Use ${manifest.packageManager} for ${pin.id}; got pnpm@${packageManager}`);
    const lockHash = hash(await readFile(path.join(root, 'pnpm-lock.yaml')));
    const markerPath = path.join(cacheDir, `${pin.id}.prepared.json`);
    const toolchain = pin.id === 'native' ? {
      rustc: await output('rustc', ['--version'], root), wasmPack: await output('wasm-pack', ['--version'], root),
    } : {};
    const marker = { revision: pin.revision, lockHash, packageManager, node: process.version, sdkRevision: pin.sdkRevision ?? null, ...toolchain };
    const previous = await readJson(markerPath);
    let sdkHash;
    if (pin.id === 'native') {
      const sdkPin = await readJson(path.join(root, 'scripts/notion-formula-source.json'));
      if (sdkPin?.revision !== pin.sdkRevision) throw new Error('Native SDK pin differs from the reviewed benchmark manifest');
      sdkHash = await directoryHash(path.join(root, '.notion-formula-sdk')).catch(() => null);
      if (!sdkHash || previous?.sdkHash !== sdkHash || previous?.sdkRevision !== pin.sdkRevision ||
          previous?.rustc !== toolchain.rustc || previous?.wasmPack !== toolchain.wasmPack) {
        console.log('Build native pinned SDK');
        await logged('pnpm', ['formula:prepare'], root, path.join(cacheDir, 'prepare-sdk.log'));
        sdkHash = await directoryHash(path.join(root, '.notion-formula-sdk'));
      }
    }
    const expected = { ...marker, ...(sdkHash ? { sdkHash } : {}) };
    const installed = await readFile(path.join(root, 'node_modules/.modules.yaml')).then(() => true).catch(() => false);
    if (!installed || JSON.stringify(previous) !== JSON.stringify(expected)) {
      console.log(`Install frozen ${pin.id} dependencies`);
      await logged('pnpm', ['install', '--frozen-lockfile', '--prod=false'], root, path.join(cacheDir, `install-${pin.id}.log`));
      await writeFile(markerPath, JSON.stringify(expected, null, 2) + '\n');
    }
    await verifySource(root, pin.revision);
    variants.push({ ...pin, root, repository: pins.repository, lockHash, packageManager, ...toolchain, ...(sdkHash ? { sdkHash } : {}) });
  }
  return variants;
}

function buildScript(entry) {
  return `import { build, loadConfigFromFile } from 'vite';
import { readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
const root = process.cwd();
const loaded = await loadConfigFromFile({ command: 'build', mode: 'production' }, path.join(root, 'vite.config.ts'));
if (!loaded) throw new Error('Missing production Vite configuration');
const entry = ${JSON.stringify(entry)};
const dist = path.join(root, '.formula-benchmark/dist');
await build({ ...loaded.config, configFile: false, root, mode: 'production', logLevel: 'warn',
  build: { ...loaded.config.build, outDir: dist, emptyOutDir: true, manifest: true,
    rollupOptions: { ...loaded.config.build?.rollupOptions, input: entry } } });
const manifest = JSON.parse(await readFile(path.join(dist, '.vite/manifest.json'), 'utf8'));
const main = manifest[entry];
if (!main) throw new Error('Missing fixture entry in production manifest');
const css = new Set();
const seen = new Set();
function styles(key) { if (seen.has(key)) return; seen.add(key); const item = manifest[key]; for (const file of item.css ?? []) css.add(file); for (const imported of item.imports ?? []) styles(imported); }
styles(entry);
await writeFile(path.join(dist, 'index.html'), '<!doctype html><html><head><meta charset="utf-8"><title>Formula computed-chain benchmark</title>' + [...css].map(file => '<link rel="stylesheet" href="/' + file + '">').join('') + '</head><body><div id="root"></div><script type="module" src="/' + main.file + '"></script></body></html>');
`;
}

const mime = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml' };
async function serve(dist) {
  const sockets = new Set();
  const server = http.createServer(async (request, response) => {
    try {
      const url = new URL(request.url, 'http://localhost');
      const file = path.resolve(dist, '.' + decodeURIComponent(url.pathname === '/' ? '/index.html' : url.pathname));
      if (!file.startsWith(dist + path.sep)) { response.writeHead(403); response.end(); return; }
      const body = await readFile(file);
      response.writeHead(200, { 'content-type': mime[path.extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-store' });
      response.end(body);
    } catch { response.writeHead(404); response.end(); }
  });
  server.on('connection', socket => {
    sockets.add(socket);
    socket.once('close', () => sockets.delete(socket));
    // Closing a browser can reset an in-flight asset connection. The failed
    // request stays failed; it must not crash unrelated measurements.
    socket.on('error', () => socket.destroy());
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  return { url: `http://127.0.0.1:${server.address().port}/`,
    close: async () => { for (const socket of sockets) socket.destroy(); await new Promise(resolve => server.close(resolve)); } };
}

export async function buildFixture(variant, { browserDir }) {
  await verifySource(variant.root, variant.revision);
  const directory = path.join(variant.root, '.formula-benchmark');
  await mkdir(directory, { recursive: true });
  const entry = `.formula-benchmark/browser/${variant.id}.tsx`;
  const script = buildScript(entry);
  const marker = { revision: variant.revision, lockHash: variant.lockHash, sdkHash: variant.sdkHash ?? null,
    harnessHash: await directoryHash(browserDir), buildScriptHash: hash(script), node: process.version };
  const markerPath = path.join(directory, 'build.json');
  const previous = await readJson(markerPath);
  const dist = path.join(directory, 'dist');
  let artifactHash = await directoryHash(dist).catch(() => null);
  if (!artifactHash || previous?.artifactHash !== artifactHash || JSON.stringify(previous?.inputs) !== JSON.stringify(marker)) {
    await cp(browserDir, path.join(directory, 'browser'), { recursive: true });
    await writeFile(path.join(directory, 'build.mjs'), script);
    const env = { ...process.env, NODE_ENV: 'production', COVERAGE: 'false' };
    delete env.ANALYZE_MODE;
    await logged(process.execPath, [path.join(directory, 'build.mjs')], variant.root, path.join(directory, 'build.log'), env);
    artifactHash = await directoryHash(dist);
    await writeFile(markerPath, JSON.stringify({ inputs: marker, artifactHash }, null, 2) + '\n');
  }
  const host = await serve(dist);
  return { ...host, metadata: { ...marker, artifactHash, nodeEnv: 'production' } };
}
