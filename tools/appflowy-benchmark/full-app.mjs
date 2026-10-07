import { createHash } from 'node:crypto';
import { chmod, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';

import { deadline } from './deadline.mjs';
import { buildFullApp, buildInstrumentationHash, instrumentationHash, timingScriptHash } from './full-app/build.mjs';
import { serveFullApp } from './full-app/server.mjs';
import { installBrowserTiming, timingResult } from './full-app/timing.mjs';

const VIEWPORT = { width: 1600, height: 1050 };
const DATASET_OWNER = 'notion-formula-full-app-benchmark-v1';
const DATASET_TITLE = 'Formula A/B benchmark ';
const positiveInteger = (value, fallback, label) => {
  const number = Number(value ?? fallback);
  if (!Number.isInteger(number) || number < 1) throw new Error(`${label} must be a positive integer`);
  return number;
};
const nonnegativeInteger = (value, fallback, label) => {
  const number = Number(value ?? fallback);
  if (!Number.isInteger(number) || number < 0) throw new Error(`${label} must be a nonnegative integer`);
  return number;
};
const digest = value => createHash('sha256').update(JSON.stringify(value)).digest('hex');

function generateRows(count) {
  return Array.from({ length: count }, (_, index) => {
    const price = 10 + index % 91;
    const quantity = 2 + index % 9;
    return { name: `Benchmark row ${String(index + 1).padStart(5, '0')}`, price, quantity, subtotal: price * quantity, total: price * quantity + 5 };
  });
}

async function privateJson(filePath, value) {
  await writeFile(filePath, JSON.stringify(value, null, 2) + '\n', { mode: 0o600 });
  await chmod(filePath, 0o600);
}

async function readPrivateJson(filePath, label, missingValue) {
  let contents;
  try { contents = await readFile(filePath, 'utf8'); }
  catch (error) {
    if (error.code === 'ENOENT' && arguments.length >= 3) return missingValue;
    throw new Error(`Could not read private ${label} JSON`);
  }
  // JSON.parse errors can quote malformed input, including secrets. Never
  // propagate either that input or the user's private path into report.failure.
  try { return JSON.parse(contents); }
  catch { throw new Error(`Private ${label} JSON is malformed`); }
}

async function until(page, predicate, argument, timeoutMs) {
  await page.waitForFunction(predicate, argument, { timeout: timeoutMs });
}

async function gridReady(page, timeoutMs) {
  await page.locator('.grid-row-cell').first().waitFor({ state: 'visible', timeout: timeoutMs });
  await until(page, () => Boolean(window.__FORMULA_BENCHMARK__?.context?.databaseDoc), null, timeoutMs);
}

function accountFingerprint(credentials, state) {
  let account = credentials?.email;
  if (!account) {
    const token = state?.origins?.flatMap(origin => origin.localStorage ?? []).find(entry => entry.name === 'token');
    try {
      const value = JSON.parse(token?.value ?? '{}');
      account = value.user?.email ?? value.user?.id;
    } catch { /* Authentication will report an unusable state separately. */ }
  }
  if (!account) throw new Error('Cannot identify the authenticated account for private dataset reuse');
  return String(account).toLowerCase();
}

function remapAuthState(state, origin) {
  const token = state.origins?.flatMap(entry => entry.localStorage ?? []).find(entry => entry.name === 'token');
  if (!token) throw new Error('Private authentication state has no AppFlowy token');
  // Cold contexts carry authentication only: no provider cache, device ID,
  // theme/language preferences, IndexedDB or cookies from the setup browser.
  return { cookies: [], origins: [{ origin, localStorage: [{ name: 'token', value: token.value }] }] };
}

async function createContext(browser, origin, state, timeoutMs, navigation = null) {
  const context = await browser.newContext({ viewport: VIEWPORT, timezoneId: 'UTC', locale: 'en-US', colorScheme: 'light', ...(state ? { storageState: remapAuthState(state, origin) } : {}) });
  await context.addInitScript(installBrowserTiming, navigation);
  const errors = [];
  const assets = { wasmResponses: 0, failedWasmResponses: 0 };
  context.on('page', page => page.on('pageerror', () => errors.push('Uncaught application error')));
  context.on('response', response => {
    if (/\.wasm(?:\?|$)/.test(response.url())) {
      assets.wasmResponses++;
      if (response.status() >= 400) assets.failedWasmResponses++;
    }
  });
  const page = await context.newPage();
  page.setDefaultTimeout(timeoutMs);
  page.setDefaultNavigationTimeout(timeoutMs);
  return { context, page, errors, assets };
}

async function openAuthenticated(browser, origin, state, credentials, timeoutMs) {
  const active = await createContext(browser, origin, state, timeoutMs);
  const { context, page } = active;
  await page.goto(`${origin}/app`, { waitUntil: 'domcontentloaded' });
  const shell = page.getByTestId('sidebar-page-header');
  const ready = await Promise.race([
    shell.waitFor({ state: 'visible', timeout: timeoutMs }).then(() => true),
    page.getByTestId('login-email-input').waitFor({ state: 'visible', timeout: timeoutMs }).then(() => false),
  ]);
  if (!ready) {
    if (!credentials?.email || !credentials?.password) { await context.close(); throw new Error('Authentication state is unavailable or expired; supply a private credentials JSON file'); }
    await page.getByTestId('login-email-input').fill(credentials.email);
    await page.getByTestId('login-password-button').click();
    await page.getByTestId('password-input').fill(credentials.password);
    await page.getByTestId('password-submit-button').click();
    await shell.waitFor({ state: 'visible', timeout: timeoutMs });
  }
  return active;
}

async function source(page, expression, timeoutMs = 60_000) {
  const editor = page.getByTestId('formula-editor-input');
  await editor.waitFor({ state: 'visible' });
  await until(page, () => document.querySelector('[data-testid="formula-editor-input"]')?.getAttribute('aria-readonly') !== 'true', null, timeoutMs);
  await editor.click();
  await editor.press('ControlOrMeta+a');
  // One insert event replaces the current selection: no intermediate Backspace
  // event can start the timer against an unrelated empty expression.
  await page.keyboard.insertText(expression);
}

async function previewReady(page, expression, expected, timeoutMs) {
  await until(page, ({ expression, expected }) => {
    const editor = document.querySelector('[data-testid="formula-editor-input"]');
    const done = document.querySelector('[data-testid="formula-editor-done"]');
    const bridge = window.__FORMULA_BENCHMARK__;
    const fields = bridge.context.databaseDoc.getMap(bridge.editorKey.data_section).get(bridge.editorKey.database).get(bridge.databaseKey.fields);
    // The legacy editor displays property names; the native editor retains
    // canonical IDs. Verify that either representation binds to the SAME IDs.
    const canonical = editor?.getAttribute('data-value')?.replace(/prop\("([^"\\]*)"\)/g, (match, reference) => {
      const field = [...fields.values()].find(field => field.get(bridge.databaseKey.id) === reference || field.get(bridge.databaseKey.name) === reference);
      return field ? `prop("${field.get(bridge.databaseKey.id)}")` : match;
    });
    return canonical === expression && document.querySelector('[data-testid="formula-preview-value"]')?.textContent?.trim() === expected && done && !done.disabled && !document.querySelector('[data-testid="formula-editor-error"]');
  }, { expression, expected: String(expected) }, timeoutMs);
}

