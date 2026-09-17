---
doc_id: contributing.spec-codegen
title: "Generate Rust headers from Markdown"
language: en
source_language: en
counterpart: ./spec-codegen.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-17
---

# Generate Rust headers from Markdown

[简体中文](spec-codegen.zh-CN.md)

`spec-codegen` turns selected Markdown declarations into Rust headers. Keep interface declarations
in a canonical document and private state and method bodies in handwritten Rust. Existing Engine
and builtin declarations have not been migrated.

## Mark declarations, not examples

Use one `header=<snake_case_name>` attribute. Multiple blocks in the same document generate one
`<name>.h.rs` file, in document order. Select only the canonical document, not its translations.

````markdown
<details>
<summary>Imports</summary>

```rust header=counter
use std::string::String;
```

</details>

```rust header=counter
pub struct Counter {
    pub label: String,
}
```

```rust header=counter
impl Counter {
    pub fn new(label: String) -> Self;
    pub fn value(&self) -> u64;
}
```
````

Keep a blank line after `<summary>` so Markdown parses the fence rather than treating it as raw HTML.
Blockquotes and lists also work. Each marked fence must close explicitly and contain complete
declarations. Unmarked fences, including outer fences that display Markdown examples, are ignored.

```text
use / enum / type / ordinary struct → preserve declaration
struct + bodyless inherent impl    → inject private inner: <Type>Inner
receiver method                   → self.<method>_impl(arguments)
associated function               → Self::<method>_impl(arguments)
```

All blocks are collected before classification, so methods may precede their struct. Repeated impl
blocks are allowed; duplicate declarations and methods are not. A header key cannot belong to two
input documents.

The generated structure retains the public field:

```rust
pub struct Counter {
    pub label: String,    // Declared visibility is preserved.
    inner: CounterInner, // Injected private storage.
}
```

Write the implementation in `counter.rs`:

```rust
use spec_header::include_header;

include_header!(counter); // Same module, not a nested module.

struct CounterInner {
    value: u64,
}

impl Counter {
    fn new_impl(label: String) -> Self {
        Self { label, inner: CounterInner { value: 0 } }
    }

    fn value_impl(&self) -> u64 {
        self.inner.value
    }
}
```

Callers can read and write `counter.label` directly. They cannot access `inner` or construct the
struct with an external struct literal. The generator does not initialize fields, require
`Default`, or implement behavior. Include each header once per consuming module.

## Connect generation to the build

For a consumer crate in a top-level workspace directory:

```toml
[dependencies]
spec_header = { path = "../tools/spec-codegen/header-support" }

[build-dependencies]
spec-codegen = { path = "../tools/spec-codegen" }
```

Keep the canonical source inside the consumer package, for example `spec/api.md`.
In its `build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=spec/api.md");
    spec_codegen::generate(
        &["spec/api.md"], // Pass all canonical inputs in one invocation.
        std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"),
    ).expect("generate Rust headers"); // Failure must stop the build.
}
```

Register every source with `rerun-if-changed`. Ensure package inclusion rules retain those sources;
check with `cargo package --list`. The macro expands in the consumer's compilation context and only
includes `OUT_DIR/<key>.h.rs`. The normal dependency `spec_header` is `no_std` and has no dependencies;
parsers remain build-time dependencies.

The CLI uses the same library and accepts an explicit output directory:

```sh
cargo run -p spec-codegen -- --out-dir target/spec-headers path/to/api.md
just test-spec-codegen
```

## Know what is checked

- Ordinary data declarations can be generic. Facades require non-generic named-field structs.
- Methods support `self`, `&self`, `&mut self`, or no receiver, with named parameters.
- `doc` and `derive` attributes are preserved. Unsupported attributes are rejected.
- Trait impls, method bodies, generic facade methods (including argument-position `impl Trait`), typed receivers, destructuring, and
  `async`/`const`/`unsafe`/`extern`/variadic methods are rejected.
  Return-position `impl Trait` does not introduce generic parameters and is preserved.
- `inner`, `<Type>Inner`, and `*_impl` are reserved for facade storage and handwritten hooks.
- Missing or incompatible hooks fail Rust compilation. Tests and review still own behavioral
  correctness and any additional handwritten public methods.

Generation diagnostics report the input path and Markdown line. Headers record declaration and
method provenance without timestamps or machine-specific absolute paths. Rust compiler diagnostics
may point to the generated header; they are not automatically remapped to Markdown.

The output directory's `.spec-codegen-manifest` tracks owned headers. A successful run removes
obsolete headers when keys disappear or change, preserving unrelated files. Existing unowned files,
invalid manifests, and output-file symlinks are rejected. Do not edit generated files or run multiple
generations concurrently against one output directory.

Inputs are validated before writing; replacements are atomic per file, not a transaction across
all headers. Propagate every failure: old files remaining after a failed run are not a fallback.

The [generator](../../tools/spec-codegen/src/lib.rs) and
[Cargo consumer test](../../tools/spec-codegen/tests/cargo_consumer.rs) provide implementation and
end-to-end examples. Builtin-catalog migration remains separate work.
