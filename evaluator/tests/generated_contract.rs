#[path = "../build_support.rs"]
mod build_support;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use builtin_fn::{BuiltinCategory, builtin_functions};

fn synthetic_complete_shape() -> BuiltinCategory {
    builtin_functions! {
        category: General;

        caseOf<T, U: Variant>(
            subject: T,
            repeat(min = 1) {
                candidate: T,
                result: () -> U,
            },
            otherwise: () -> U,
        ) -> U;
    }
}

#[test]
fn generated_contract_is_deterministic_ordered_and_unique() {
    let categories = builtin_fn::builtin_categories();
    let first = build_support::generate_contract(&categories);
    let second = build_support::generate_contract(&categories);
    assert_eq!(first, second);

    let names = categories
        .iter()
        .flat_map(|category| &category.entries)
        .filter(|entry| entry.is_supported())
        .map(|entry| entry.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 83);
    assert_eq!(names.iter().copied().collect::<HashSet<_>>().len(), 83);

    let mut cursor = 0;
    for name in names {
        let marker = format!("{name:?} => ::core::option::Option::Some(Self::");
        let offset = first[cursor..]
            .find(&marker)
            .unwrap_or_else(|| panic!("generated catalog omitted `{name}`"));
        cursor += offset + marker.len();
    }

    assert!(!first.contains("dyn BuiltinEvalContext"));
    assert!(!first.contains("BuiltinKernelContext<'_>"));
    assert!(!first.contains("todo!"));
    assert!(!first.contains("unreachable!"));
    assert!(!first.contains("placeholder implementation"));
    assert!(!first.contains("DynamicValueArgs"));
    assert!(!first.contains("into_dynamic"));
    assert!(first.contains("fn eval<C: BuiltinValueContext>"));
    for line in first.lines().filter(|line| {
        line.starts_with("pub(crate) struct ")
            && (line.contains("Args") || line.contains("Plans") || line.contains("RepeatGroup"))
    }) {
        assert!(!line.contains("<'"), "generated lifetime in `{line}`");
    }
}

#[test]
fn generated_structures_cover_the_five_parameter_shapes() {
    let categories = builtin_fn::builtin_categories();
    let generated = build_support::generate_contract_for_names(
        &categories,
        &["flat", "concat", "splice", "ifs"],
    )
    .expect("production representatives are supported");

    assert!(generated.contains("pub(crate) struct FlatArgs {"));
    assert!(generated.contains("pub(crate) list: KernelColumn<ListKind>"));
    assert!(generated.contains("pub(crate) struct ConcatRepeatGroup {"));
    assert!(generated.contains("pub(crate) lists: KernelColumn<ListKind>"));
    assert!(generated.contains("pub(crate) struct SpliceArgs {"));
    assert!(generated.contains("pub(crate) repeat_groups: RepeatGroups<SpliceRepeatGroup>"));
    assert!(generated.contains("pub(crate) struct IfsPlans {"));
    assert!(generated.contains("pub(crate) else_: ThunkPlan<AnyKind>"));

    let generated =
        build_support::generate_contract_for_names(&[synthetic_complete_shape()], &["caseOf"])
            .expect("synthetic complete shape is supported");
    assert!(generated.contains("pub(crate) struct CaseOfRepeatGroup {"));
    assert!(generated.contains("pub(crate) subject: ValuePlan<AnyKind>"));
    assert!(generated.contains("pub(crate) candidate: ValuePlan<AnyKind>"));
    assert!(generated.contains("pub(crate) result: ThunkPlan<AnyKind>"));
    assert!(generated.contains("pub(crate) repeat_groups: RepeatGroups<CaseOfRepeatGroup>"));
    assert!(generated.contains("pub(crate) otherwise: ThunkPlan<AnyKind>"));
}

#[test]
fn missing_flat_implementation_is_a_compile_error() {
    let generated =
        build_support::generate_contract_for_names(&builtin_fn::builtin_categories(), &["flat"])
            .expect("flat is supported");
    let output = compile_fixture("flat_missing_impl", &generated, "");
    assert!(
        !output.status.success(),
        "missing impl unexpectedly compiled"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E0277") && stderr.contains("FlatKernel") && stderr.contains("Flat"),
        "unexpected compiler diagnostic:\n{stderr}"
    );
}

