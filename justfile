set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

test-analyzer:
  cargo test -p analyzer

test-analyzer_wasm:
  cargo test -p analyzer_wasm

test-analyzer-bless:
  BLESS=1 cargo test -p analyzer

run-example-web:
  wasm-pack build analyzer_wasm --target web --out-dir ../examples/web/pkg
  python3 -m http.server -d examples/web 8000

run-example-vite:
  wasm-pack build analyzer_wasm --target web --out-dir ../examples/vite/src/pkg
  cd examples/vite && npm run dev
