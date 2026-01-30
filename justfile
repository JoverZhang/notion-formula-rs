set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

build:
  cd examples/vite && pnpm -s run wasm:build && pnpm -s i && pnpm -s run build

check:
  cargo check && cargo clippy && cd examples/vite && pnpm -s run check

fix:
  cargo clippy --fix --allow-dirty --allow-staged && cd examples/vite && pnpm -s run lint:fix

fmt:
  cargo fmt --all && cd examples/vite && pnpm -s run format:fix

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

clean:
  cargo clean
  cd examples/vite && rm -rf node_modules dist src/pkg test-results
