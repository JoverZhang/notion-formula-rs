---
doc_id: how.analyzer-wasm
title: "How analyzer_wasm bridges JavaScript and Rust"
language: en
source_language: en
counterpart: ./README.zh-CN.md
implementation_status: current
document_status: stable
translation_status: synced
last_verified: 2026-09-01
---

# How analyzer_wasm bridges JavaScript and Rust

[简体中文](README.zh-CN.md)

The `analyzer_wasm` crate is the translation layer between JavaScript editor data and the Rust
`analyzer` and `ide` crates. It keeps configured semantic context in an exported `Analyzer`,
converts source coordinates at each call boundary, and serializes internal results into DTOs under
the `dto::v1` namespace. This guide explains the Current implementation for maintainers who need to
change or debug that bridge.

This crate does not define formula semantics or IDE algorithms. It also does not own the stable
external WASM contract, which belongs in `docs/specs/`. Method names and data-flow details below
describe the implementation that serves that contract; they are not a second API specification.

## The facade keeps configuration, not documents

[`Analyzer`](../../../analyzer_wasm/src/lib.rs) is a `wasm_bindgen` export that stores an Analyzer
`Context` and a completion preference limit. Construction first checks that the incoming `JsValue`
is an object with only recognized top-level keys, then uses `serde_wasm_bindgen` to deserialize
`dto::v1::AnalyzerConfig`. The constructor converts configured properties into Analyzer properties,
sets `disabled_reason` to `None`, installs the Rust builtin function registry, and resolves the
default preference limit.

Every exported operation takes `&self`. An instance therefore reuses its immutable semantic
configuration, but does not retain source text, parse trees, diagnostics, or edit history. Each
`analyze`, `format`, `apply_edits`, or `help` call supplies a complete source string and builds fresh
core results.

```text
JsValue / UTF-16 positions
            |
            v
 wasm_bindgen Analyzer
  | config state |
  +--------------+
            |
     deserialize inputs
     convert coordinates
            |
            v
     analyzer or ide
       byte positions
            |
            v
  stateless Converter
  core types -> dto::v1
            |
            v
 serde_wasm_bindgen::to_value
```

Read the diagram from a JavaScript call at the top to the serialized result at the bottom. The
configuration box is the only state held between calls. The diagram intentionally omits analysis,
completion, signature, formatting, and edit algorithms owned by the downstream crates.

## Each method composes a different internal call path

The exports share serialization helpers but compose different helpers. This table is a source-reading
map, not the caller contract for accepted inputs or competing failures; those guarantees belong to
the WASM API specification.

| Method | Current calls in `lib.rs` |
| --- | --- |
| `new` | `validate_config_keys` -> `from_value::<AnalyzerConfig>` -> build `Context` |
| `analyze` | `analyzer::analyze` -> `Converter::analyze_output` -> `to_value` |
| `format` | `utf16_to_8_cursor` -> `ide::format` -> `utf8_to_16_offset` -> `to_value` |
| `apply_edits` | deserialize edits -> `utf16_to_8_text_edits` -> `utf16_to_8_cursor` -> `ide::apply_edits` -> reverse cursor conversion -> `to_value` |
| `help` | `utf16_to_8_offset` -> `ide::help` -> `Converter::help_output_view` -> `to_value` |

When debugging, follow the row until the first helper that owns the suspect representation.
`apply_edits`, for example, prepares DTO edits and byte coordinates before it delegates to IDE.
Sorting, overlap checks, edit application, and cursor rebasing after byte conversion remain IDE
responsibilities. The WASM specification, rather than this call map, defines which error a caller
observes when more than one input condition is invalid.

`analyze` passes the instance `Context` directly to Analyzer. Its converter does not reinterpret
diagnostics or inferred types. `format` and `apply_edits` similarly delegate the operation after
coordinate conversion. `help` supplies the saved preference limit, but completion and signature
mechanics stay in `ide`.

## Coordinate conversion keeps Rust on scalar boundaries

Rust core spans and cursors are UTF-8 byte offsets. JavaScript editor positions arriving at this
crate are UTF-16 code-unit offsets. [`offsets.rs`](../../../analyzer_wasm/src/offsets.rs) centralizes
both directions. The WASM specification owns the accepted-input and externally visible coordinate
rules; this section explains the loops that implement them:

- `Converter::utf16_to_8_offset` clamps an offset to the UTF-16 document length, walks Unicode
  scalar values, and floors a position inside a scalar's UTF-16 encoding to that scalar's byte
  start.
- `Converter::utf8_to_16_offset` similarly clamps a byte position and floors a position inside a
  scalar's UTF-8 encoding before counting UTF-16 code units.

Both loops accumulate positions one Unicode scalar at a time. If a requested position falls inside
the current scalar encoding, they return the position accumulated before that scalar rather than
slicing through it. The generic converters are total, while exported methods compose them through
different helpers:

- `help` calls `Converter::utf16_to_8_offset` directly.
- `format` and `apply_edits` route cursors through `utf16_to_8_cursor` before entering IDE.
- `apply_edits` routes DTO ranges through `utf16_to_8_text_edits`, which converts each endpoint and
  produces Analyzer byte edits before IDE validates the batch.

Refer to the WASM specification for the caller-visible treatment of past-end, reversed, and
scalar-interior positions.

This composition means an endpoint inside a surrogate pair can collapse a range to an empty byte
range or change how converted ranges relate to one another. The core edit validator sees only the
resulting byte ranges. Unit tests beside `offsets.rs` anchor the flooring and out-of-bounds paths;
the Current WASM integration suite exercises emoji conversion only at valid scalar boundaries, so a
change to checked scalar-interior handling should add a direct integration case.

