---
doc_id: experiments.appflowy-formula-benchmark
title: "AppFlowy formula A/B benchmark"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-10-07
---

# AppFlowy formula A/B benchmark

[简体中文](README.zh-CN.md)

Compare the original JavaScript implementation and the Rust/WASM integration at fixed AppFlowy revisions. The [runner](../../../tools/appflowy-benchmark/run.mjs) records paired browser measurements, verifies outputs, and fails on missing samples or unequal work. A timing advantage in one workload does not establish an advantage in every database or environment.

## Reproduce a comparison

Install Node, the pinned AppFlowy package manager, Rust with the `wasm32-unknown-unknown` target, `wasm-pack`, and Chromium. Preparation fetches the [immutable revisions](../../../tools/appflowy-benchmark/revisions.json), installs each frozen lockfile, and builds the native revision's pinned SDK. Generated checkouts stay in the runner's cache, outside the source tree.

Without `--browser-executable`, the runner uses the Chromium bundled with the pinned Playwright package. Run `--prepare` first, then install that browser with `pnpm --dir <cache>/native exec playwright install chromium`.

```sh
# Validate the harness with small samples; these do not support performance claims.
just benchmark-appflowy --smoke --output /tmp/formula-benchmark-smoke \
  --browser-executable /path/to/chromium

# Default: four workloads × 100/1,000/10,000 rows, five independent sessions,
# five warmups and twenty measured repetitions per hot operation/session.
just benchmark-appflowy --output /tmp/formula-benchmark-chain \
  --browser-executable /path/to/chromium
```

Use a new output directory for each run. `--cache` selects the reusable checkout/build cache; `--families`, `--rows`, `--sessions`, `--samples`, and `--warmups` narrow an explicitly recorded configuration. `--help` lists the options. Builds finish before measurements begin, and only one version is measured at a time.

The cache lock prevents concurrent runs using the same generated checkouts. If a process is killed, check the PID in `benchmark.lock` before removing a stale lock. Avoid competing builds or benchmarks on the measurement machine.

The complete application layer requires a compatible AppFlowy Cloud test deployment and a normal workspace account. Pass credentials as a private JSON file with `email` and `password`, or use a private Playwright storage-state file. The runner creates its own benchmark databases; it does not use an existing user database as its workload.

```sh
just benchmark-appflowy --layer all --output /tmp/formula-benchmark-all \
  --backend-url http://127.0.0.1:8000 \
  --gotrue-url http://127.0.0.1:9999 \
  --credentials /private/appflowy-test-account.json \
  --full-app-rows 100 --browser-executable /path/to/chromium
```

Credentials, database identities, auth state, and detailed application failures are private files, separate from the shareable results. Do not publish those private files. `--layer full-app` runs only the complete application comparison. Its default representative database has 100 rows; larger full-app datasets require an explicit `--full-app-rows` selection.

The cache retains identities so subsequent runs reuse the same dedicated databases, which remain available for inspection. `--screenshots` captures only dummy Grid/editor content after UI measurements; these images can be shared separately.

## What is measured

| Layer | Start → completion | Included work |
| --- | --- | --- |
| Computed chain | First request or input/formula mutation → every requested projected result | Production Yjs decoding, schema/Engine initialization when required, dependencies, cache behavior, native batch conversion, Worker/WASM round trips, and AppFlowy result projection |
| Complete application | Navigation, input submission, or editor input → the current correct visible result | Production application assets, real Cloud loading, scheduling, rendering, and native Worker/WASM; subsequent operations retain the documented cache state |

The [chain workloads](../../../tools/appflowy-benchmark/browser/dataset.ts) cover arithmetic, text/lists, a dependency chain/diamond, and an invoice workflow with a long saved formula. Both variants request identical rows and formula targets. Data construction precedes the timer; independent output models and checksums run after it stops. These workloads have no wall-clock-dependent formulas.

Each chain session starts a new browser process. Its first evaluation is reported separately; hot operations are repeated reads, one-row edits, all-row edits, and saved-formula changes. Production cache policies remain enabled. In particular, the legacy result cache holds at most 512 entries / 4 MiB, so repeated reads of larger working sets may evict entries. A repeated read is not labeled a guaranteed cache hit.