async function addField(page, name, type, expression, expected, timeoutMs) {
  const add = page.getByTestId('grid-new-property-button').last();
  await add.scrollIntoViewIfNeeded();
  await add.evaluate(element => element.click());
  await page.getByTestId('property-name-input').last().fill(name);
  await page.getByTestId('property-type-trigger').last().hover();
  const option = page.getByTestId(`property-type-option-${type}`).last();
  await option.waitFor({ state: 'attached' });
  await option.evaluate(element => element.click());
  if (expression) {
    await page.getByTestId('formula-editor-dialog').waitFor({ state: 'visible' });
    await source(page, expression, timeoutMs);
    await previewReady(page, expression, expected, timeoutMs);
    await page.getByTestId('formula-editor-done').click();
    await page.getByTestId('formula-editor-dialog').waitFor({ state: 'hidden' });
  } else {
    await page.keyboard.press('Escape');
    await page.keyboard.press('Escape');
  }
  return page.evaluate(name => {
    const bridge = window.__FORMULA_BENCHMARK__;
    const root = bridge.context.databaseDoc.getMap(bridge.editorKey.data_section);
    const fields = root.get(bridge.editorKey.database).get(bridge.databaseKey.fields);
    const field = [...fields.values()].find(field => field.get(bridge.databaseKey.name) === name);
    if (!field) throw new Error('New benchmark field was not found');
    return field.get(bridge.databaseKey.id);
  }, name);
}

