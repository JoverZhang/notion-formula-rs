# Builtin Functions Design

This document defines the builtin-function declaration syntax, call-site signature
resolution, catalog and documentation generation, and the compile-time and runtime
contracts that the evaluator must implement.

See [`builtin_fn/README.md`](../../builtin_fn/README.md) for the module implementation
entry point and [`contracts.md`](contracts.md) for shared cross-crate constraints.

## Goals

The builtin-functions module lets authors declare each function once and derives the
semantic behavior and presentation data required throughout the formula pipeline.

A category-level declaration must produce:

1. a validated builtin registry entry;
2. a parameter shape for arity validation and argument-to-parameter mapping;
3. generic-binding rules and the call-site return type;
4. a custom resolver for specialized type inference;
5. canonical completion detail and Signature Help metadata;
6. catalog data that can be checked against the builtin documentation;
7. a per-function trait, marker, and typed dispatch binding that the evaluator must
   implement; and
8. the dependency manifest and synchronous evaluation contract callers use to prepare
   input columns.

The core flow is:

```text
category declaration
        |
        v
signature template + parameter shape + presentation metadata
        |
        +-------------------+
        | call arguments    |
        v                   |
shape resolution -> generic binding / custom resolver
        |
        v
resolved parameters + return type
        |
        +--> semantic validation and expression type inference
        +--> completion and Signature Help
        +--> catalog and README renderer
        +--> evaluator trait / typed dispatch
                         |
                         v
              prepared columns -> synchronous row-batch evaluation
```

`builtin_fn` provides a shared, AST-independent interface for call-site signature
resolution. The Analyzer and IDE supply call observations and consume the same resolved
signature rules; the evaluator build step generates implementation contracts from the
same declarations.

## Authoring Interface

### One declaration block per category

All production category declarations live together in `builtin_fn/src/builtins.rs`. Each
category has one invocation of a function-like procedural macro. The macro expands to a
structured `BuiltinCategory` expression; an ordinary surrounding Rust function owns the
item name and visibility.

```rust,ignore
fn text_definitions() -> BuiltinCategory {
    builtin_functions! {
        category: Text;

        substring(
            text: string,
            start: number,
            end?: number,
        ) -> string;

        #[unsupported]
        /// `StyledText` is not represented yet.
        style(
            text: string,
            repeat(min = 1) {
                styles: string,
            },
        ) -> StyledText;
    }
}
```

The expression follows these contracts:

- every declaration receives `FunctionCategory::Text`;
- the returned category preserves declaration order;
- any invalid declaration fails the entire macro expansion;
- the macro does not generate a module, surrounding function, static, or visibility; and
- registry and structural tests can call the category function directly.

### Category output

```rust,ignore
pub struct BuiltinCategory {
    pub category: FunctionCategory,
    pub entries: Vec<BuiltinCatalogEntry>,
}

pub struct BuiltinCatalogEntry {
    pub name: String,
    /// Canonically rendered from the parsed declaration.
    pub signature: String,
    pub detail: String,
    pub docs: Vec<String>,
    /// `None` only for `#[unsupported]` declarations.
    pub implementation: Option<FunctionSig>,
}

impl BuiltinCategory {
    pub fn into_functions(self) -> impl Iterator<Item = FunctionSig>;
}
```

The macro lowers every parsed declaration to a `BuiltinCatalogEntry`. Supported
declarations also lower to `FunctionSig`; unsupported declarations retain
`implementation = None`.

This model does not store Markdown. Catalog metadata remains independent of a specific
presentation format.

The top-level registry continues to use ordinary Rust composition:

```rust,ignore
pub fn builtin_categories() -> Vec<BuiltinCategory> {
    vec![
        general_definitions(),
        text_definitions(),
        math_definitions(),
        date_definitions(),
        people_definitions(),
        list_definitions(),
        special_definitions(),
    ]
}

pub fn builtins_functions() -> Vec<FunctionSig> {
    builtin_categories()
        .into_iter()
        .flat_map(BuiltinCategory::into_functions)
        .collect()
}
```

### Signature syntax

The following EBNF is the normative syntax for category declarations:

```text
CategoryBlock  := CategoryHeader FunctionDecl*
CategoryHeader := "category" ":" Name ";"
FunctionDecl   := Attribute* Name GenericParams? "(" ParamList? ")" "->" Type ";"
GenericParams  := "<" GenericParam ("," GenericParam)* ">"
GenericParam   := Name (":" GenericKind)?
GenericKind    := "Plain" | "Variant"
ParamList      := ParamItem ("," ParamItem)* ","?
ParamItem      := Param | RepeatBlock
RepeatBlock    := "repeat" "(" "min" "=" Integer ")"
                  "{" Param ("," Param)* ","? "}"
Integer        := ASCII decimal integer literal
Param          := Name "?"? ":" Type
Attribute      := "#[resolver(" RustPath ")]"
                | "#[unsupported]"
                | "///" DocText
Type           := PostfixType ("|" PostfixType)*
PostfixType    := PrimaryType "[]"*
PrimaryType    := Primitive
                | GenericName
                | "(" Type ")"
                | "(" LambdaParams? ")" "->" Type
                | "Ident" "<" Type ">"
LambdaParams   := LambdaParam ("," LambdaParam)*
LambdaParam    := Name ":" Type
Primitive      := "number" | "string" | "boolean" | "date" | "null" | "any"
```

The syntax must satisfy these rules:

- generic declarations are ordered, and every generic reference must refer to an earlier
  declaration;
- omitting the generic kind means `Plain`;
- `<T>` and `<T: Plain>` lower to the same `GenericParam`;
- `Variant` must be written explicitly;
- generic IDs are assigned deterministically in declaration order;
- an unknown generic kind is a compile-time declaration error;
- optional parameters use `name?: Type`;
- a function may contain at most one `repeat` block;
- parameters before `repeat` automatically form the fixed `head`;
- parameters inside `repeat` form one repeating group;
- parameters after `repeat` automatically form the fixed `tail`;
- repeat members use logical base names without `1`, `2`, or `N` suffixes;
- resolver metadata attaches to the function declaration; and
- the system generates detail strings, which ordinary declarations cannot override.

## End-to-End Examples

The following examples show what authors write and what the rest of the project derives.

### Fixed signature: `substring`

Declaration:

```rust,ignore
substring(
    text: string,
    start: number,
    end?: number,
) -> string;
```

Generated template:

```text
name       = substring
params     = head[text: string, start: number, end?: number]
repeat     = []
tail       = []
return     = string
detail     = substring(text, start, end?)
```

Derived behavior:

- accepts two or three arguments;
- checks argument types against fixed parameter slots;
- returns `string` for every valid call; and
- shares generated names and optional markers between completion and Signature Help.

### Homogeneous repetition: `concat`

Declaration:

```rust,ignore
concat<T>(
    repeat(min = 2) {
        lists: T[],
    },
) -> T[];
```

Generated template:

```text
generics   = [T: Plain]              // generated from `<T>`
params     = head[]
repeat     = [lists: T[]]
tail       = []
min groups = 2
return     = T[]
detail     = concat(lists1, lists2, ...)
```

Call-site resolution example:

```text
call       = concat([1], ["x"])
bindings   = T := number | string
return     = (number | string)[]
signature  = concat(lists1: number[], lists2: string[], ...)
             -> (number | string)[]
