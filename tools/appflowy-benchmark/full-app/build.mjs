import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import { createWriteStream } from 'node:fs';
import { chmod, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

import { directoryHash } from '../prepare.mjs';

// Both pinned applications receive exactly this instrumentation. It preserves
// selectors and exposes real dispatch hooks; it changes no formula code.
const bridge = `
import { useDatabaseContext as useFormulaBenchmarkContext } from '@/application/database-yjs/context';
import { useNewRowDispatch as useFormulaBenchmarkNewRow } from '@/application/database-yjs/dispatch/row';
import { useUpdateAnyCellDispatch as useFormulaBenchmarkUpdateCell } from '@/application/database-yjs/dispatch/cell';
import { YjsDatabaseKey as FormulaBenchmarkDatabaseKey, YjsEditorKey as FormulaBenchmarkEditorKey } from '@/application/types';
import { waitForDrain as waitForFormulaBenchmarkDrain } from '@/application/sync-outbox';
function FormulaBenchmarkBridge() {
  const context = useFormulaBenchmarkContext();
  const newRow = useFormulaBenchmarkNewRow();
  const updateCell = useFormulaBenchmarkUpdateCell();
  useEffect(() => {
    if (typeof window === 'undefined' || !window.__FORMULA_BENCHMARK_ACTIVE__ || context.isDatabaseRowPage || context.closeRowDetailModal) return;
    const value = { context, newRow, updateCell, waitForDrain: waitForFormulaBenchmarkDrain, databaseKey: FormulaBenchmarkDatabaseKey, editorKey: FormulaBenchmarkEditorKey };
    window.__FORMULA_BENCHMARK__ = value;
    return () => { if (window.__FORMULA_BENCHMARK__ === value) delete window.__FORMULA_BENCHMARK__; };
  }, [context, newRow, updateCell]);
  return null;
}
`;

const script = `
import { build, loadConfigFromFile } from 'vite';
import path from 'node:path';
const root = process.cwd();
const result = await loadConfigFromFile({ command: 'build', mode: 'production' }, path.join(root, 'vite.config.ts'));
if (!result) throw new Error('Pinned AppFlowy Vite configuration was not found');
const bridge = ${JSON.stringify(bridge)};
let instrumented = 0;
const instrumentation = {
  name: 'formula-full-app-benchmark-context',
  enforce: 'pre',
  transform(code, id) {
    if (!id.endsWith('/src/components/database/DatabaseContext.tsx')) return null;
    const before = '<DatabaseContext.Provider value={value}>{children}</DatabaseContext.Provider>';
    if (!code.includes(before)) throw new Error('Pinned DatabaseContext provider changed; benchmark instrumentation requires review');
    instrumented++;
    return { code: bridge + code.replace(before, '<DatabaseContext.Provider value={value}><FormulaBenchmarkBridge />{children}</DatabaseContext.Provider>'), map: null };
  },
};
const plugins = result.config.plugins.flat(Infinity).filter(Boolean).filter(plugin => plugin.name !== 'strip-test-id');
await build({ ...result.config, configFile: false, root, mode: 'production', plugins: [instrumentation, ...plugins], build: { ...result.config.build, outDir: '.formula-benchmark-full-app/dist', emptyOutDir: true } });
if (instrumented !== 1) throw new Error('Full app benchmark context was not instrumented exactly once');
`;

export const instrumentationHash = createHash('sha256').update(script).digest('hex');

export async function buildFullApp(variant, outputDir, { rebuild = false } = {}) {
  const generated = path.join(variant.root, '.formula-benchmark-full-app');
  const dist = path.join(generated, 'dist');
  const markerPath = path.join(generated, 'build.json');
  const lockHash = variant.lockHash ?? createHash('sha256').update(await readFile(path.join(variant.root, 'pnpm-lock.yaml'))).digest('hex');
  const sdkHash = variant.sdkHash ?? (variant.id === 'native' ? await directoryHash(path.join(variant.root, '.notion-formula-sdk')) : null);
  const marker = { revision: variant.revision, instrumentationHash, nodeEnv: 'production', node: process.version, lockHash, sdkHash };
  if (!rebuild) {
    const previous = await readFile(markerPath, 'utf8').then(JSON.parse).catch(() => null);
    const indexExists = await readFile(path.join(dist, 'index.html')).then(() => true).catch(() => false);
    const inputsMatch = previous && Object.keys(marker).every(key => previous[key] === marker[key]);
    if (indexExists && inputsMatch && previous.artifactHash === await directoryHash(dist)) return { dist, cached: true, metadata: previous };
  }
  await mkdir(generated, { recursive: true });
  const scriptPath = path.join(generated, 'build.mjs');
  await writeFile(scriptPath, script, { mode: 0o600 });
  const logPath = path.join(outputDir, `full-app-build-${variant.id}.log`);
  const log = createWriteStream(logPath, { mode: 0o600 });
  const env = { ...process.env, NODE_ENV: 'production', COVERAGE: 'false' };
  delete env.ANALYZE_MODE;
  await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [scriptPath], { cwd: variant.root, env, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.pipe(log, { end: false });
    child.stderr.pipe(log, { end: false });
    child.once('error', reject);
    child.once('close', code => {
      log.end();
      if (code === 0) resolve();
      else reject(new Error(`Production build failed for ${variant.id}; inspect its private build log`));
    });
  });
  await chmod(logPath, 0o600);
  const metadata = { ...marker, artifactHash: await directoryHash(dist) };
  await writeFile(markerPath, JSON.stringify(metadata), { mode: 0o600 });
  return { dist, cached: false, logPath, metadata };
}
