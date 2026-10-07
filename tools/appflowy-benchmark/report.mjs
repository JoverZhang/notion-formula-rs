const dimensions = ['layer', 'scenario', 'operation', 'rows'];
const csvFields = [...dimensions, 'variant', 'session', 'iteration', 'elapsedMs', 'rowCount', 'formulaCount',
  'checksum', 'inputChecksum', 'visibleRows', 'visibleFormulaCells', 'domReadyMs', 'stableFrameMs', 'revision', 'verified'];

function quantile(values, probability) {
  const sorted = [...values].sort((a, b) => a - b);
  const index = (sorted.length - 1) * probability;
  const lower = Math.floor(index);
  return sorted[lower] + (sorted[Math.ceil(index)] - sorted[lower]) * (index - lower);
}

function statistics(samples) {
  const values = samples.map((sample) => sample.elapsedMs);
  const p50Ms = quantile(values, 0.5);
  const first = samples[0];
  const projectedCells = first.layer === 'chain' && samples.every((sample) =>
    sample.rowCount === first.rowCount && sample.formulaCount === first.formulaCount)
    ? first.rowCount * first.formulaCount : NaN;
  return { count: values.length, sessions: new Set(samples.map((sample) => sample.session)).size,
    p50Ms, p95Ms: quantile(values, 0.95), minMs: Math.min(...values), maxMs: Math.max(...values),
    ...(Number.isFinite(projectedCells) ? { projectedCellsPerSecond: p50Ms > 0 ? projectedCells * 1000 / p50Ms : null } : {}) };
}

// Resample whole paired browser sessions, not correlated iterations as if they
// were independent trials. The seed fixes report generation, not measurements.
function ratioInterval(pairs) {
  const blocks = new Map();
  for (const pair of pairs) {
    const block = blocks.get(pair.legacy.session) ?? [];
    block.push(pair);
    blocks.set(pair.legacy.session, block);
  }
  const sessions = [...blocks.values()];
  if (sessions.length < 5) return null;
  let seed = 0x47183a21;
  const ratios = [];
  for (let iteration = 0; iteration < 2000; iteration++) {
    const legacy = [];
    const native = [];
    for (let draw = 0; draw < sessions.length; draw++) {
      seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
      for (const pair of sessions[seed % sessions.length]) {
        legacy.push(pair.legacy.elapsedMs);
        native.push(pair.native.elapsedMs);
      }
    }
    const denominator = quantile(legacy, 0.5);
    if (denominator <= 0) return null;
    ratios.push(quantile(native, 0.5) / denominator);
  }
  return [quantile(ratios, 0.025), quantile(ratios, 0.975)];
}

export function summarize(samples) {
  if (samples.length === 0) throw new Error('No measurement samples');
  const groups = new Map();
  for (const sample of samples) {
    if (!['legacy', 'native'].includes(sample.variant) || sample.verified !== true || sample.warmup === true ||
        !Number.isFinite(sample.elapsedMs) || sample.elapsedMs < 0 ||
        !Number.isInteger(sample.rows) || sample.rows < 1 ||
        !Number.isInteger(sample.session) || sample.session < 0 ||
        !Number.isInteger(sample.iteration) || sample.iteration < 0 ||
        dimensions.slice(0, 3).some((field) => typeof sample[field] !== 'string' || !sample[field])) {
      throw new Error(`Invalid measurement: ${JSON.stringify(sample)}`);
    }
    const key = JSON.stringify(dimensions.map((field) => sample[field]));
    const group = groups.get(key) ?? new Map();
    const pairKey = `${sample.session}:${sample.iteration}`;
    const pair = group.get(pairKey) ?? {};
    if (pair[sample.variant]) throw new Error(`Duplicate measurement: ${key} ${pairKey} ${sample.variant}`);
    pair[sample.variant] = sample;
    group.set(pairKey, pair);
    groups.set(key, group);
  }
  return [...groups.entries()].map(([key, group]) => {
    const pairs = [...group.values()];
    for (const pair of pairs) {
      if (!pair.legacy || !pair.native) throw new Error(`Unpaired measurement: ${key}`);
      for (const field of ['checksum', 'inputChecksum', 'rowCount', 'formulaCount', 'visibleRows', 'visibleFormulaCells']) {
        if (pair.legacy[field] !== pair.native[field]) throw new Error(`Unequal ${field}: ${key}`);
      }
    }
    const legacy = statistics(pairs.map((pair) => pair.legacy));
    const native = statistics(pairs.map((pair) => pair.native));
    const nativeOverLegacy = legacy.p50Ms > 0 && native.p50Ms > 0 ? native.p50Ms / legacy.p50Ms : null;
    const ratio95 = nativeOverLegacy === null ? null : ratioInterval(pairs);
    const conclusion = nativeOverLegacy === null ? 'below-clock-resolution' :
      legacy.sessions < 5 ? 'insufficient-sessions' :
        ratio95 === null ? 'below-clock-resolution' :
          ratio95[1] < 1 ? 'native-faster' : ratio95[0] > 1 ? 'legacy-faster' : 'inconclusive';
    return { ...Object.fromEntries(dimensions.map((field, index) => [field, JSON.parse(key)[index]])),
      legacy, native, nativeOverLegacy, ratio95, conclusion };
  });
}

export function samplesCsv(samples) {
  const escape = (value) => {
    const text = value === undefined ? '' : String(value);
    return /[",\n\r]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
  };
  return [csvFields.join(','), ...samples.map((sample) => csvFields.map((field) => escape(sample[field])).join(','))].join('\n') + '\n';
}

export function renderReport(report) {
  const number = (value) => value == null ? '—' : Number(value.toPrecision(4)).toString();
  const lines = [
    '# AppFlowy formula comparison', '',
    `Run: ${report.createdAt}. Status: ${report.status}.`, '',
    'Native / legacy compares median elapsed time: values below 1 favor native. Results apply only to these workloads and this environment.', '',
    '| Layer / scenario / operation | Rows | Samples per version | Legacy P50 / P95 (ms) | Native P50 / P95 (ms) | Native / legacy [95% interval] | Conclusion |',
    '| --- | ---: | ---: | ---: | ---: | --- | --- |',
  ];
  for (const row of report.summary ?? []) {
    lines.push(`| ${row.layer} / ${row.scenario} / ${row.operation} | ${row.rows} | ${row.native.count} | ${number(row.legacy.p50Ms)} / ${number(row.legacy.p95Ms)} | ${number(row.native.p50Ms)} / ${number(row.native.p95Ms)} | ${number(row.nativeOverLegacy)}${row.ratio95 ? ` [${row.ratio95.map(number).join(', ')}]` : ''} | ${row.conclusion} |`);
  }
  lines.push('', 'The interval uses 2,000 deterministic paired-session bootstrap resamples. Fewer than five independent sessions do not support a directional conclusion. Zero-resolution medians do not produce speedup ratios. Tail percentiles from small sample counts are exploratory.', '',
    'All result checks execute after elapsed time is captured. Correctness errors, missing pairs, unequal work and invalid timings fail the run; they are not silently excluded.', '',
    'Computed-chain JSON summaries also report requested projected cells per second at the median duration, including the complete operation and its cache behavior. This is not raw evaluator throughput. Full-app UI latency is not converted into whole-database throughput.', '',
    'See results.json for versions, instrumentation hashes, environment, setup limits and raw measurements; samples.csv contains the same measurement rows.', '');
  return lines.join('\n');
}
