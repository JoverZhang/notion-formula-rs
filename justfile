set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

test-analyzer:
  cargo test -p analyzer

test-analyzer-bless:
  BLESS=1 cargo test -p analyzer

