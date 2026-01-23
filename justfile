set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

fmt:
  cargo fmt --all && cd examples/vite && pnpm -s run format:fix

check:
  cargo check && cd examples/vite && pnpm -s run check

gen-ts:
  cargo run -p analyzer_wasm --bin export_ts

test: test-analyzer test-analyzer_wasm test-example-vite

test-analyzer:
  cargo test -p analyzer

test-analyzer_wasm:
  cargo test -p analyzer_wasm

test-analyzer-bless:
  BLESS=1 cargo test -p analyzer

test-example-vite:
  cd examples/vite && pnpm -s run wasm:build && pnpm -s run test && pnpm -s run test:e2e

run-example-vite:
  cd examples/vite && pnpm -s run wasm:build && npm run dev