The complete application uses the original Grid and editor with the same viewport and data in both builds. The benchmark retains test selectors and injects a small setup bridge into both production builds; instrumentation hashes are recorded. The Grid remains virtualized. Visible-row readiness and complete-column computation are distinct workloads, and their results are reported separately. Input updates measure local propagation, not the later Cloud durability acknowledgement.

Each complete-app session also starts a fresh browser process. Its first authenticated database navigation is cold with respect to browser caches; Cloud caches remain shared. Later navigation, cell edits, preview updates, invalid-expression diagnostics, and property completion use the warmed session. UI readiness requires two stable animation frames; raw samples separate initial DOM readiness (`domReadyMs`) from that confirmation interval (`stableFrameMs`).

Browser timestamps define all durations. Playwright polls only to collect completed timestamps. Stale results, pending rows, incorrect values, unexpected errors, and different requested work fail validation rather than becoming fast samples. Timed runs do not intercept and serialize every Worker message. Memory and separately instrumented CPU profiles are outside this initial latency comparison.

## Read the evidence

`results.json` contains application and benchmark source revisions/hashes, dataset and instrumentation details, the environment, limits, raw samples, and the comparison summary. Browser timer code is included in the recorded provenance. Atomic checkpoints preserve the current phase and completed layers; an interrupted report remains `running` or `failed`. `samples.csv` provides the raw measurement rows; `samples.ndjson` preserves progress if a later operation fails. The generated `report.md` is a convenience view of the JSON.

The [reporter](../../../tools/appflowy-benchmark/report.mjs) pairs samples by workload, operation, row count, session, and iteration. It reports absolute P50/P95 durations and the ratio **native / legacy**; values below 1 favor native. Its 95% interval resamples whole paired browser sessions 2,000 times with a fixed reporting seed. Fewer than five sessions yield no directional conclusion; zero-resolution medians yield no speedup ratio. Small-sample tail percentiles remain exploratory. Failures and unsupported cases are not silently removed.

Computed-chain JSON summaries include requested projected cells per second at the median duration. This rate includes the whole operation and its cache behavior; it does not isolate evaluator throughput. Full-app visible-result latency is not converted into whole-database throughput.

Run `just test-benchmark-report` to check pairing, correctness guards, zero-resolution handling, and raw CSV output. The real browser smoke run checks the production integration and independent result models. Machine-specific timing thresholds are not part of normal CI.

## 2026-10-07 observations

This run supports faster large-dataset computation for the native integration, while the tested complete editor interactions are slower. It does not establish that the Rust evaluator alone is faster: the comparison includes the implementations' different caching, batching, Worker, scheduling, and rendering behavior.

The machine was Linux x64 with an AMD Ryzen 9 7950X, 32 logical CPUs, and 66.5 GB RAM. Both versions used Chrome for Testing 145.0.7632.6, Node 25.9.0, and pnpm 10.9.0. AppFlowy pins were legacy `4fdb4f4`, native `c4485cd`, and SDK `43b393c`; the real backend reported Cloud 0.19.0. Exact revisions, build hashes, and [backend image identities](2026-10-07/backend-images.json) accompany the evidence. Each comparison used five independent paired sessions, two warmups and five measured repetitions per hot operation per session: five cold/initial samples and 25 hot samples per variant and case.

The original all-layer attempt at benchmark revision `535a5e9` **failed** during full-app setup because a disconnected WebSocket raised an unhandled `ECONNRESET`, after all 2,520 chain samples had completed. The [original append-only log](2026-10-07/chain-samples.ndjson) is preserved byte-for-byte. The [recovery context](2026-10-07/chain-recovery-context.json) distinguishes recorded samples from metadata reconstructed using the preceding smoke run and matching cache build markers; original live load averages are unavailable. The [recovered chain report](2026-10-07/chain-recovered.json) checks the entire expected sample matrix and retains the original failed-attempt status. It is not a successful all-layer run.

After fixing transport cleanup and adding report checkpoints, the separate full-app run at `9ee4aaf` **passed all 260 samples** with the same application pins, browser, and dedicated database. [UI results](2026-10-07/full-app-results.json), [CSV](2026-10-07/full-app-samples.csv), and [append-only samples](2026-10-07/full-app-samples.ndjson) preserve that run independently. No timing sample was removed from either measured layer.