#[test]
fn wrong_ifs_method_signature_is_a_compile_error() {
    let generated =
        build_support::generate_contract_for_names(&builtin_fn::builtin_categories(), &["ifs"])
            .expect("ifs is supported");
    let wrong_impl = r#"
impl IfsKernel for Ifs {
    fn eval<C: BuiltinEvalContext>(
        _context: &mut C,
        _args: IfsPlans,
        _mask: &Mask,
    ) -> KernelResult<ListKind> {
        KernelResult(::core::marker::PhantomData)
    }
}
"#;
    let output = compile_fixture("ifs_wrong_signature", &generated, wrong_impl);
    assert!(!output.status.success(), "wrong impl unexpectedly compiled");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E0053")
            && stderr.contains("expected `AnyKind`, found `ListKind`")
            && stderr.contains("method `eval`"),
        "unexpected compiler diagnostic:\n{stderr}"
    );
}

fn compile_fixture(name: &str, generated: &str, implementation: &str) -> Output {
    let directory = TempDirectory::new(name);
    let source = directory.path.join("fixture.rs");
    fs::write(
        &source,
        format!("{COMPILE_PRELUDE}\n{generated}\n{implementation}"),
    )
    .expect("write generated compile fixture");

    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .arg("--crate-name")
        .arg(name)
        .arg("--crate-type=lib")
        .arg("--edition=2024")
        .arg("--error-format=short")
        .arg("--out-dir")
        .arg(&directory.path)
        .arg(source)
        .output()
        .expect("run rustc for generated contract fixture")
}

struct TempDirectory {
    path: PathBuf,
}

impl TempDirectory {
    fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "notion-formula-evaluator-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create generated contract fixture directory");
        Self { path }
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

const COMPILE_PRELUDE: &str = r#"
#![allow(dead_code)]

use ::core::marker::PhantomData;
use ::std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AbiKind { Any, Number, Boolean, Text, Date, List }

struct FunctionSig { name: String }

fn signature_for_key(key: BuiltinKey) -> &'static FunctionSig {
    static SIGNATURE: OnceLock<FunctionSig> = OnceLock::new();
    SIGNATURE.get_or_init(|| FunctionSig { name: key.name().to_owned() })
}

#[derive(Clone, Copy)]
enum ParamRef { Head(usize), Repeat(usize), Tail(usize) }

struct AnyKind;
struct NumberKind;
struct BooleanKind;
struct TextKind;
struct DateKind;
struct ListKind;
struct KernelColumn<K>(PhantomData<K>);

struct KernelResult<K>(PhantomData<K>);
trait BuiltinValueContext {}
#[derive(Clone)]
struct Mask;
struct EvalBlock;
struct DebugCallContract;
struct PreparedArgumentError;

struct RepeatGroups<G>(Vec<G>);

impl<G> RepeatGroups<G> {
    fn new(groups: Vec<G>) -> Self { Self(groups) }
}

struct ValuePlan<K>(PhantomData<K>);
struct ThunkPlan<K>(PhantomData<K>);
struct LambdaPlan<K>(PhantomData<K>);
struct BinderHandle<K>(PhantomData<K>);

trait BuiltinEvalContext {}

struct PreparedValueArguments { mask: Mask }

impl PreparedValueArguments {
    fn take_value<K>(
        &mut self,
        _parameter: ParamRef,
        _group: Option<usize>,
    ) -> Result<KernelColumn<K>, PreparedArgumentError> {
        Ok(KernelColumn(PhantomData))
    }

    fn eligible(&self) -> &Mask { &self.mask }

    fn finish<K>(
        self,
        _result: KernelResult<K>,
        _key: BuiltinKey,
        _contract: Option<&DebugCallContract>,
    ) -> EvalBlock { EvalBlock }

    fn contract_failure(self, _error: PreparedArgumentError) -> EvalBlock { EvalBlock }

    fn invalid_mode(self, _key: BuiltinKey) -> EvalBlock { EvalBlock }
}

struct PreparedControlledArguments;

impl PreparedControlledArguments {
    fn repeat_group_count(&self) -> usize { 1 }

    fn take_value<K>(
        &mut self,
        _parameter: ParamRef,
        _group: Option<usize>,
    ) -> Result<ValuePlan<K>, PreparedArgumentError> {
        Ok(ValuePlan(PhantomData))
    }

    fn take_thunk<K>(
        &mut self,
        _parameter: ParamRef,
        _group: Option<usize>,
    ) -> Result<ThunkPlan<K>, PreparedArgumentError> {
        Ok(ThunkPlan(PhantomData))
    }

    fn contract_failure(
        self,
        _error: PreparedArgumentError,
        _mask: &Mask,
    ) -> EvalBlock { EvalBlock }

    fn invalid_mode(self, _key: BuiltinKey, _mask: &Mask) -> EvalBlock { EvalBlock }
}

fn finish_controlled_result<K>(
    _result: KernelResult<K>,
    _key: BuiltinKey,
    _mask: &Mask,
    _contract: Option<&DebugCallContract>,
) -> EvalBlock { EvalBlock }
"#;
