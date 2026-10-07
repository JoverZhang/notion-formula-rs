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

`results.json` contains source revisions, dataset and instrumentation details, the environment, limits, raw samples, and the comparison summary. `samples.csv` provides the raw measurement rows; `samples.ndjson` preserves progress if a later operation fails. The generated `report.md` is a convenience view of the JSON.

The [reporter](../../../tools/appflowy-benchmark/report.mjs) pairs samples by workload, operation, row count, session, and iteration. It reports absolute P50/P95 durations and the ratio **native / legacy**; values below 1 favor native. Its 95% interval resamples whole paired browser sessions 2,000 times with a fixed reporting seed. Fewer than five sessions yield no directional conclusion; zero-resolution medians yield no speedup ratio. Small-sample tail percentiles remain exploratory. Failures and unsupported cases are not silently removed.

Computed-chain JSON summaries include requested projected cells per second at the median duration. This rate includes the whole operation and its cache behavior; it does not isolate evaluator throughput. Full-app visible-result latency is not converted into whole-database throughput.

Run `just test-benchmark-report` to check pairing, correctness guards, zero-resolution handling, and raw CSV output. The real browser smoke run checks the production integration and independent result models. Machine-specific timing thresholds are not part of normal CI.