async function datasetIdentity(page) {
  return page.evaluate(() => {
    const bridge = window.__FORMULA_BENCHMARK__;
    const root = bridge.context.databaseDoc.getMap(bridge.editorKey.data_section);
    const database = root.get(bridge.editorKey.database);
    const fields = database.get(bridge.databaseKey.fields);
    const primary = [...fields.values()].find(field => field.get(bridge.databaseKey.is_primary));
    const rows = database.get(bridge.databaseKey.views).get(bridge.context.activeViewId).get(bridge.databaseKey.row_orders).toArray().map(row => row.id);
    return { rowIds: rows, primary: primary.get(bridge.databaseKey.id) };
  });
}

async function seedDataset(page, rows, origin, timeoutMs) {
  // Create through the real application outline. Never open or alter a supplied
  // user database, including an existing Formula Playground.
  const space = page.getByTestId('space-item').filter({ has: page.getByTestId('space-name') }).first();
  await space.getByTestId('space-name').first().hover();
  await space.getByTestId('inline-add-page').click();
  await page.getByTestId('add-grid-button').click();
  await gridReady(page, timeoutMs);
  const title = `${DATASET_TITLE}${rows.length} rows ${new Date().toISOString()}`;
  const titleInput = page.getByTestId('page-title-input');
  await titleInput.fill(title);
  await titleInput.press('Enter');
  const identity = await datasetIdentity(page);
  if (identity.rowIds.length > rows.length) throw new Error('Fresh Grid contains more rows than the requested benchmark');
  const fields = { name: identity.primary };
  fields.price = await addField(page, 'Price', 1, null, null, timeoutMs);
  fields.quantity = await addField(page, 'Quantity', 1, null, null, timeoutMs);
  const rowIds = [...identity.rowIds];
  for (let index = 0; index < rows.length; index++) {
    const row = rows[index];
    const cells = { [fields.name]: row.name, [fields.price]: String(row.price), [fields.quantity]: String(row.quantity) };
    if (index < identity.rowIds.length) {
      await page.evaluate(({ rowId, cells }) => {
        for (const [fieldId, data] of Object.entries(cells)) window.__FORMULA_BENCHMARK__.updateCell(rowId, fieldId, data);
      }, { rowId: rowIds[index], cells });
    } else {
      const rowId = await deadline(page.evaluate(cells => window.__FORMULA_BENCHMARK__.newRow({ tailing: true, cellsData: cells, skipDefaultTemplate: true }), cells), timeoutMs, 'Real row dispatch');
      if (!rowId) throw new Error('Real row dispatch failed to create a benchmark row');
      rowIds.push(rowId);
    }
  }
  const subtotalExpression = `prop("${fields.price}") * prop("${fields.quantity}")`;
  fields.subtotal = await addField(page, 'Subtotal', 19, subtotalExpression, rows[0].subtotal, timeoutMs);
  const totalExpression = `prop("${fields.subtotal}") + 5`;
  fields.total = await addField(page, 'Total', 19, totalExpression, rows[0].total, timeoutMs);
  const drained = await page.evaluate(timeoutMs => window.__FORMULA_BENCHMARK__.waitForDrain(undefined, { timeoutMs }), timeoutMs);
  if (!drained) throw new Error('Benchmark setup mutations did not drain to the real Cloud backend');
  const url = new URL(page.url());
  if (url.origin !== origin) throw new Error('Benchmark database was created outside the served application');
  return { owner: DATASET_OWNER, title, route: url.pathname + url.search, fields, rowIds, subtotalExpression, totalExpression, rows: rows.length, checksum: digest(rows) };
}

async function verifyDataset(page, dataset, rows, timeoutMs) {
  await gridReady(page, timeoutMs);
  await verifyOwnership(page, dataset);
  await checkRows(page, dataset, rows, timeoutMs, false);
}