For the chain, all numbers below are P50 milliseconds, **legacy → native**. The complete [chain CSV](2026-10-07/chain-samples.csv) and JSON include all sizes, five operations, P95, and paired-session intervals.

| Workload | First evaluation, 100 rows | First evaluation, 10,000 rows | Edit all rows, 10,000 rows | Repeated read, 10,000 rows |
| --- | ---: | ---: | ---: | ---: |
| Arithmetic | 8.8 → 35.3 | 386.7 → 136.4 | 409.9 → 120.8 | 367.7 → 5.9 |
| Text/list | 10.8 → 38.5 | 532.5 → 261.0 | 550.2 → 236.7 | 496.5 → 8.9 |
| Dependencies | 42.4 → 38.1 | 3189.7 → 178.5 | 3047.9 → 155.6 | 3035.9 → 12.7 |
| Long invoice formula | 13.5 → 38.9 | 622.8 → 204.7 | 657.9 → 168.8 | 609.5 → 6.5 |

The small initial workloads expose native setup costs. The large repeated-read advantage includes legacy cache eviction versus native snapshot reuse; it must not be reported as raw Rust-versus-JavaScript arithmetic throughput.

For the complete UI, the database has 100 rows and two dependent formula fields. Navigation readiness covers 23 visible rows / 46 formula cells. The interval is for the ratio of native to legacy P50; an interval crossing 1 is inconclusive.

| UI operation | Legacy P50 / P95 (ms) | Native P50 / P95 (ms) | Native / legacy [95% interval] |
| --- | ---: | ---: | ---: |
| Cold database open | 1351.6 / 1405.6 | 1409.0 / 2267.6 | 1.04 [0.99, 1.93] |
| Warm database open | 913.9 / 2011.0 | 958.7 / 2105.7 | 1.05 [1.02, 1.10] |
| Input submit → dependent result | 22.3 / 39.1 | 39.1 / 70.5 | 1.75 [1.74, 1.77] |
| Formula change → valid preview | 24.0 / 25.0 | 223.6 / 256.7 | 9.32 [9.26, 9.41] |
| Formula change → current diagnostic | 27.3 / 27.9 | 93.9 / 94.4 | 3.44 [3.41, 3.45] |
| Formula change → property suggestion | 27.2 / 29.1 | 93.5 / 94.1 | 3.44 [3.43, 3.46] |

The native editor includes a [70 ms analysis debounce](https://github.com/JoverZhang/AppFlowy-Web/blob/c4485cdec14ed4e089e2472c882237d8a4ec0c3e/src/components/database/components/property/formula/FormulaEditor.tsx#L213) and a [120 ms preview debounce](https://github.com/JoverZhang/AppFlowy-Web/blob/c4485cdec14ed4e089e2472c882237d8a4ec0c3e/src/components/database/components/property/formula/use-native-preview.ts#L287). These are concrete contributors to user-visible waiting and candidates for a later optimization experiment. This latency run does not isolate their cost from the Engine or rendering. Both versions retain the two-frame readiness check. Cold opening shows no clear directional result; the five hot UI operations favor legacy in this sample.

Captured after measurements: [legacy Grid](2026-10-07/full-app-legacy-100-grid.png), [native Grid](2026-10-07/full-app-native-100-grid.png), [legacy editor](2026-10-07/full-app-legacy-100-editor.png), and [native editor](2026-10-07/full-app-native-100-editor.png). The editor images use the harmless draft `1 + 2`; they demonstrate the complete interface, not the timed expressions.

To repeat these measurement counts, add `--sessions 5 --samples 5 --warmups 2` to the commands above. To verify the recovered chain evidence without running a browser, use a new destination:

```sh
node docs/experiments/appflowy-formula-benchmark/2026-10-07/recover-chain.mjs \
  /tmp/formula-benchmark-recovered-check
```

Recovery verifies the original SHA-256, exact sample coverage and pairing before regenerating statistics. It does not rerun measurements or turn the failed attempt into a passing run. These findings cover one host, one Chromium version and synthetic workloads; the five-session intervals and small-sample P95 values warrant follow-up measurement before a broader performance claim.