```

The declaration has one logical parameter name. Numeric suffixes in presentation names
are generated from the number of repeating groups present at the call site.

### Repeating tuple with an automatic tail: `ifs`

Declaration:

```rust,ignore
ifs<T: Variant>(
    repeat(min = 1) {
        condition: boolean,
        value: () -> T,
    },
    else: () -> T,
) -> T;
```

Generated template:

```text
generics   = [T: Variant]
params     = head[]
repeat     = [condition: boolean, value: () -> T]
tail       = [else: () -> T]
min groups = 1
return     = T
detail     = ifs(condition1, value1, condition2, value2, ..., else)
```

Call-site resolution example:

```text
call       = ifs(true, 1, false, "two", 0)
shape      = repeat #1, repeat #2, tail
bindings   = T := number | string
return     = number | string
signature  = ifs(condition1: boolean, value1: number,
                 condition2: boolean, value2: string, ...,
                 else: number) -> number | string
```

Ordinary parameters after `repeat` are automatically recognized as the `tail`. The lambda
wrapper belongs to the semantic model; users still write branch expressions directly.

### Custom type resolution: `flat`

Declaration:

```rust,ignore
#[resolver(resolve_flat)]
flat<T>(list: T[]) -> T[];
```

Generated template:

```text
generics   = [T: Plain]              // generated from `<T>`
params     = head[list: T[]]
return     = T[]
resolver   = resolve_flat
detail     = flat(list)
```

Call-site resolution example:

```text
call       = flat([1, ["two"]])
resolver   = collect non-list leaf types and normalize them into a union
return     = (number | string)[]
```

`Flat` is not a generic binding kind. `Plain` and `Variant` describe how generics bind;
when ordinary generic substitution cannot express the return type, a function-level
resolver supplies the additional behavior.

## Generated Semantic Model

Declarations must contain enough information to derive the following model:

| Model | Purpose |
| --- | --- |
| `FunctionCategory` | Registry grouping and completion classification |
| `FunctionSig` | Reusable signature template |
| `ParamShape` | Fixed head, repeating group, fixed tail, and minimum group count |
| `ParamSig` | Logical name, type template, and optionality |
| `GenericParam` | Stable generic identity and binding kind |
| `SigResolver` | Pure custom return-type refinement hook |
| Canonical detail | Stable completion presentation |

The macro interface does not expose the Rust construction details of these types.
Parsing, validation, ID generation, and metadata normalization are encapsulated behind
the declaration interface.

## Call-Site Signature Resolution

### Resolution interface

```rust,ignore
let resolved = resolve_call_signature(
    &signature,
    CallSignatureInput {
        arguments: &[
            ArgumentObservation::Typed(Ty::List(Box::new(Ty::Number))),
            ArgumentObservation::Empty,
        ],
    },
);
```

Resolution is a deterministic pure function. The end of the slice means an argument has
not appeared yet; `Empty` means a syntactic argument slot exists but has no content;
`Typed(Ty::Unknown)` means an expression exists at that position but its type could not be
inferred.

For calls with lambdas, the Analyzer resolves immutable snapshots in phases:

1. infer ordinary arguments;
2. resolve a partial snapshot to obtain instantiated lambda parameter types;
3. infer the lambda body; and
4. resolve the complete snapshot to obtain the final signature.

The IDE can resolve any partial snapshot directly. Resolving again after an edit does not
reuse hidden state.

### Input and output records

```rust,ignore
pub struct CallSignatureInput<'a> {
    /// Semantic argument order; a postfix receiver is already in the first position.
    pub arguments: &'a [ArgumentObservation],
}

pub enum ArgumentObservation {
    Empty,
    Typed(Ty),
}

pub struct ResolvedFunctionSig {
    /// Whether the current argument count is an exact shape match.
    pub validity: ShapeValidity,

    /// Exact parameter slots when valid; otherwise the minimum completable shape.
    pub projection: Vec<ResolvedParamSlot>,

    /// One entry per observed argument, including excess unmapped arguments.
    pub arguments: Vec<ResolvedArgument>,

    pub return_ty: Ty,
}

pub enum ShapeValidity {
    Valid,
    Invalid(CallShapeError),
}

pub enum ParamRef {
    Head(usize),
    Repeat(usize),
    Tail(usize),
}

pub struct ResolvedParamSlot {
    pub logical_param: ParamRef,
    /// Repeat slots are numbered from 1; fixed head/tail slots use `None`.
    pub repeat_group: Option<usize>,
    pub argument_index: Option<usize>,
    pub expected_ty: Ty,
}

pub struct ResolvedArgument {
    pub parameter: Option<ParamRef>,
    pub repeat_group: Option<usize>,
    pub expected_ty: Option<Ty>,
    pub type_status: ArgumentTypeStatus,
}

pub enum ArgumentTypeStatus {
    Compatible,
    Mismatch { actual: Ty },
    /// The argument slot is empty or its inferred type is unknown.
    Indeterminate,
    /// The call shape is invalid, so this argument has no corresponding parameter.
    Unmapped,
}
```

`projection` stores semantic parameter slots rather than formatted text. Repeat-group
numbers and logical parameter references remain structured; presentation code uses them
to generate labels such as `condition2`.

Every consumer uses the same result:

- the Analyzer reads `validity`, each argument's `type_status`, and `return_ty`;
- the IDE reads `projection`, the original observations, and `return_ty`; and
- invalid or incomplete calls still receive a deterministic projection.

The interface therefore supports:

- missing argument types in incomplete IDE input;
- strict and completable interpretations of repeat shapes;
- observing non-lambda arguments before the lambda body;
- querying instantiated expected argument types;
- ordinary generic substitution and custom resolvers; and
- obtaining resolved parameters, the return type, and shape status in one result.

Given a signature template and call-site observations, resolution conceptually:

1. checks whether the total argument count matches the fixed/repeat/tail shape;
2. maps every argument position to a logical parameter;
3. infers non-lambda argument types;
4. binds generics according to their declared kinds;
5. uses substitutions, when necessary, to determine implicit lambda parameter and body
   types;
6. calls the function-level resolver, if declared;
7. instantiates parameter and return types;
8. compares typed observations with instantiated expected types; and
9. provides the resolved result to semantic validation and IDE presentation.

### Generic binding kinds

- `Plain`: unknown observations do not bind; conflicting concrete types accumulate into a
  deterministic union.
- `Variant`: concrete branch types accumulate into a deterministic union; any unknown
  observation makes the result unknown.

`Plain` is the default:

```rust,ignore
map<T, U>(list: T[], mapper: (current: T) -> U) -> U[];
ifs<T: Variant>(...) -> T;
```

Explicit `<T: Plain>` remains valid but redundant. `Flat` is not allowed in this position.

### Repeat shape

`ParamShape { head, repeat, tail, repeat_min_groups }` is the canonical semantic shape.
The repeating group cannot be empty and cannot contain optional members.

#### Repeat cardinality

Every repeat block declares the minimum number of complete repeating groups:

```rust,ignore
// Zero or more single-parameter groups.
repeat(min = 0) {
    items: T,
}

