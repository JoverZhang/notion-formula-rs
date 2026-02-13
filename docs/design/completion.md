# Completion and signature help (Rust core)

Editor-facing behavior implemented in `analyzer/src/ide/completion/`.

## Coordinates

- Core completion uses UTF-8 byte offsets (`Span`, `[start,end)`).
- WASM converts to UTF-16 DTO spans.
- Code: `analyzer/src/ide/completion/*`, `analyzer_wasm/src/*`

## Entry point + outputs

- `completion::complete(text, cursor_byte, ctx, config) -> CompletionOutput`
  - Code: `analyzer/src/ide/completion/mod.rs`
- `ide::help(source, cursor_byte, ctx, config) -> HelpResult`
  - Code: `analyzer/src/ide/mod.rs`
- `CompletionOutput`:
  - `items: Vec<CompletionItem>`
  - `replace: Span` (byte offsets in the original doc)
  - `signature_help: Option<SignatureHelp>`
  - `preferred_indices: Vec<usize>`
- `HelpResult`:
  - `completion: CompletionResult { items, replace, preferred_indices }`
  - `signature_help: Option<SignatureHelp>`

Edit/cursor rule:

- If `CompletionItem.cursor` is set, it is a byte offset in the updated document after applying the
  primary edit.
  - Code: `analyzer/src/ide/completion/mod.rs` (`CompletionItem`)

## Completion item kinds

- Function kinds:
  - `FunctionGeneral`
  - `FunctionText`
  - `FunctionNumber`
  - `FunctionDate`
  - `FunctionPeople`
  - `FunctionList`
  - `FunctionSpecial`
- Function item labels render as `name()` (call shape); `insert_text` is also `name()`.
- Other kinds: `Builtin`, `Property`, `Operator`
- Builtin items include reserved keywords: `true`, `false`, `not`
  - Code: `analyzer/src/ide/completion/items.rs`

Property items:

- Insert text: `prop("Name")`.
- Properties can be disabled via `Property.disabled_reason`.
  - Disabled completion items have no `primary_edit` and no `cursor`.
  - Code: `analyzer/src/analysis/mod.rs` (`Property`), `analyzer/src/ide/completion/items.rs`,
    `analyzer/src/ide/completion/rank.rs` (`attach_primary_edits`)

## Postfix completion (after atom / after dot)

- Candidate set starts from `semantic::postfix_capable_builtin_names()`.
  - Code: `analyzer/src/analysis/mod.rs`, `analyzer/src/ide/completion/items.rs`
- After `.` (member-access mode), candidates are additionally filtered by receiver type:
  - infer receiver type best-effort from source prefix before `.`
  - keep methods whose postfix first parameter accepts the receiver via `semantic::ty_accepts`
  - for `receiver = Unknown`, current behavior keeps all postfix-capable methods (TODO: narrow to
    explicit `any`-accepting signatures once `any` exists in the type model)
  - Code: `analyzer/src/ide/completion/pipeline.rs`, `analyzer/src/ide/completion/items.rs`
- UI forms:
  - Labels render as `.name()` (method-call shape).
  - After an atom: inserts `.name()`.
  - After `.`: inserts `name()` (the dot already exists in source).
  - Postfix items use function kinds (`Function*`, from builtin `FunctionCategory`) so UI grouping
    keeps them in function sections instead of `Operator`.
  - `detail` is method-style (`(receiverParam).name(otherParams)`), including repeat-shape `...`
    where applicable.
  - Code: `analyzer/src/ide/completion/items.rs` (`postfix_method_items`)

## Replace span and “prefix editing”

Position and replace-span logic is in:

- `analyzer/src/ide/completion/position.rs`

Key rules:

- `AfterDot` is determined by token connectivity (`receiver_atom` + `.` + optional method prefix),
  not by fuzzy/prefix ranking.
