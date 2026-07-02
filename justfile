set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

ci-image := "notion-formula-rs-ci:local"

# Dependencies

deps: deps-rust deps-node

deps-rust:
  cargo fetch --locked
  cargo fetch --locked --target wasm32-unknown-unknown

deps-node:
  pnpm -C examples/vite install --frozen-lockfile

# CI

verify: deps check test

_docker-build-ci:
  docker buildx build --load -f Dockerfile.ci -t {{ci-image}} .

docker-test: _docker-build-ci
  docker run --rm --ipc=host -e CI=true -v "$PWD:/work" -w /work {{ci-image}} just verify

# Checks and fixes

check:
  cargo fmt --all -- --check
  cargo check
  cargo clippy
  pnpm -C examples/vite -s run check

typecheck:
  cargo check

fix:
  cargo clippy --fix --allow-dirty --allow-staged
  cargo fmt --all
  pnpm -C examples/vite -s run lint:fix
  pnpm -C examples/vite -s run format:fix

# Build and dev

gen-ts:
  cargo run -p analyzer_wasm --bin export_ts

wasm:
  pnpm -C examples/vite -s run wasm:build

build: deps-node wasm
  pnpm -C examples/vite -s run build

run-example-vite: deps-node wasm
  pnpm -C examples/vite -s run dev

clean:
  cargo clean
  cd examples/vite && rm -rf node_modules dist src/pkg test-results

# Tests

test: test-rust test-example-vite

test-rust: test-builtin_fn test-analyzer test-evaluator test-ide test-analyzer_wasm

test-builtin_fn:
  cargo test -p builtin_fn

test-analyzer:
  cargo test -p analyzer

test-evaluator:
  cargo test -p evaluator

test-ide:
  cargo test -p ide

test-analyzer_wasm:
  cargo test -p analyzer_wasm
  wasm-pack test --node analyzer_wasm

test-analyzer-bless:
  BLESS=1 cargo test -p analyzer

test-ide-bless:
  BLESS=1 cargo test -p ide format_golden

bless: test-analyzer-bless test-ide-bless

test-example-vite: deps-node wasm
  pnpm -C examples/vite -s run test
  pnpm -C examples/vite -s run test:e2e