// One or more two-parameter groups.
repeat(min = 1) {
    condition: boolean,
    value: () -> T,
}

// At least two single-parameter groups.
repeat(min = 2) {
    lists: T[],
}
```

`min` counts complete groups, not parameters. The `ifs` group above therefore contributes
at least two arguments, while `concat` contributes at least two list arguments.

Rules:

- `min` is required;
- it must be a non-negative integer literal;
- arbitrary Rust expressions and constants are not accepted;
- there is no separate `max`; all remaining repetitions are unbounded;
- `min` lowers directly to `ParamShape::repeat_min_groups`; and
- omitting `min` is a compile-time declaration error.

#### Head, repeat, and tail layouts

A declaration may contain at most one repeat block:

```rust,ignore
example<T>(
    prefix: string,          // head
    repeat(min = 1) {
        key: string,         // repeat group
        value: T,
    },
    fallback: T,             // tail
) -> T;
```

Position alone determines the semantic shape:

- ordinary parameters before `repeat` become the `head`;
- parameters inside `repeat` become one repeating group;
- ordinary parameters after `repeat` become the `tail`; and
- declarations do not write separate `head`, `repeat`, or `tail` keywords.

All combinations expressible by `ParamShape` are supported:

| Declaration layout | Example |
| --- | --- |
| Fixed only | `substring` |
| Repeat only | `concat` |
| head + repeat | `splice` |
| repeat + tail | `ifs` |
| head + repeat + tail | Supported when needed |

For signatures containing repeat, a valid argument count satisfies:

```text
total = head.len + repeat.len * groups + tail.len
groups >= repeat.min
```

Shape mapping uses parameter positions only and never consults parameter types. To keep
the mapping deterministic:

- a repeat block cannot be empty;
- each declaration has at most one repeat block;
- repeat members cannot be optional;
- fixed head and tail parameters cannot be optional when repeat is present;
- without repeat, optional fixed parameters must form one contiguous suffix; and
- a required fixed parameter after an optional parameter is rejected.

### Custom resolvers

A resolver is a pure return-type refinement function:

```rust,ignore
pub struct ResolverInput<'a> {
    pub arguments: &'a [ArgumentObservation],
    pub default_return_ty: &'a Ty,
}

pub type SigResolver = for<'a> fn(&ResolverInput<'a>) -> Ty;
```

`#[resolver(...)]` accepts a Rust function path. The referenced function may be private to
the declaration module; the generated Rust expression type-checks it. Production resolvers
live alongside the category functions in `builtin_fn/src/builtins.rs`.

Resolution order:

1. resolve the shape and bind ordinary generics;
2. instantiate the declared return type;
3. call the resolver, if present; and
4. store the result in `ResolvedFunctionSig::return_ty`.

A resolver cannot change:

- shape validity or the completable projection;
- argument-to-parameter mapping;
- expected argument types;
- the function name, category, detail, or generic declarations.

Both complete and partial snapshots invoke the resolver. For `Empty` or
`Typed(Ty::Unknown)`, the resolver must return a best-effort type, normally falling back
to `default_return_ty`.

Because resolvers consume immutable observations, they can coexist with signatures that
contain lambdas. Partial resolution may produce only an imprecise type; a second snapshot
after lambda-body inference can refine it further.

`flat` is defined as:

```rust,ignore
fn resolve_flat(input: &ResolverInput<'_>) -> Ty {
    match input.arguments.first() {
        Some(ArgumentObservation::Typed(Ty::List(element))) => {
            Ty::List(Box::new(flatten_leaf_types(element)))
        }
        _ => input.default_return_ty.clone(),
    }
}
```

`flatten_leaf_types` recursively walks lists and unions and normalizes every non-list leaf
type into a deterministic union.

## Canonical Presentation Metadata

Presentation metadata is derived entirely from declarations. There is no
`#[detail(...)]` override.

### Parameter names

- fixed head and tail parameters retain their declared names;
- repeat members use unnumbered logical base names;
- rendering appends the one-based group number directly to the base name;
- `condition` in group 2 therefore renders as `condition2`;
- `...` is a separate presentation slot after all preview groups; and
- tail parameters render after `...` and are never numbered.

Repeat-member names ending in an ASCII digit or using a legacy `N` suffix are rejected.
The macro does not strip or interpret numbering written by the author.

### Static completion detail

For declarations with repeat:

```text
preview_groups = max(repeat.min, 2)
```

Static detail renders in this order:

1. fixed head parameters;
2. `preview_groups` numbered repeating groups;
3. one separate `...`; and
4. fixed tail parameters.

Examples:

| Declaration | Canonical detail |
| --- | --- |
| `concat`, `min = 2` | `concat(lists1, lists2, ...)` |
| `ifs`, `min = 1` | `ifs(condition1, value1, condition2, value2, ..., else)` |
| `splice`, `min = 0` | `splice(list, startIndex, deleteCount, items1, items2, ...)` |
| `substring` | `substring(text, start, end?)` |

### Dynamic Signature Help

Dynamic rendering uses `ResolvedFunctionSig::projection`, not `preview_groups`:

- valid calls show the repeating groups actually present;
- incomplete calls show the minimum completable projection;
- group numbering always starts at 1;
- actual argument types may narrow individual presentation slots;
- the rendered return type is instantiated or resolver-refined; and
- `...` has no parameter index and is never the active parameter.

## Consumer-Visible Results

### Semantic analysis

- argument-count and invalid-shape diagnostics;
- expected-type validation for each argument;
- generic return-type inference;
- implicit lambda parameter and result-type inference; and
- custom return-type inference from resolver functions.

### Completion

- builtin names and categories;
- canonical call detail; and
- postfix eligibility derived from the first parameter when applicable.

### Signature Help

- call-site-instantiated parameters and return type;
- repeat names generated from the number of groups present;
- stable active-parameter mapping in incomplete repeating groups;
- fixed-tail presentation after `...`; and
- postfix presentation without duplicating the semantic signature.

### Documentation and catalog checks

- stable category and function order;
- declared names and canonical detail;
- the generated catalog region in `docs/builtin_functions/README.md`; and
- deterministic drift failures in CI.

Category declarations are the sole source of truth for the function inventory, signature
syntax, ordering, and inclusion in the executable registry. Explanatory README prose
remains hand-maintained; the function catalog region is generated from declaration
metadata.

