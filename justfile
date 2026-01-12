set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

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

test-all: test-analyzer test-analyzer_wasm test-example-vite