[`byte_span_to_utf16_span`](../../../analyzer_wasm/src/span.rs) converts output span endpoints
separately through the reverse converter. Most Analyzer and IDE spans already lie on valid byte
boundaries; flooring keeps the boundary adapter panic-free if an internal endpoint does not.

The source string used for conversion must match the coordinate's lifecycle:

- diagnostics, tokens, code actions, replacement spans, and completion edit ranges are converted
  against the original source;
- `format` and `apply_edits` cursors are converted against the updated source returned by IDE;
- a completion item's requested cursor belongs to the hypothetical document after applying that
  item's edits, so the completion converter first constructs that updated text before converting
  the cursor.

## Converters adapt structure without adding semantics

`Converter` is a zero-sized namespace split across
[`converter/analyze.rs`](../../../analyzer_wasm/src/converter/analyze.rs),
[`converter/completion.rs`](../../../analyzer_wasm/src/converter/completion.rs), and
[`converter/shared.rs`](../../../analyzer_wasm/src/converter/shared.rs). Each adapter accepts a core
result and builds the corresponding DTO field by field.

The analyze adapter creates one `SourceMap`, converts each diagnostic span and attached code-action
edit, filters trivia tokens, and maps remaining token kinds to explicit DTO strings. Diagnostic
`line` and `col` come from `SourceMap::line_col` at the original byte-span start; they are display
coordinates counted by Analyzer, not another UTF-16 offset conversion. The inferred root type is
rendered with its Rust display implementation.

The help adapter preserves IDE item order, preferred indices, signature indices, and structured
display segments. It converts primary and additional edit ranges against the original source. When
an item has a primary edit, `completion_item_view` determines a byte cursor in the edited document,
accounts for valid additional edits before the primary edit, applies the sorted edits to a temporary
source, clamps the requested byte cursor to that result, and only then converts it to UTF-16. It does
not reproduce completion ranking or active-parameter logic.

Enum conversions such as `token_kind_string`, `completion_kind_view`, and
`display_segment_view` use exhaustive Rust matches. Adding an internal enum variant therefore
creates a compiler-visible conversion obligation instead of silently emitting a fallback value.

## DTO types drive serialization and TypeScript declarations

[`dto::v1`](../../../analyzer_wasm/src/dto/v1.rs) isolates the wire-shaped Rust types from Analyzer
and IDE structures. Input types derive `Deserialize`, output types derive `Serialize`, and types
shared with TypeScript derive `ts_rs::TS`. Serde attributes define wire details such as the
`Property.type` rename, the tagged `DisplaySegment` representation, and rejection of unknown config
fields. The `v1` module is a source namespace; there is no runtime version negotiation in this
crate.

The DTO layer and `serde_wasm_bindgen` are separate seams:

- `from_value` is the constructor's typed-config deserialization gateway;
- `apply_edits` deserializes its edit array before coordinate conversion;
- `to_value` is the common output-serialization gateway;
- `operation_err` adapts an `IdeError` into a `js_sys::Error` for the methods that call it.

These helper placements identify where to debug conversion failures. The WASM specification owns
which failures are returned as data, which are thrown, their messages, and their observable
precedence.

## Type generation and WASM packaging are two build seams

The TypeScript DTO file committed to the repository and the executable WASM package come from
different tools.

[`export_ts`](../../../analyzer_wasm/src/bin/export_ts.rs) calls `TS::decl()` for an explicit ordered
list of `dto::v1` types, adds `export` where needed, and overwrites
[`wasm_dto.ts`](../../../examples/vite/src/analyzer/generated/wasm_dto.ts). The `just gen-ts` recipe
runs this binary. `ts-rs` reflects Rust field types, while Serde controls runtime acceptance; for
example, a Serde default does not by itself make a generated TypeScript property optional. Changes
to input DTOs must therefore review both deserialization attributes and the generated declaration.

Separately, [`analyzer_wasm/Cargo.toml`](../../../analyzer_wasm/Cargo.toml) builds the library as both
`cdylib` and `rlib`. The Vite `wasm:build` recipe invokes `wasm-pack build --target web` and writes
JavaScript glue plus the `.wasm` module into the example's ignored `src/pkg/` directory. Those
wasm-bindgen artifacts do not replace the checked-in `wasm_dto.ts`, and exporting DTO declarations
does not rebuild the WASM module.

There is currently no automated drift test that regenerates `wasm_dto.ts` and compares it with the
checked-in file. A DTO change should run `just gen-ts`, review the generated diff, then build the
WASM consumer separately when the binding surface changed.

## Tests isolate native conversion from the JavaScript boundary

Native tests exercise pure Rust pieces without a JavaScript runtime:

```bash
cargo test -p analyzer_wasm
```

The unit tests beside `offsets.rs` cover scalar-interior flooring and checked out-of-range
conversion, while
the analyze-converter test anchors multiline diagnostic locations. The `wasm_bindgen_test` cases in
[`analyzer_wasm/tests/analyze.rs`](../../../analyzer_wasm/tests/analyze.rs) compile natively but run
only through a WASM test runner:

```bash
wasm-pack test --node analyzer_wasm
```

Those integration tests cross the real `JsValue` boundary and cover config rejection, serialized
ASCII/Chinese/emoji spans, diagnostic actions, format errors, edit conversion and overlap errors,
and preference propagation. When a failure appears only in Node, start at `lib.rs` to identify the
method stage, then inspect `offsets.rs` for coordinate failures or the relevant converter for output
shape. Serialization shape changes should also inspect `dto/v1.rs` and the generated TypeScript
diff.