### Evaluator

- every supported builtin has a required trait, marker, and dispatch binding;
- declared types, optional parameters, repeat, tail, and lambda shapes lower to named,
  typed Args/Plans;
- the Planner reuses the resolved signature stored by the Analyzer and does not infer
  generics again from batch data;
- `PreparedFormula` exposes the property-column manifest required by the formula;
- after the caller prepares complete columns, the evaluator executes the row batch
  synchronously; and
- execution masks, row errors, and null validity remain independent.

### Unsupported catalog entries

Functions not yet supported by the target model are still written as real declarations:

```rust,ignore
#[unsupported]
/// The semantic type model does not yet represent `DateRange`.
dateRange(start: date, end: date) -> DateRange;

#[unsupported]
/// Currently expressed by the `&&` operator rather than a builtin call.
and(
    repeat(min = 2) {
        condition: boolean,
    },
) -> boolean;
```

`#[unsupported]` deliberately remains binary instead of introducing a blocker-kind enum.
One or more `///` lines provide the human-readable reason.

For these entries, the macro:

- parses and validates declaration syntax;
- includes the entry in category ordering and duplicate-name checks;
- derives canonical parameter names and documentation detail;
- generates the catalog metadata required by the README;
- does not generate a runtime `FunctionSig`;
- permits named documentation-only types such as `DateRange`, `Link`, and `StyledText`
  without requiring matching `Ty` variants;
- still rejects unknown types in supported declarations; and
- rejects using `#[resolver(...)]` together with `#[unsupported]`.

Generated documentation renders both the reason and signature:

```rust
// Unsupported: the semantic type model does not yet represent `DateRange`.
dateRange(start: date, end: date) -> DateRange
```

When making an entry supported, remove `#[unsupported]` and the obsolete explanation; the
declaration must then lower successfully to `FunctionSig`.

### README generation

An independent renderer consumes `builtin_categories()`:

```rust,ignore
let markdown = render_builtin_catalog(&builtin_categories());
```

The renderer replaces only the marked generated region in
`docs/builtin_functions/README.md`; surrounding explanatory prose remains hand-maintained.
It requires exactly one begin marker followed by exactly one end marker. CI renders the
same region in memory and compares it byte-for-byte with the committed file. The procedural
macro itself never reads or writes repository files.

## Module Interfaces and Ownership

The builtin-functions design provides three interfaces:

1. author declaration -> validated signature template;
2. signature template + call observations -> resolved call signature; and
3. catalog -> evaluator implementation contracts, typed dispatch, and input dependency
   descriptions.

`builtin_fn` does not perform AST traversal, expression inference, or row-batch
computation. The Analyzer performs the first two and supplies type observations to the
second interface; the evaluator implements the contracts generated by the third. The IDE
uses the same resolution interface for partial calls, preventing generic binding, repeat
shape, and resolver rules from silently diverging between consumers.

## Evaluator Implementation Contract

Every supported builtin declaration generates a compile-time contract that the evaluator
implements:

```text
category proc macro
        |
        v
BuiltinCategory / FunctionSig
        |
        v
evaluator/build.rs
        |
        v
$OUT_DIR/builtin_contract.rs
        |
        +--> builtin identifier
        +--> implementation trait
        +--> typed dispatch binding
                         |
                         v
              handwritten evaluator implementation
```

### Generation boundary

`evaluator` declares `builtin_fn` as a build dependency. Its build script calls
`builtin_categories()`, filters `#[unsupported]` entries, and emits evaluator-side contracts
into `OUT_DIR`:

```rust,ignore
mod contract {
    include!(concat!(env!("OUT_DIR"), "/builtin_contract.rs"));
}
```

The category procedural macro generates evaluator-independent catalog and signature data;
the evaluator build step then converts it into declarations involving evaluator runtime
types. Therefore, `builtin_fn` never depends on `evaluator`.

Generating trait declarations alone is not enough to require implementations. Each
generated dispatch binding must also reference its corresponding handwritten
implementation. Therefore:

- a missing implementation target is a compile error;
- a missing trait impl is a compile error;
- incorrect method parameters or return types are compile errors;
- every supported declaration has a dispatch binding; and
- unsupported declarations create no evaluator implementation obligation.

`cargo check -p evaluator` automatically regenerates and compiles the contracts. Output must
be deterministic and is not committed to the source repository. Trait granularity and
signature-to-runtime-type mapping are defined separately below.

### Trait granularity

Every supported builtin generates:

1. an implementation trait;
2. a marker type that must implement the trait; and
3. a dispatch binding that calls the marker through the trait.

```rust,ignore
// Generated contract; the Runtime ABI mapping below determines parameter types.
pub(crate) trait FlatKernel {
    fn eval(args: FlatArgs, mask: &Mask) -> KernelResult<ListKind>;
}

pub(crate) struct Flat;

fn dispatch_flat(args: FlatArgs, mask: &Mask) -> KernelResult<ListKind> {
    <Flat as FlatKernel>::eval(args, mask)
}
```

The evaluator provides only the implementation:

```rust,ignore
impl FlatKernel for Flat {
    fn eval(args: FlatArgs, mask: &Mask) -> KernelResult<ListKind> {
        // Handwritten behavior.
    }
}
```

The generated marker fixes the required implementation target. Rust coherence guarantees
that at most one `impl FlatKernel for Flat` exists; the generated dispatch binding makes a
missing implementation or incorrect signature fail at compile time.

Rust modules may organize handwritten impl blocks independently from the declaration file;
they do not form contract boundaries. Unsupported declarations do not generate evaluator
traits, markers, or dispatch bindings.

### Debug runtime assertions

Declaration validation, Rust type checking, and runtime-value checks protect different
boundaries:

1. the category macro validates the signature model;
2. generated traits validate the handwritten evaluator ABI; and
3. debug runtime assertions validate prepared input and output values.

Semantic predicates are owned by `builtin_fn` and shared by the Analyzer and evaluator:

```rust,ignore
pub fn type_accepts(expected: &Ty, actual: &Ty) -> bool;

pub fn check_argument_type(
    expected: &Ty,
    observation: &ArgumentObservation,
) -> ArgumentTypeStatus;
```

The Analyzer supplies inferred `Ty` observations. Call resolution records the result in
each `ResolvedArgument::type_status`, so semantic diagnostics no longer implement separate
compatibility rules for unions, lists, generics, or lambdas.

The evaluator adapts its runtime representation to the shared type model:

```rust,ignore
#[cfg(debug_assertions)]
fn runtime_ty(value: &Value) -> Ty;
```

The adapter recursively represents lists and heterogeneous list members. The evaluator
can then call `type_accepts` or `check_argument_type` directly without exposing `Value`,
`Column`, or other evaluator-internal types to `builtin_fn`.

Every generated marker also implements a shared metadata interface:

```rust,ignore
pub(crate) trait BuiltinContract {
    const KEY: BuiltinKey;
    fn signature() -> &'static FunctionSig;
}
```