async function checkRows(page, dataset, rows, timeoutMs, restore) {
  await deadline(page.evaluate(async ({ fields, rowIds, rows, timeoutMs, restore }) => {
    const started = performance.now();
    for (let index = 0; index < rowIds.length; index++) {
      let doc = await window.__FORMULA_BENCHMARK__.context.ensureRow(rowIds[index]);
      for (;;) {
        const bridge = window.__FORMULA_BENCHMARK__;
        doc = bridge.context.rowMap?.[rowIds[index]] ?? doc;
        const row = doc?.getMap(bridge.editorKey.data_section).get(bridge.editorKey.database_row);
        const cells = row?.get(bridge.databaseKey.cells);
        const inputs = Object.entries(fields).filter(([key]) => key !== 'subtotal' && key !== 'total');
        const loaded = inputs.every(([, fieldId]) => cells?.has(fieldId));
        const mismatch = inputs.find(([key, fieldId]) => String(cells?.get(fieldId)?.get(bridge.databaseKey.data) ?? '') !== String(rows[index][key]));
        if (loaded && restore && mismatch) {
          for (const [key, fieldId] of inputs) {
            if (String(cells.get(fieldId)?.get(bridge.databaseKey.data) ?? '') !== String(rows[index][key])) bridge.updateCell(rowIds[index], fieldId, String(rows[index][key]));
          }
        }
        if (loaded && !mismatch) break;
        if (performance.now() - started >= timeoutMs) throw new Error(`Fresh Cloud input verification timed out at row index ${index}, input ${mismatch?.[0] ?? 'unloaded'}`);
        await new Promise(resolve => setTimeout(resolve, 25));
        if (!loaded) doc = await window.__FORMULA_BENCHMARK__.context.ensureRow(rowIds[index]);
      }
    }
  }, { fields: dataset.fields, rowIds: dataset.rowIds, rows, timeoutMs, restore }), timeoutMs + 2_000, 'Fresh Cloud input verification');
}

async function verifyOwnership(page, dataset) {
  if (dataset.owner !== DATASET_OWNER || !dataset.title?.startsWith(DATASET_TITLE)) throw new Error('Refusing to mutate a database without benchmark ownership metadata');
  const title = (await page.getByTestId('page-title-input').textContent())?.trim();
  if (title !== dataset.title) throw new Error('Refusing to mutate a database whose title differs from the dedicated benchmark manifest');
  const identity = await datasetIdentity(page);
  if (JSON.stringify(identity.rowIds) !== JSON.stringify(dataset.rowIds)) throw new Error('Shared benchmark row identity/order differs from its manifest');
  const formulaDefinitions = await page.evaluate(({ fields }) => {
    const bridge = window.__FORMULA_BENCHMARK__;
    const database = bridge.context.databaseDoc.getMap(bridge.editorKey.data_section).get(bridge.editorKey.database);
    const storedFields = database.get(bridge.databaseKey.fields);
    return ['subtotal', 'total'].map(key => storedFields.get(fields[key])?.get(bridge.databaseKey.type_option)?.get('19')?.get(bridge.databaseKey.expression));
  }, { fields: dataset.fields });
  if (JSON.stringify(formulaDefinitions) !== JSON.stringify([dataset.subtotalExpression, dataset.totalExpression])) throw new Error('Dedicated benchmark formula definitions changed; refusing to overwrite saved formulas');
}

async function restoreDataset(page, dataset, rows, timeoutMs) {
  await gridReady(page, timeoutMs);
  await verifyOwnership(page, dataset);
  await checkRows(page, dataset, rows, timeoutMs, true);
  const drained = await page.evaluate(timeoutMs => window.__FORMULA_BENCHMARK__.waitForDrain(undefined, { timeoutMs }), timeoutMs);
  if (!drained) throw new Error('Restored benchmark inputs did not drain to Cloud');
  await verifyDataset(page, dataset, rows, timeoutMs);
}

function navigationConfig(dataset, rows, timeoutMs) {
  return {
    kind: 'grid', timeoutMs, minimumVisibleRows: Math.min(10, rows.length), formulaFields: 2,
    expected: Object.fromEntries(dataset.rowIds.map((id, index) => [id, { [dataset.fields.subtotal]: String(rows[index].subtotal), [dataset.fields.total]: String(rows[index].total) }])),
  };
}

function cellSelector(dataset, field) {
  return `[data-testid="grid-row-${dataset.rowIds[0]}"] .grid-row-cell[data-column-id="${dataset.fields[field]}"]`;
}

async function cellInput(page, dataset, value) {
  await page.keyboard.press('Escape');
  const cell = page.locator(cellSelector(dataset, 'price'));
  await cell.scrollIntoViewIfNeeded();
  await cell.evaluate(element => element.click());
  const input = page.locator('textarea:visible').first();
  await input.waitFor({ state: 'visible' });
  await input.fill(String(value));
  return input;
}