- Strictly inside an identifier → replace the identifier span.
- At an identifier boundary (cursor at end of identifier token) → replace the identifier span only
  if the prefix can be extended by something in-scope (builtins/functions/properties).
- Otherwise expression-start completion is insertion-only (empty replace span).

## Query ranking

Ranking runs in `analyzer/src/ide/completion/rank.rs`.

Query derivation:

- If `replace` is empty → no query ranking; `preferred_indices = []`.
- Otherwise:
  - If the replaced substring contains any non-identifier characters
    (allowed set = ASCII alnum + `_` + whitespace) → no query ranking; `preferred_indices = []`.
  - Normalize by lowercasing and removing whitespace and `_`.
  - If normalized query is empty → no query ranking; `preferred_indices = []`.

Ranking behavior:

- Normal (expr-start) mode:
  - query ranking applies to function kinds and `Property` only
  - other kinds keep their relative order
- Postfix (after-dot) mode:
  - label matching ignores the leading `.`
  - non-matching items are filtered out

Match classes (in order):

1) exact match
2) substring contains (earlier occurrence wins)
3) fuzzy subsequence match

Tie-breaking:

- Exact: shorter normalized label first, then original index.
- Contains: earlier match position, then shorter normalized label, then original index.
- Fuzzy: fuzzy score, then kind priority, then original index.

## Type ranking

Type ranking is a separate pass (`apply_type_ranking`) used when an expected type is available:

- Skipped when the expected type is wildcard-ish:
  - `Unknown` and `Generic(_)` are treated as “no signal” and do not produce type ranking.
- Groups items into completion-kind buckets, scores each item, sorts within buckets, then reorders
  buckets by best score (ties broken by bucket priority).
- Code: `analyzer/src/ide/completion/rank.rs` (`apply_type_ranking`)

## preferred_indices

- `preferred_indices` are “smart picks” for UI defaults.
- Computed from ranked items that match the query, up to `preferred_limit`.
- `preferred_limit` defaults to `5`.
- `context_json.completion.preferred_limit` overrides; `0` disables preferred computation.
- Code: `analyzer/src/ide/completion/rank.rs`

## Signature help

Signature help is returned from completion when the cursor is inside a call.

- Detection + rendering: `analyzer/src/ide/completion/signature.rs`
- Uses semantic instantiation (`instantiate_sig`):
  - Code: `analyzer/src/analysis/infer.rs`

Instantiation model (current behavior):

- Signature help is call-site instantiated:
  - best-effort infers argument expression types from the source
  - instantiates the `FunctionSig` using the same unification/substitution logic as semantic inference
  - type strings are formatted via `analyzer/src/ide/display.rs` (`format_ty(...)`)
  - instantiated `Unknown` renders as `unknown` (including unconstrained generics)
  - parameter slots prefer per-argument inferred (actual) types when the argument expression is non-empty
    and the inferred type is helpful/compatible; empty argument slots fall back to instantiated expected types

Output model:

- Structured segments (`DisplaySegment`) suitable for direct UI rendering.
  - Punctuation and separators are separate segments.
  - `DisplaySegment::Param` carries `param_index` for highlight mapping.
  - Code: `analyzer/src/ide/display.rs`
- `active_signature` is currently `0`.
- `active_parameter` is computed from call-site arg index and `ParamShape`.

Type string rendering:

- Types are rendered via `Ty`’s `Display` impl (`number`, `string`, `unknown`, `T0`, `(A | B)[]`,
  ...).
  - Code: `analyzer/src/analysis/mod.rs` (`impl Display for Ty`)

Postfix calls:

- `receiver.fn(...)` can render as method-style when the callee is postfix-capable.
- Receiver segments are a prefix `(<receiver_param>).` and are never highlighted.

Repeat-group rendering:

- Repeat params are numbered by group (`condition1/value1`, `condition2/value2`, ...).
- `...` is inserted between the last repeat group and the tail params.

Spec:
- `docs/signature-help.md`