Generated dispatch bindings automatically assert prepared inputs before calling the
handwritten implementation and assert successful outputs after it returns. Implementations
may use the same helpers and signature metadata to check intermediate values.

Runtime assertions use instantiated expected types from the resolved call contract, not
unresolved generics from the declaration template. They check only active, successful,
non-null rows. Expected data errors must first become ordinary evaluator errors; rows
already marked as failed skip assertions.

Assertion failures report the builtin, parameter or return position, row, expected type,
and observed runtime type. Recursive checking and formatting compile only under
`debug_assertions`; these checks do not replace release-mode error handling or safety
checks.

### Resolved-contract handoff

The Analyzer stores the final resolved signature for every builtin call:

```rust,ignore
pub struct SemanticMap {
    pub expression_types: TypeMap,
    pub builtin_calls: HashMap<ExprId, ResolvedFunctionSig>,
}
```

For calls with lambdas, only the final snapshot after lambda-body inference is stored;
partial snapshots are temporary inference results.

The Analyzer consumes the result directly:

- `ShapeValidity` determines argument-count and repeat-shape diagnostics;
- `ArgumentTypeStatus` determines argument type-mismatch diagnostics; and
- `return_ty` becomes the call expression type.

The evaluator Planner consumes the same entry instead of repeating generic binding,
resolver execution, parameter-shape resolution, or type-compatibility checks. Only calls
with `ShapeValidity::Valid` may lower to executable builtin IR.

In debug builds, the Planner derives the smaller contract required at runtime:

```rust,ignore
#[cfg(debug_assertions)]
pub struct DebugCallContract {
    pub arguments: Box<[DebugArgumentContract]>,
    pub return_ty: Ty,
}

#[cfg(debug_assertions)]
pub struct DebugArgumentContract {
    pub parameter: ParamRef,
    pub repeat_group: Option<usize>,
    pub expected_ty: Ty,
}
```

Builtin-call IR nodes carry this contract only when `debug_assertions` are enabled. The
runtime-value adapter compares each active, successful, non-null row with the instantiated
parameter and return types.

`Column::F64` is known to represent `Ty::Number` without per-value checks. Dynamic columns
convert row by row; lists recursively normalize observed member types. Empty lists provide
no contrary type evidence and therefore use an unknown element type.

The evaluator never resolves generics again from batch contents and never invokes custom
signature resolvers at runtime. Release builds store no compact contract and contain no
recursive runtime type observation.

### Evaluation modes

The generator selects the evaluator contract automatically from semantic parameter types:

```rust,ignore
pub enum BuiltinEvaluationMode {
    Value,
    Controlled,
}
```

- when all parameters are ordinary value types, generate a `Value` contract;
- when any top-level parameter is `Ty::Fn` or `Ty::Ident`, generate a `Controlled`
  contract; and
- the declaration DSL requires no evaluator-specific attribute.

#### Value functions

The evaluator executes every argument plan before dispatch. The generated wrapper converts
the results into typed arguments and then calls the handwritten implementation.

`flat`, `concat`, `splice`, ordinary text functions, math functions, and date functions
belong to this mode.

#### Controlled functions

At dispatch, the complete structured argument set remains unevaluated. The generated trait
receives evaluator-owned plan handles and a restricted `BuiltinEvalContext`; the function
implementation decides evaluation order and the row mask used at each step.

Delaying only `Fn` parameters is insufficient. For example, later `condition` parameters
of `ifs` are declared as `boolean` but may be evaluated only for rows not matched by an
earlier branch. Therefore, once a call enters `Controlled` mode, all its arguments remain
as plans.

| Function | Why evaluation must be controlled |
| --- | --- |
| `if` | Execute only the selected branch |
| `ifs` | Evaluate conditions in order and shrink the remaining mask |
| `let` | Establish a binder before executing the body |
| `map` | Bind `current` for each element and execute the lambda |
| `filter` | Execute a predicate for each element |
| `find`, `some`, `every` | Predicate execution and early termination |

Every controlled function still generates its own trait, marker, and dispatch binding,
which the evaluator must genuinely implement. Planner rewrites do not bypass these
contracts, and the generator does not emit unreachable placeholder implementations.

`BuiltinEvalContext` does not expose the Analyzer AST. It provides only evaluator-owned
plan handles and a limited operation set:

```rust,ignore
context.eval(plan, mask);
context.eval_thunk(thunk, mask);
context.apply_lambda(lambda, bindings, mask);
context.split_mask(condition, mask);
```

Debug input checks for value functions happen at the dispatch boundary. Argument checks
for controlled functions happen when `context.eval*()` materializes a plan into actual
values. Final outputs from both modes are checked at the dispatch return boundary.

### Runtime ABI type mapping

Generated traits use typed owned columns rather than bare slices or uniform `Value`
arguments:

```rust,ignore
pub trait ColumnKind {
    type Scalar;
    type Storage: AsRef<[Self::Scalar]>;
}

pub struct SharedStorage<S> {
    inner: Arc<S>,
}

pub struct SharedBitmap {
    inner: Arc<BitMask>,
}

pub enum Validity {
    AllValid,
    AllNull,
    Bitmap(SharedBitmap), // 1 means this row contains a non-null value
}

pub struct KernelColumn<K: ColumnKind> {
    storage: SharedStorage<K::Storage>,
    validity: Validity,
}

pub struct KernelResult<K: ColumnKind> {
    column: KernelColumn<K>,
    ok: Mask,
    errors: Vec<(usize, EvalError)>,
}
```

Semantic types map as follows:

| `Ty` | ABI kind | Runtime storage |
| --- | --- | --- |
| `Number` | `NumberKind` | `Column::F64` |
| `Boolean` | `BooleanKind` | `Column::Bool` |
| `String` | `TextKind` | `Column::Text` |
| `Date` | `DateKind` | `Column::Date` |
| `List(_)` | `ListKind` | `Column::List` |
| `Generic(_)` | `AnyKind` | `Column::Any` |
| heterogeneous `Union(_)` | `AnyKind` | `Column::Any` |
| `Unknown` | `AnyKind` | `Column::Any` |

If all non-null members of a union map to the same ABI kind, that kind may be retained;
otherwise the mapping falls back to `AnyKind`.

`Null` is represented by independent validity and does not require a matching `Value`
variant. Optional parameters use `Option<KernelColumn<K>>` to represent whether the call
provided that argument; it does not represent per-row nulls.

ABI kinds are generated statically from the declaration template. Generic parameters and
return types map to `AnyKind`; a call's resolved generic binding does not alter the trait.
Future specialization should generate a separate dispatch variant rather than changing an
existing trait ABI.

Controlled functions do not materialize `Fn` and `Ident` as columns:

- `() -> T` maps to a typed thunk plan;
- `(current: T) -> U` maps to a typed lambda plan;
- `Ident<T>` maps to a binder handle; and
- other parameters map to unevaluated typed value plans.