async function iteration(page, dataset, rows, origin, timeoutMs, variant, record) {
  await page.goto(origin + dataset.route, { waitUntil: 'domcontentloaded' });
  const opened = await timingResult(page, timeoutMs);
  await record('dependency-chain', 'open-database-hot-visible-formulas', opened);

  const first = rows[0];
  const changedPrice = first.price + 7;
  const changedTotal = changedPrice * first.quantity + 5;
  let input = await cellInput(page, dataset, changedPrice);
  await page.evaluate(config => window.__FORMULA_BENCHMARK_TIMING__.arm(config), {
    kind: 'cell', timeoutMs, inputSelector: cellSelector(dataset, 'price'), input: String(changedPrice), resultSelector: cellSelector(dataset, 'total'), result: String(changedTotal),
  });
  await input.press('Enter');
  await record('dependency-chain', 'submit-input-dependent-result', await timingResult(page, timeoutMs));
  input = await cellInput(page, dataset, first.price);
  await input.press('Enter');
  await until(page, ({ selector, expected }) => document.querySelector(selector)?.textContent?.trim() === expected, { selector: cellSelector(dataset, 'total'), expected: String(first.total) }, timeoutMs);

  await page.locator(cellSelector(dataset, 'total')).evaluate(element => element.click());
  await page.getByTestId('formula-editor-dialog').waitFor({ state: 'visible' });
  await previewReady(page, dataset.totalExpression, first.total, timeoutMs);
  const edited = `prop("${dataset.fields.subtotal}") + 17`;
  await page.getByTestId('formula-editor-input').click();
  await page.getByTestId('formula-editor-input').press('ControlOrMeta+a');
  await page.evaluate(config => window.__FORMULA_BENCHMARK_TIMING__.arm(config), { kind: 'preview', timeoutMs, source: edited, result: String(first.subtotal + 17) });
  await page.keyboard.insertText(edited);
  await record('valid-expression', 'source-change-preview-and-validation', await timingResult(page, timeoutMs));

  const invalid = `prop("${dataset.fields.subtotal}") +`;
  await page.getByTestId('formula-editor-input').click();
  await page.getByTestId('formula-editor-input').press('ControlOrMeta+a');
  await page.evaluate(config => window.__FORMULA_BENCHMARK_TIMING__.arm(config), { kind: 'diagnostic', timeoutMs, source: invalid });
  await page.keyboard.insertText(invalid);
  const diagnostic = await timingResult(page, timeoutMs);
  // These exact pinned UI messages were observed in both real production apps.
  // Validate the text CAPTURED at the completed timestamp, outside the timer:
  // a caught Worker/action/preview failure must invalidate the sample even if
  // the UI subsequently recovers to a legitimate syntax error.
  const expectedDiagnostic = variant === 'legacy'
    ? `Unexpected end of formula [1,${invalid.length + 1}]`
    : 'expected expression after `+`';
  if (diagnostic.diagnosticText !== expectedDiagnostic) throw new Error(`Unexpected diagnostic endpoint for ${variant}: ${diagnostic.diagnosticText}`);
  await record('invalid-expression', 'source-change-current-diagnostic', { ...diagnostic, diagnosticCategory: 'syntax' });

  await source(page, dataset.totalExpression, timeoutMs);
  await previewReady(page, dataset.totalExpression, first.total, timeoutMs);
  await page.getByTestId('formula-editor-input').click();
  await page.getByTestId('formula-editor-input').press('ControlOrMeta+a');
  await page.evaluate(config => window.__FORMULA_BENCHMARK_TIMING__.arm(config), { kind: 'completion', timeoutMs, source: 'Sub', suggestionSelector: '[data-testid="formula-suggestion-Subtotal"]' });
  await page.keyboard.insertText('Sub');
  await record('property-completion', 'source-change-property-suggestion', await timingResult(page, timeoutMs));
  await page.getByTestId('formula-suggestion-Subtotal').click();
  await previewReady(page, `prop("${dataset.fields.subtotal}")`, first.subtotal, timeoutMs);
  await page.getByTestId('formula-editor-cancel').click();
  await page.getByTestId('formula-editor-dialog').waitFor({ state: 'hidden' });
  await until(page, ({ selector, expected }) => document.querySelector(selector)?.textContent?.trim() === expected, { selector: cellSelector(dataset, 'total'), expected: String(first.total) }, timeoutMs);
}

