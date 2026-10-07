// Runs in Chromium. Playwright polling retrieves a completed browser timestamp;
// it is never the timer. Two animation frames require a stable rendered result.
export function installBrowserTiming(navigation) {
  window.__FORMULA_BENCHMARK_ACTIVE__ = true;
  let pending;
  const visible = element => {
    if (!element) return false;
    const rect = element.getBoundingClientRect();
    return rect.width > 0 && rect.height > 0 && rect.bottom > 0 && rect.top < innerHeight && rect.right > 0 && rect.left < innerWidth;
  };
  const text = selector => document.querySelector(selector)?.textContent?.trim() ?? '';
  const condition = config => {
    if (config.kind === 'grid') {
      const rows = [...document.querySelectorAll('[data-testid^="grid-row-"]')].filter(element => visible(element) && element.getAttribute('data-testid') !== 'grid-row-undefined');
      if (rows.length < config.minimumVisibleRows) return null;
      const ids = [];
      for (const row of rows) {
        const id = row.getAttribute('data-testid').slice('grid-row-'.length);
        const expected = config.expected[id];
        if (!expected) return null;
        for (const [fieldId, result] of Object.entries(expected)) {
          const cell = row.querySelector(`.grid-row-cell[data-column-id="${fieldId}"]`);
          if (!visible(cell) || cell.textContent?.trim() !== result || cell.querySelector('[data-evaluation-state="pending"], [data-error-source]')) return null;
        }
        ids.push(id);
      }
      return { signature: ids.join(','), visibleRows: ids.length, visibleFormulaCells: ids.length * config.formulaFields };
    }
    if (config.kind === 'cell') {
      if (text(config.inputSelector) !== config.input || text(config.resultSelector) !== config.result) return null;
      return { signature: config.result };
    }
    const input = document.querySelector('[data-testid="formula-editor-input"]');
    if (input?.getAttribute('data-value') !== config.source) return null;
    const error = document.querySelector('[data-testid="formula-editor-error"]');
    const done = document.querySelector('[data-testid="formula-editor-done"]');
    if (config.kind === 'preview') {
      if (visible(error) || !done || done.disabled || text('[data-testid="formula-preview-value"]') !== config.result) return null;
      return { signature: config.result };
    }
    if (config.kind === 'diagnostic') {
      if (!visible(error) || !error.textContent?.trim() || !done?.disabled || text('[data-testid="formula-preview-value"]')) return null;
      return { signature: error.textContent.trim() };
    }
    if (config.kind === 'completion') {
      const suggestion = document.querySelector(config.suggestionSelector);
      if (!visible(suggestion)) return null;
      return { signature: config.source };
    }
    return null;
  };
  const check = () => {
    const record = pending;
    if (!record || record.start === undefined || record.framePending || record.done) return;
    const match = condition(record.config);
    if (!match) { record.firstReadyAt = undefined; return; }
    if (record.firstReadyAt === undefined) record.firstReadyAt = performance.now();
    record.framePending = true;
    requestAnimationFrame(() => requestAnimationFrame(() => {
      record.framePending = false;
      if (pending !== record || record.done) return;
      const next = condition(record.config);
      if (!next || next.signature !== match.signature) { record.firstReadyAt = undefined; check(); return; }
      const completedAt = performance.now();
      record.done = true;
      clearTimeout(record.timer);
      window.__FORMULA_BENCHMARK_TIMING__.result = { elapsedMs: completedAt - record.start, domReadyMs: record.firstReadyAt - record.start, stableFrameMs: completedAt - record.firstReadyAt, startedAt: record.start, completedAt, visibleRows: next.visibleRows, visibleFormulaCells: next.visibleFormulaCells };
    }));
  };
  window.__FORMULA_BENCHMARK_TIMING__ = {
    result: null,
    arm(config) {
      if (pending) clearTimeout(pending.timer);
      this.result = null;
      pending = { config, start: config.kind === 'grid' ? 0 : undefined };
      const record = pending;
      record.timer = setTimeout(() => {
        if (pending === record && !record.done) {
          record.done = true;
          this.result = { error: 'Current UI result did not become ready within the configured timeout' };
        }
      }, config.timeoutMs);
      check();
    },
  };
  document.addEventListener('keydown', event => {
    if (pending?.config.kind === 'cell' && pending.start === undefined && event.key === 'Enter' && event.target instanceof HTMLTextAreaElement) {
      pending.start = performance.now();
      check();
    }
  }, true);
  document.addEventListener('beforeinput', event => {
    if (pending && pending.config.kind !== 'grid' && pending.config.kind !== 'cell' && pending.start === undefined && event.target instanceof Element && event.target.closest('[data-testid="formula-editor-input"]')) {
      pending.start = performance.now();
      check();
    }
  }, true);
  new MutationObserver(check).observe(document, { subtree: true, childList: true, characterData: true, attributes: true });
  if (navigation) window.__FORMULA_BENCHMARK_TIMING__.arm(navigation);
}

export async function timingResult(page, timeoutMs) {
  await page.waitForFunction(() => window.__FORMULA_BENCHMARK_TIMING__?.result, null, { timeout: timeoutMs + 2_000 });
  const result = await page.evaluate(() => window.__FORMULA_BENCHMARK_TIMING__.result);
  if (result.error) throw new Error(result.error);
  if (!Number.isFinite(result.elapsedMs) || result.elapsedMs < 0) throw new Error('Invalid browser timing');
  return result;
}