The generated wrapper consumes argument `EvalBlock`s and moves shared column handles and
validity into `KernelColumn`. In debug builds it confirms that the physical `Column`
variant matches the ABI kind. This conversion must not copy row values. Handwritten
kernels do not match the `Column` enum directly.

### Generated argument structures

Every supported builtin generates a named argument struct. Fixed head and tail parameters
become top-level fields rather than being nested in dynamic `head` or `tail` collections.

Optional parameters use `Option<FieldType>`. This represents whether the call supplied the
argument, not per-row nulls.

Every repeat block generates a named group struct, even when it contains one member. The
top-level argument struct holds these groups in a `repeat_groups` field:

```rust,ignore
pub struct ConcatArgs {
    pub repeat_groups: RepeatGroups<ConcatRepeatGroup>,
}

pub struct ConcatRepeatGroup {
    pub lists: KernelColumn<ListKind>,
}
```

Controlled functions use the same structural rules, but field types map to plan handles:

- ordinary value parameter -> `ValuePlan<K>`;
- parameterless lambda -> `ThunkPlan<K>`;
- lambda with parameters -> `LambdaPlan<...>`;
- `Ident<T>` -> `BinderHandle<T>`;
- repeat block -> named plan group.

Field names are deterministically converted from DSL parameter names to snake_case. Rust
keywords receive a trailing `_`; for example, `startIndex` becomes `start_index` and
`else` becomes `else_`. If two parameters produce the same converted field name, the
declaration macro reports a compile error at the latter parameter.

Generated APIs always use logical base names and never generate numbered fields such as
`lists1` or `lists2`. Group numbers exist only in repeat-group element positions and the
presentation layer.

#### Argument ownership

Generated `Args`, `Plans`, and repeat groups carry no lifetime parameters. Ordinary value
arguments use owned `KernelColumn<K>` handles. The dispatch wrapper consumes each freshly
computed `EvalBlock` and moves its matching shared storage handle and validity directly
into the argument struct; moving or cloning a handle does not copy row values.

`KernelColumn` does not expose its storage field. It provides read-only slices, per-row
access, and a way to recover the owned buffer when the reference is unique.
`SharedStorage` hides the concrete reference-counting and buffer implementation from
generated traits.

Repeat groups use owned collections:

```rust,ignore
pub struct RepeatGroups<G>(Box<[G]>);
```

The Planner and runtime have already determined group boundaries; dispatch only moves
groups and neither resolves the shape again nor copies column storage.

Controlled arguments use typed ID handles that do not borrow the IR:

```rust,ignore
pub struct ValuePlan<K> {
    id: PlanId,
    kind: PhantomData<K>,
}

pub struct ThunkPlan<K> {
    body: PlanId,
    result_kind: PhantomData<K>,
}
```

Handles can be resolved only through the current execution context. The context verifies
that a handle belongs to the current plan; it does not remove lifetimes through raw
pointers, `transmute`, or fabricated `'static` references.

Generated controlled traits borrow a generic context directly and do not use trait
objects:

```rust,ignore
pub trait IfsKernel {
    fn eval<C: BuiltinEvalContext>(
        context: &mut C,
        args: IfsPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind>;
}
```

The lifetimes of `&mut C` and `&Mask` are inferred from the function signature and do not
become generic parameters of generated types. A concrete runtime context may carry its own
internal lifetimes and input-store or evaluator-state types; these details do not leak into
generated traits.

`BuiltinEvalContext` need not be object-safe and may provide typed generic methods.
Generated controlled dispatch uses an exhaustive match on `BuiltinKey` and statically
calls the corresponding marker:

```rust,ignore
fn dispatch_controlled<C: BuiltinEvalContext>(
    key: BuiltinKey,
    context: &mut C,
    args: ControlledArgs,
    mask: &Mask,
) -> BuiltinResult {
    match key {
        BuiltinKey::Ifs => {
            <Ifs as IfsKernel>::eval(context, args.into_ifs(), mask).into()
        }
        // The generator exhaustively lists the remaining controlled builtins.
    }
}
```

This requires neither `dyn` dispatch nor erasing the concrete context type to hide
lifetimes.

### Null, error, and output-result contract

The evaluator maintains three separate states in each row batch:

- execution `Mask`: which rows the current step is asked to execute;
- `ok` mask: which rows have no upstream evaluation error; and
- `Validity`: which successfully evaluated rows contain a non-null value.

These states must not be merged in storage. Null is a successful value state, not an
error; a row not selected by the current control flow is also neither null nor an error.

`Validity` has three representations: `AllValid`, `AllNull`, and `Bitmap`. The common
null-free batch needs neither an allocation nor per-row bitmap checks. For kernels that
explicitly propagate null, `AllNull` also permits directly producing an all-null result.
In `Bitmap`, `1` means valid, matching the positive direction of the execution mask, but
the two remain distinct types with distinct semantics.

#### Generated-wrapper responsibilities

For a `Value` builtin, the generated dispatch wrapper:

1. forwards row errors already produced by every argument unchanged;
2. computes `eligible = execution_mask & all_present_inputs_ok`;
3. moves each argument's shared storage handle and validity into named `Args` without
   universally filtering null rows;
4. calls the handwritten kernel, which reports only errors newly produced by this call;
   and
5. merges old and new errors and, in debug builds, checks structural invariants for output
   length, ABI kind, validity, `ok`, and error rows.

Step 2 must not merge validity into `eligible`. For example, `empty(null)` must actively
observe null on a row that remains eligible. For `Controlled` builtins, unevaluated
arguments have no `ok` state available for eager merging; each `context.eval*()` applies
the same rules within that branch's execution mask.

#### Arrow-style compute fast paths

The evaluator provides three implementation-level helper families. Function
implementations choose explicitly according to their semantics:

- `eval_infallible_all_slots`: compute over the full physical buffer, then apply the
  execution mask, input validity, and `ok`. This is valid only for pure, total operations
  that cannot fail or panic;
- `eval_fallible_selected`: compute only active, `ok`, and valid rows, avoiding spurious
  errors from arbitrary placeholders or unselected rows; and
- `eval_null_aware`: retain null rows and let the function implementation inspect validity
  and define the result.

For an ordinary null-propagating kernel, the helper uses:

```text
compute_mask = eligible & all_inputs_valid
```

If the operation is infallible, it may ignore `compute_mask`, perform SIMD computation
over the entire physical buffer, and let output validity and masks determine which results
are observable. Physical results in invalid or inactive slots are placeholders. If the
operation can fail, report an error, or panic, it may compute only rows selected by
`compute_mask`.

Controlled builtins such as `if` and `ifs` do not compute all branches and select a result
afterward. They use `BuiltinEvalContext` to construct an independent execution mask for
each branch; unselected branches must not be evaluated.