export async function runFullApp({ variants, chromium, options = {}, onSample = async () => {} }) {
  if (!Array.isArray(variants) || variants.length !== 2 || variants[0].id === variants[1].id || variants.some(variant => !['legacy', 'native'].includes(variant.id))) throw new Error('Full app benchmark requires the distinct pinned legacy and native variants');
  if (!options.backendUrl) throw new Error('Full app benchmark requires an explicit real Cloud backend URL');
  if (!options.credentialsPath && !options.authStatePath) throw new Error('Supply a private credentialsPath or authStatePath for real application authentication');
  const rowCounts = options.fullAppRows ?? options.rowCounts ?? [100];
  if (!Array.isArray(rowCounts) || !rowCounts.length || new Set(rowCounts).size !== rowCounts.length) throw new Error('Full app rowCounts must be a nonempty list without duplicates');
  for (const rows of rowCounts) positiveInteger(rows, null, 'row count');
  const sessions = positiveInteger(options.sessions, 5, 'sessions');
  const samplesPerSession = positiveInteger(options.samples, 20, 'samples');
  const warmups = nonnegativeInteger(options.warmups, 5, 'warmups');
  const timeoutMs = positiveInteger(options.timeoutMs, 60_000, 'timeoutMs');
  const outputDir = path.resolve(options.outputDir ?? '.benchmark-output');
  await mkdir(outputDir, { recursive: true });
  const health = await fetch(options.backendUrl.replace(/\/$/, '') + '/api/health', { signal: AbortSignal.timeout(timeoutMs) });
  if (!health.ok) throw new Error('Configured real Cloud backend failed its /api/health check');
  const backendInfo = await fetch(options.backendUrl.replace(/\/$/, '') + '/api/server-info', { headers: { 'x-platform': 'web' }, signal: AbortSignal.timeout(timeoutMs) })
    .then(async response => {
      if (!response.ok) return { available: false };
      const body = await response.json();
      const info = body.data ?? body;
      return { available: true, reportedVersion: typeof info.version === 'string' ? info.version : null, minWebClientVersion: typeof info.min_web_client_version === 'string' ? info.min_web_client_version : null, selfHosted: typeof info.self_hosted === 'boolean' ? info.self_hosted : null };
    }).catch(() => ({ available: false }));
  const credentials = options.credentialsPath ? await readPrivateJson(options.credentialsPath, 'credentials') : null;
  let authState = options.authStatePath ? await readPrivateJson(options.authStatePath, 'authentication state') : null;
  const fingerprint = digest({ backend: options.backendUrl.replace(/\/$/, ''), account: accountFingerprint(credentials, authState), workload: DATASET_OWNER });
  const privateCacheDir = path.join(path.resolve(options.cacheDir ?? path.join(outputDir, 'cache')), 'full-app-private', fingerprint);
  await mkdir(privateCacheDir, { recursive: true, mode: 0o700 });
  await chmod(privateCacheDir, 0o700);
  const manifestPath = path.join(privateCacheDir, 'datasets.json');
  const cachedAuthPath = path.join(privateCacheDir, 'authentication.json');
  if (!authState) authState = await readPrivateJson(cachedAuthPath, 'cached authentication', null);
  const manifest = await readPrivateJson(manifestPath, 'cached dataset manifest', { owner: DATASET_OWNER, datasets: {} });
  if (manifest.owner !== DATASET_OWNER) throw new Error('Existing private dataset manifest is not owned by this benchmark');
  const builds = [];
  for (const variant of variants) builds.push({ ...variant, ...await buildFullApp(variant, outputDir, { rebuild: options.rebuildFullApp }) });
  const server = await serveFullApp({ backendUrl: options.backendUrl, gotrueUrl: options.gotrueUrl, port: options.fullAppPort ?? 0 });
  const samples = [];
  const assetsByVariant = Object.fromEntries(variants.map(variant => [variant.id, { wasmResponses: 0, failedWasmResponses: 0 }]));
  const artifacts = { privateDatasetManifest: path.join(outputDir, 'full-app-dataset.private.json'), privateCacheManifest: manifestPath, privateAuthenticationState: path.join(outputDir, 'full-app-auth.private.json') };
  const launch = () => chromium.launch({ headless: true, ...(options.browserExecutablePath ? { executablePath: options.browserExecutablePath } : {}) });
  let browser;
  let browserVersion;
  let active;
  let stage = 'authentication and dedicated dataset setup';
  try {
    // Legacy authorship keeps persisted definitions readable by BOTH engines.
    const seedVariant = builds.find(variant => variant.id === 'legacy') ?? builds[0];
    server.setVariant(seedVariant.dist);
    browser = await launch();
    browserVersion = browser.version();
    active = await openAuthenticated(browser, server.origin, authState, credentials, timeoutMs);
    authState = await active.context.storageState();
    await privateJson(artifacts.privateAuthenticationState, authState);
    await privateJson(cachedAuthPath, authState);
    for (const count of rowCounts) {
      const rows = generateRows(count);
      let dataset = manifest.datasets[count];
      if (dataset && dataset.checksum !== digest(rows)) throw new Error('Existing benchmark dataset differs from the deterministic generator');
      if (!dataset) {
        await active.page.goto(server.origin + '/app', { waitUntil: 'domcontentloaded' });
        await active.page.getByTestId('sidebar-page-header').waitFor({ state: 'visible' });
        dataset = await seedDataset(active.page, rows, server.origin, timeoutMs);
        manifest.datasets[count] = dataset;
        await privateJson(manifestPath, manifest);
      } else {
        await active.page.goto(server.origin + dataset.route, { waitUntil: 'domcontentloaded' });
        // A failed prior sample may have left a changed input. Restoration is
        // restricted to ownership-verified benchmark data, before any timing.
        await restoreDataset(active.page, dataset, rows, timeoutMs);
      }
    }
    await privateJson(artifacts.privateDatasetManifest, manifest);
    if (active.errors.length) throw new Error('Dedicated benchmark setup raised an uncaught browser error');
    await active.context.close();
    active = null;
    await browser.close();
    browser = null;
    for (let session = 0; session < sessions; session++) {
      const order = session % 2 === 0 ? builds : [...builds].reverse();
      for (const variant of order) {
        server.setVariant(variant.dist);
        for (const count of rowCounts) {
          stage = `${variant.id}, ${count} rows, session ${session}`;
          const dataset = manifest.datasets[count];
          const rows = generateRows(count);
          // Every variant/session/dataset uses its OWN browser process. The
          // cold deep link is its first app navigation, before any /app visit.
          browser = await launch();
          active = await createContext(browser, server.origin, authState, timeoutMs, navigationConfig(dataset, rows, timeoutMs));
          const { page } = active;
          await page.goto(server.origin + dataset.route, { waitUntil: 'domcontentloaded' });
          const cold = await timingResult(page, timeoutMs);
          const record = async (scenario, operation, timing, iterationIndex) => {
            const sample = { layer: 'full-app', variant: variant.id, scenario, operation, rows: count, session, iteration: iterationIndex, elapsedMs: timing.elapsedMs, domReadyMs: timing.domReadyMs, stableFrameMs: timing.stableFrameMs, verified: true, inputChecksum: dataset.checksum, ...(timing.visibleRows === undefined ? {} : { visibleRows: timing.visibleRows, visibleFormulaCells: timing.visibleFormulaCells }), ...(timing.diagnosticText === undefined ? {} : { diagnosticText: timing.diagnosticText, diagnosticCategory: timing.diagnosticCategory }) };
            samples.push(sample);
            await onSample(sample);
          };
          // Verification is outside the interval, but MUST pass before its
          // captured cold sample is accepted.
          await verifyDataset(page, dataset, rows, timeoutMs);
          await record('dependency-chain', 'open-database-cold-visible-formulas', cold, 0);
          for (let iterationIndex = -warmups; iterationIndex < samplesPerSession; iterationIndex++) {
            stage = `${variant.id}, ${count} rows, session ${session}, iteration ${iterationIndex}`;
            await iteration(page, dataset, rows, server.origin, timeoutMs, variant.id, async (scenario, operation, timing) => {
              if (iterationIndex < 0) return;
              await record(scenario, operation, timing, iterationIndex);
            });
          }
          // Confirm restoration and durability before the next fresh browser
          // reads the very same server-side dataset and identities.
          await verifyDataset(page, dataset, rows, timeoutMs);
          const drained = await page.evaluate(timeoutMs => window.__FORMULA_BENCHMARK__.waitForDrain(undefined, { timeoutMs }), timeoutMs);
          if (!drained) throw new Error('Final baseline input restoration did not drain to Cloud');
          if (options.captureScreenshots && session === 0) {
            const gridPath = path.join(outputDir, `full-app-${variant.id}-${count}-grid.png`);
            await page.getByTestId('database-grid').screenshot({ path: gridPath });
            await page.locator(cellSelector(dataset, 'total')).evaluate(element => element.click());
            await page.getByTestId('formula-editor-dialog').waitFor({ state: 'visible' });
            await source(page, '1 + 2', timeoutMs);
            await previewReady(page, '1 + 2', 3, timeoutMs);
            const editorPath = path.join(outputDir, `full-app-${variant.id}-${count}-editor.png`);
            const dialogBox = await page.getByTestId('formula-editor-dialog').boundingBox();
            const previewBox = await page.getByTestId('formula-editor-preview').boundingBox();
            if (!dialogBox || !previewBox) throw new Error('Constant editor preview was not visible for evidence capture');
            // Lower docs can display canonical field IDs. Capture only the
            // controls, harmless constant source and current preview value.
            await page.screenshot({ path: editorPath, clip: { x: dialogBox.x, y: dialogBox.y, width: dialogBox.width, height: Math.min(dialogBox.height, previewBox.y + previewBox.height - dialogBox.y + 8) } });
            await page.getByTestId('formula-editor-cancel').click();
            await page.getByTestId('formula-editor-dialog').waitFor({ state: 'hidden' });
            (artifacts.publicScreenshots ??= []).push(gridPath, editorPath);
          }
          if (active.errors.length) throw new Error('Full application raised an uncaught browser error; samples are invalid');
          if (active.assets.failedWasmResponses) throw new Error('Full application failed to load its production WASM assets');
          assetsByVariant[variant.id].wasmResponses += active.assets.wasmResponses;
          assetsByVariant[variant.id].failedWasmResponses += active.assets.failedWasmResponses;
          authState = await active.context.storageState();
          await privateJson(cachedAuthPath, authState);
          await active.context.close();
          active = null;
          await browser.close();
          browser = null;
        }
      }
    }
    return {
      samples,
      metadata: {
        production: true, realCloudBackend: true, sameOriginProxy: true,
        backendInfo,
        variants: variants.map(({ id, revision }) => ({ id, revision })),
        builds: builds.map(({ id, metadata }) => ({ variant: id, ...metadata })),
        instrumentationHash, buildInstrumentationHash, timingScriptHash, instrumentation: 'Identical retained test IDs, real database dispatch/context bridge and browser timing observers; no formula implementation changes',
        viewport: VIEWPORT, timezone: 'UTC', locale: 'en-US', browserVersion,
        sessions, samplesPerSession, warmups, rowCounts,
        timer: 'Browser performance.now(): navigation start, captured Enter keydown, or captured editor beforeinput → correct current DOM stable for two animation frames',
        diagnosticValidation: { legacy: 'Unexpected end of formula [1,<source length + 1>]', native: 'expected expression after `+`', category: 'syntax', capture: 'Exact error text captured with the completed browser timestamp, then compared outside the timed interval' },
        cacheProfile: 'Fresh browser PROCESS per variant/session/dataset. Cold sample is its first authenticated deep link, before /app hydration or dataset verification. Hot repetitions retain HTTP/IndexedDB caches after untimed verification and warmups.',
        scheduling: 'Serial sessions alternate variant order; all operations restore baseline inputs and cancel editor drafts before the next sample',
        datasets: rowCounts.map(count => ({ rows: count, formulaFields: 2, checksum: manifest.datasets[count].checksum })),
        generator: 'Deterministic row-index data: Price=10+(index%91), Quantity=2+(index%9), Subtotal=Price*Quantity, Total=Subtotal+5',
        assetsByVariant,
        privateArtifactsExcluded: true,
      },
      limits: [
        'Default complete-app dataset is 100 rows. Other explicit rowCounts are supported; this layer reports visible formula DOM readiness, not evaluation of every database row.',
        'Cold means a fresh browser process and storage context, with saved authentication only. Cloud process and server caches are shared and are not reset between variants.',
        'Grid virtualization and the editor’s 50-row preview limit remain enabled. Whole navigation includes application loading, authentication hydration, real Cloud requests, and rendering.',
        'Setup creates dedicated persistent databases through the real application/row dispatch. Fresh-context verification checks every seeded input before measuring either variant. Setup, build, login, resets, and verification are excluded from samples.',
        'Retained test selectors, the setup bridge, and DOM observers add identical instrumentation to both production builds. Two animation frames are included in each UI readiness measurement.',
        'Input updates measure local responsive propagation; they do not measure the later Cloud durability acknowledgement. Editor drafts are cancelled, preserving legacy-compatible saved formulas.',
      ],
      artifacts,
    };
  } catch (error) {
    // Playwright call logs can contain the private login text or database IDs.
    // Keep diagnostic material private and expose only a sanitized stage error.
    const failurePath = path.join(outputDir, 'full-app-failure.private.json');
    await privateJson(failurePath, { stage, message: String(error?.message ?? error), stack: String(error?.stack ?? '') }).catch(() => {});
    if (active?.page) {
      const screenshotPath = path.join(outputDir, 'full-app-failure.private.png');
      await active.page.screenshot({ path: screenshotPath }).then(() => chmod(screenshotPath, 0o600)).catch(() => {});
    }
    throw new Error(`Complete AppFlowy benchmark failed during ${stage}; inspect full-app-failure.private.json in the private output directory`);
  } finally {
    await active?.context.close().catch(() => {});
    await browser?.close().catch(() => {});
    await server.close();
  }
}