Helper selection is neither a builtin-declaration DSL attribute nor inferred automatically
from the function signature. A signature determines parameter and return types but cannot
determine whether an implementation is total, fallible, or null-observing. Generated
wrappers own uniform structure; handwritten kernels choose the semantically correct
compute path.

### Data preparation and synchronous evaluation boundary

The builtin evaluator is a purely synchronous execution layer. It does not call an
asynchronous Provider while evaluating a formula, and `BuiltinEvalContext::eval*()` does
not return Futures. Before entering the evaluator, the caller prepares every column
referenced by the current formula and row batch:

```rust,ignore
let prepared = prepare_formula(formula, schema)?;
let columns = caller.load_columns(prepared.required_columns(), batch).await?;
let result = prepared.evaluate(batch, columns);
```

The asynchronous load above belongs to the caller or an adapter around the evaluator, not
to the builtin trait ABI. Therefore, both `Value` and `Controlled` kernels are synchronous:

```rust,ignore
pub trait IfsKernel {
    fn eval<C: BuiltinEvalContext>(
        context: &mut C,
        args: IfsPlans,
        mask: &Mask,
    ) -> KernelResult<AnyKind>;
}
```

Functions such as `ifs` still compute lazily by branch mask: unselected plans do not execute
and cannot produce runtime errors. The caller has already prepared the corresponding
property columns, so this laziness controls computation, not I/O. A column referenced only
by a branch that is ultimately unselected may still be preloaded; this is an explicit
tradeoff of the synchronous evaluator boundary.

The evaluator provides no `block_on` and maintains no asynchronous runtime internally.
External data sources may still expose asynchronous APIs, but data preparation must finish
before calling the synchronous evaluator. Generated Args/Plans, kernel traits, and
`BuiltinEvalContext` expose no Future, explicit lifetime, or `dyn` trait object.

#### Input dependency manifest

`prepare_formula` statically collects every property reference in the formula, deduplicates
by canonical name, and assigns stable plan-local `InputSlot`s in first-appearance order.
Each property occupies one slot regardless of reference count; references in unselected
branches are also present in the manifest.

```rust,ignore
pub struct InputSlot(u32);

pub struct RequiredColumn {
    pub slot: InputSlot,
    pub name: String,
    pub expected_type: Ty,
}

struct PreparedFormulaBuilder {
    required_columns: Vec<RequiredColumn>,
}

pub struct PreparedFormula {
    plan: ExecPlan,
    required_columns: Box<[RequiredColumn]>,
}

pub struct EvalInputsBuilder {
    columns: Vec<InputColumn>,
}

pub struct EvalInputs {
    batch_len: usize,
    columns: Box<[InputColumn]>,
}

impl PreparedFormula {
    pub fn required_columns(&self) -> &[RequiredColumn];

    pub fn evaluate(
        &self,
        batch: RowBatch,
        inputs: EvalInputs,
    ) -> Result<EvalBlock, InputContractError>;
}
```

Property plan nodes store `InputSlot`s and access columns directly by slot during
synchronous evaluation, without string lookup. The caller loads data according to
`required_columns()` and constructs `EvalInputs` matching that prepared plan.

Manifest and input storage use `Vec<T>` during building and convert to `Box<[T]>` when
finalized. `PreparedFormula`, `EvalInputs`, and `RepeatGroups` all represent owned slices
whose lengths are frozen; fields remain private and expose only `&[T]`, iterators, or
methods that consume the whole container.

`Box<[T]>` does not make elements immutable; it makes the element count fixed. Finalized
structures expose no `&mut [T]`, so callers cannot mutate elements either. Sequential read
performance is identical to `Vec<T>`; the choice expresses a finalized shape rather than a
performance optimization.

#### Input contract validation

When finalized, `EvalInputsBuilder` checks slots, ABI kinds, and batch length against the
`PreparedFormula` input manifest. `EvalInputs` carries an opaque input-layout identity;
`evaluate` uses it to reject inputs constructed for a different prepared plan.

```rust,ignore
pub enum InputContractError {
    MissingColumn {
        slot: InputSlot,
        name: String,
    },
    DuplicateColumn {
        slot: InputSlot,
    },
    WrongKind {
        slot: InputSlot,
        expected: AbiKind,
        actual: AbiKind,
    },
    WrongLength {
        slot: InputSlot,
        expected: usize,
        actual: usize,
    },
    WrongInputLayout,
}

impl EvalInputsBuilder {
    pub fn finish(
        self,
        prepared: &PreparedFormula,
        batch_len: usize,
    ) -> Result<EvalInputs, InputContractError>;
}

impl PreparedFormula {
    pub fn evaluate(
        &self,
        batch: RowBatch,
        inputs: EvalInputs,
    ) -> Result<EvalBlock, InputContractError>;
}
```

These errors mean the caller has not satisfied evaluation preconditions, so the evaluator
starts no kernels and produces no partial `EvalBlock`. They are not copied into one
identical `EvalError` per row:

- `InputContractError`: missing columns, duplicate slots, incorrect ABI kinds, incorrect
  lengths, or an incorrect layout;
- `EvalError`: structurally valid input whose formula evaluation fails for one row;
- null: a valid successful value represented by `Validity`; and
- debug signature assertion: an evaluator or kernel implementation violates the resolved
  function contract.

#### Column fan-out

The same input or intermediate result may be referenced multiple times in a plan. For
example, `prop("Price") + prop("Price")` has one `InputSlot`, but two downstream arguments
need the column. The evaluator neither copies the full column nor adds lifetimes to
generated Args for this purpose.

`KernelColumn<K>` owns cheaply cloneable `SharedStorage<K::Storage>`; bitmap validity is
likewise shared through `SharedBitmap`. Cloning copies only the handle, never row values:

```rust,ignore
let left = context.eval(price_plan, mask);
let right = left.clone();
```

Storage is read-only to kernels by default. New results can construct
`SharedStorage::from_owned(storage)` from an owned buffer. When the reference count is one,
a kernel can recover underlying storage through `try_into_unique()` and perform in-place
SIMD:

```rust,ignore
match column.try_into_unique() {
    Ok(mut storage) => {
        simd_in_place(storage.as_mut());
        KernelColumn::from_owned(storage, validity)
    }
    Err(column) => eval_to_new_storage(column),
}
```

`SharedStorage` is a deep-module boundary in the evaluator runtime. Generated code depends
only on read-only access, cheap cloning, construction from owned storage, and attempting to
recover unique ownership. The implementation may use `Arc<Buffer>` or another
reference-counted representation without changing generated per-function traits.

`Box<[T]>` remains appropriate for finalized collections such as `RequiredColumn`, the
`EvalInputs` slot table, and `RepeatGroups`; it expresses a fixed count. `SharedStorage`
instead solves ownership of column buffers across plan fan-out. The two serve different
purposes.

## Declaration Diagnostics

Invalid declarations must point precisely to the relevant source token. Required test
cases include:

- duplicate builtin and generic names;
- unknown generic kinds, types, and generic references;
- malformed, empty, or repeated repeat blocks;
- optional members inside repeat groups;
- invalid repeat minimums;
- ambiguous head/repeat/tail layouts; and
- unsupported attributes and invalid resolver paths.

### Error collection and recovery

The macro collects independent errors within one category invocation:

1. parse the category header;
2. parse one function declaration at a time;
3. after a declaration syntax error, recover at the next top-level `;`;
4. validate every successfully parsed declaration;
5. combine errors into multiple `compile_error!` expansions; and
6. if any error exists, generate no partial `BuiltinCategory`.

Recovery does not attempt to continue inside a malformed declaration. This keeps
diagnostics predictable while still reporting problems in later functions.

An invocation emits at most 32 errors. When that limit is exceeded, the final diagnostic
reports that additional errors were suppressed.

### Span rules

| Error | Primary span |
| --- | --- |
| Unexpected token | The offending token |
| Missing token | The nearest enclosing function or repeat declaration |
| Unknown type/generic/kind | The unknown identifier |
| Duplicate generic or function | The latter declaration |
| First duplicate declaration | A supplementary error at the original name |
| Invalid repeat minimum | The integer literal |
| Invalid repeat layout | The `repeat` keyword |
| Unsupported declaration missing docs | `#[unsupported]` |
| Incompatible attributes | The latter conflicting attribute |
| malformed resolver path | resolver attribute |

The macro can validate resolver-path syntax and attribute combinations. Ordinary Rust type
checking of the expansion verifies that the referenced function exists and has the
`SigResolver` function type.

### Local and global invariants

A category macro can diagnose duplicate names within its own invocation but cannot see
declarations generated by another macro invocation.

Cross-category duplicate names, category order, and whole-catalog consistency are
therefore checked by catalog contract tests over `builtin_categories()`.

## Verification Strategy

Permanent tests are organized around stable interfaces rather than duplicating a suite for
every builtin:

1. Procedural-macro pass and compile-fail fixtures cover declaration syntax, lowering,
   diagnostics, and spans. Synthetic declarations cover fixed, optional, repeat, tail,
   generic, lambda, resolver, and unsupported forms.
2. Whole-catalog tests mechanically cover every declaration: stable ordering, unique
   names, support status, canonical rendering, and registry inclusion.
3. Table/property tests use synthetic `ParamShape`s and generic combinations to cover the
   shared call-resolution engine, including incomplete and invalid argument counts.
4. A bounded representative matrix permanently covers every `ParamShape` layout. Each row
   retains one canonical declaration-to-resolution test:

   | Layout | Representative | Additional coverage |
   | --- | --- | --- |
   | Fixed only | `flat` | Custom resolver and nested list/union refinement |
   | Repeat only | `concat` | Minimum group count and homogeneous binding |
   | head + repeat | `splice` | Zero or more repeated items |
   | repeat + tail | `ifs` | `Variant`, multi-member groups, tail, and lambda phase |
   | head + repeat + tail | Test fixture `caseOf` | Complete positional shape |

   The current production catalog has no head + repeat + tail builtin, so the final row
   uses a named contract fixture:

   ```rust,ignore
   caseOf<T, U: Variant>(
       subject: T,
       repeat(min = 1) {
           candidate: T,
           result: () -> U,
       },
       otherwise: () -> U,
   ) -> U;
   ```

   A future production builtin with this layout may replace the fixture.
5. Focused Analyzer and IDE end-to-end tests select cases from this matrix and validate
   inferred types, diagnostics, Signature Help projections, numbering, and active
   parameters. Add tests at a consumer layer only for behavior unique to that consumer;
   do not replicate the complete matrix at every layer.
6. The README renderer compares its output byte-for-byte with the committed catalog region.
7. Evaluator golden fixtures execute through the public prepare/input/evaluate interface.
   Every supported catalog entry has one baseline `.formula` / `.snap` pair; additional
   fixtures cover properties, masks, runtime snapshots, lazy execution, and regressions.

Long-term tests do not use a second declaration set or formula parser as an oracle.
Mechanical catalog contracts and evaluator compile contracts cover declaration and ABI
completeness. The five-representative matrix remains the structural oracle for `ParamShape`;
catalog-complete evaluator goldens separately verify runtime semantics. Fixture directives
only describe externally prepared row data and runtime context, while the formula itself
always passes through the production Analyzer and evaluator.

Structural and runtime behavior use different coverage scopes:

| Layer | Representatives | Core guarantee |
| --- | --- | --- |
| DSL / macro | `flat`, `concat`, `splice`, `ifs`, `caseOf` | All five parameter layouts generate correctly |
| Dynamic signature | `flat`, `ifs` | Resolver behavior, generic binding, and return-type inference |
| Generated contract | `flat`, `ifs` | Missing impls or incorrect method signatures fail compilation |
| Value runtime | Every supported Value builtin golden | Public formula-to-row behavior and dispatch wiring |
| Controlled runtime | Every supported Controlled builtin golden | Lambda behavior, branch masks, and selected values |
| Complete shape | Synthetic `caseOf` | Generated head + repeat + tail structure |
| Input contract | One table-driven test | Missing, duplicate, kind, length, and layout errors |
| Catalog / docs | Iterate every declaration | Unique names, ordering, rendering, and documentation consistency |

Every supported builtin keeps exactly one required baseline golden. Add another scenario
fixture only when at least one condition holds:

- it demonstrates property-column, mask, or runtime-context behavior not readable in the
  baseline;
- it protects lazy evaluation or row error isolation; or
- it protects an observed regression or important semantic boundary.

## Explicitly Rejected Designs

- Inferring repetition from parameter names such as `lists1` or `listsN`: repeat is
  expressed by an explicit block and logical base names.
- Maintaining per-function detail for ordinary presentation: detail is canonically
  generated from declaration shape.
- Treating `Flat` as a third generic binding kind: specialized return-type shaping belongs
  to a function-level resolver.
- Adding evaluator fast-path attributes to the builtin DSL: handwritten kernels select
  total, fallible, and null-aware implementation helpers.
- Waiting for external data inside the evaluator: callers prepare complete columns from
  the `RequiredColumn` manifest, and the evaluator and all kernels remain synchronous.

## Acceptance Checklist

- the category DSL can generate the catalog, `FunctionSig`, dynamic call-resolution data,
  and evaluator traits;
- compilation fails when the evaluator lacks an implementation for any supported builtin;
- representatives for fixed, repeat, head + repeat, repeat + tail, and head + repeat + tail
  all pass;
- `flat` and `ifs` cover generated debug input and output contract assertions;
- table-driven `InputContractError` tests cover missing, duplicate, kind, length, and
  layout errors;
- null, row-error, and execution-mask tests prove that the three states are never
  conflated;
- column fan-out tests prove that cloning does not copy rows and that the unique-ownership
  path can recover owned storage;
- the README renderer's catalog region matches committed content byte-for-byte; and
- workspace `just verify` passes.
