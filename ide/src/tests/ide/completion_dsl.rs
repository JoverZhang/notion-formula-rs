use crate::completion::{
    CompletionConfig, CompletionData, CompletionItem, CompletionKind, CompletionOutput, TextEdit,
    complete,
};
use analyzer::semantic::{Context, FunctionSig, Property, Ty, builtins_functions};
use std::collections::HashSet;

// ----------------------------
// Demo Properties
// ----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prop {
    Title,
    Age,
    Flag,
}

// ----------------------------
// Demo Functions
// ----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    If,
    Sum,
}

#[allow(dead_code)]
impl Func {
    pub fn name(&self) -> &'static str {
        match self {
            Func::If => "if",
            Func::Sum => "sum",
        }
    }

    pub fn label(&self) -> String {
        format!("{}()", self.name())
    }

    pub fn data(&self) -> CompletionData {
        CompletionData::Function {
            name: self.name().to_string(),
        }
    }

    pub fn kind(&self) -> CompletionKind {
        match self {
            Func::If => CompletionKind::FunctionGeneral,
            Func::Sum => CompletionKind::FunctionNumber,
        }
    }
}

// ----------------------------
// Builtins (Notion-style identifiers + operators)
// ----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Builtin {
    Not,
    True,
    False,
    EqEq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Plus,
    Minus,
    Star,
    Slash,
}

#[allow(dead_code)]
impl Builtin {
    pub fn label(&self) -> &'static str {
        match self {
            Builtin::Not => "not",
            Builtin::True => "true",
            Builtin::False => "false",
            Builtin::EqEq => "==",
            Builtin::Ne => "!=",
            Builtin::Gt => ">",
            Builtin::Ge => ">=",
            Builtin::Lt => "<",
            Builtin::Le => "<=",
            Builtin::Plus => "+",
            Builtin::Minus => "-",
            Builtin::Star => "*",
            Builtin::Slash => "/",
        }
    }

    pub fn kind(&self) -> CompletionKind {
        match self {
            Builtin::Not | Builtin::True | Builtin::False => CompletionKind::Builtin,
            _ => CompletionKind::Operator,
        }
    }

    pub fn data(&self) -> Option<CompletionData> {
        None
    }
}

// ----------------------------
// Demo Item (unified enum for all completion items)
// ----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Prop(Prop),
    Func(Func),
    Builtin(Builtin),
}

impl Item {
    pub fn label(&self) -> String {
        match self {
            Item::Prop(p) => p.name().to_string(),
            Item::Func(f) => f.label(),
            Item::Builtin(b) => b.label().to_string(),
        }
    }

    pub fn matches(&self, item: &CompletionItem) -> bool {
        match self {
            Item::Prop(prop) => {
                item.kind == CompletionKind::Property
                    && item.data == Some(prop.prop_expr_data())
                    && item.label == prop.name()
            }
            Item::Func(func) => {
                item.kind == func.kind()
                    && item.data == Some(func.data())
                    && item.label == func.label()
            }
            Item::Builtin(b) => {
                item.label == b.label() && item.kind == b.kind() && item.data.is_none()
            }
        }
    }
}

impl Prop {
    pub fn name(&self) -> &'static str {
        match self {
            Prop::Title => "Title",
            Prop::Age => "Age",
            Prop::Flag => "Flag",
        }
    }

    pub fn ty(&self) -> Ty {
        match self {
            Prop::Title => Ty::String,
            Prop::Age => Ty::Number,
            Prop::Flag => Ty::Boolean,
        }
    }

    pub fn prop_expr_data(&self) -> CompletionData {
        CompletionData::PropExpr {
            property_name: self.name().to_string(),
        }
    }
}

// ----------------------------
// Context Builder Extensions
// ----------------------------

#[derive(Clone)]
pub struct ContextBuilder {
    properties: Vec<Property>,
    functions: Vec<FunctionSig>,
}

pub fn ctx() -> ContextBuilder {
    ContextBuilder::default()
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self {
            properties: Vec::new(),
            functions: builtins_functions(),
        }
    }
}

impl ContextBuilder {
    pub fn prop(mut self, name: impl Into<String>, ty: Ty) -> Self {
        self.properties.push(Property {
            name: name.into(),
            ty,
            disabled_reason: None,
        });
        self
    }

    pub fn props_demo_basic(self) -> Self {
        self.props(&[Prop::Title, Prop::Age, Prop::Flag])
    }

    pub fn props(mut self, props: &[Prop]) -> Self {
        for prop in props {
            self.properties.push(Property {
                name: prop.name().to_string(),
                ty: prop.ty(),
                disabled_reason: None,
            });
        }
        self
    }

    pub fn only_funcs(mut self, names: &[&str]) -> Self {
        let mut remaining: HashSet<&str> = names.iter().copied().collect();
        let funcs = builtins_functions()
            .into_iter()
            .filter(|f| remaining.remove(f.name.as_str()))
            .collect::<Vec<_>>();
        assert!(
            remaining.is_empty(),
            "unknown builtin function(s): {:?}",
            remaining
        );
        self.functions = funcs;
        self
    }

    pub fn without_funcs(mut self, names: &[&str]) -> Self {
        let remove: HashSet<&str> = names.iter().copied().collect();
        self.functions.retain(|f| !remove.contains(f.name.as_str()));
        self
    }

    pub fn disabled_prop(
        mut self,
        name: impl Into<String>,
        ty: Ty,
        reason: impl Into<String>,
    ) -> Self {
        self.properties.push(Property {
            name: name.into(),
            ty,
            disabled_reason: Some(reason.into()),
        });
        self
    }

    pub fn build(self) -> Context {
        Context {
            properties: self.properties,
            functions: self.functions,
        }
    }
}

// ----------------------------
// Completion Test DSL
// ----------------------------

pub fn t(input_with_cursor: &str) -> CompletionTestBuilder {
    CompletionTestBuilder::new(input_with_cursor)
}

pub struct CompletionTestBuilder {
    #[allow(dead_code)]
    input_with_cursor: String,
    replaced: String,
    cursor: u32,
    ctx: Option<Context>,
    config: Option<CompletionConfig>,
    output: Option<CompletionOutput>,
    ignore_props: bool,
}

impl CompletionTestBuilder {
    fn new(input_with_cursor: &str) -> Self {
        let cursor = input_with_cursor
            .find("$0")
            .expect("fixture must contain $0 marker");
        let text = input_with_cursor.to_string();
        let replaced = text.replace("$0", "");
        assert!(
            replaced.len() + 2 == text.len(),
            "fixture must contain exactly one $0 marker"
        );

        Self {
            input_with_cursor: text,
            replaced,
            cursor: cursor as u32,
            ctx: None,
            config: None,
            output: None,
            ignore_props: false,
        }
    }

    pub fn ctx(mut self, ctx: Context) -> Self {
        self.ctx = Some(ctx);
        self
    }

    pub fn no_ctx(mut self) -> Self {
        self.ctx = None;
        self
    }

    pub fn preferred_limit(mut self, preferred_limit: usize) -> Self {
        self.config = Some(CompletionConfig { preferred_limit });
        self
    }

    pub fn ignore_props(mut self) -> Self {
        self.ignore_props = true;
        self
    }

    fn visible_items(&mut self) -> Vec<&CompletionItem> {
        // Ensure the output is computed before accessing items
        let _ = self.ensure_run();
        if !self.ignore_props {
            return self
                .output
                .as_ref()
                .map(|out| out.items.iter().collect())
                .unwrap_or_default();
        }
        self.output
            .as_ref()
            .map(|out| {
                out.items
                    .iter()
                    .filter(|item| {
                        // Filter out property items
                        !matches!(item.data, Some(CompletionData::PropExpr { .. }))
                            && item.kind != CompletionKind::Property
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn ensure_run(&mut self) -> &CompletionOutput {
        if self.output.is_none() {
            let config = self.config.unwrap_or_default();
            let out = complete(
                &self.replaced,
                self.cursor as usize,
                self.ctx.as_ref(),
                config,
            );
            self.output = Some(out);
        }
        self.output.as_ref().unwrap()
    }

    pub fn expect_replace_contains_cursor(mut self) -> Self {
        let cursor = self.cursor;
        let out = self.ensure_run();
        assert!(
            out.replace.start <= cursor && cursor <= out.replace.end,
            "replace span must contain cursor: {:?} vs {}",
            out.replace,
            cursor
        );
        self
    }

    pub fn expect_not_empty(mut self) -> Self {
        let out = self.ensure_run();
        assert!(
            !out.items.is_empty(),
            "expected at least one completion item"
        );
        self
    }

    #[allow(dead_code)]
    pub fn expect_empty(mut self) -> Self {
        let out = self.ensure_run();
        assert!(
            out.items.is_empty(),
            "expected no completion items, got {}",
            out.items.len()
        );
        self
    }

    pub fn expect_not_contains(mut self, expected: &[Item]) -> Self {
        let out = self.ensure_run();
        for e in expected {
            assert!(
                !out.items.iter().any(|i| e.matches(i)),
                "expected NOT to contain item: {e:?}"
            );
        }
        self
    }

    pub fn expect_order(mut self, before: &str, after: &str) -> Self {
        let out = self.ensure_run();
        let labels: Vec<&str> = out.items.iter().map(|i| i.label.as_str()).collect();
        let b = labels
            .iter()
            .position(|l| *l == before)
            .unwrap_or_else(|| panic!("missing label {before}\nactual labels: {labels:?}"));
        let a = labels
            .iter()
            .position(|l| *l == after)
            .unwrap_or_else(|| panic!("missing label {after}\nactual labels: {labels:?}"));
        assert!(
            b < a,
            "expected {before} before {after}, but got {b} >= {a}\nactual labels: {labels:?}"
        );
        self
    }

    pub fn item(&mut self, label: &str) -> &CompletionItem {
        let out = self.ensure_run();
        out.items
            .iter()
            .find(|i| i.label == label)
            .unwrap_or_else(|| {
                let labels: Vec<&str> = out.items.iter().map(|i| i.label.as_str()).collect();
                panic!("missing completion item for label {label}\nactual labels: {labels:?}")
            })
    }

    pub fn expect_item_data(mut self, label: &str, data: CompletionData) -> Self {
        let item = { self.item(label).clone() };
        assert_eq!(item.data, Some(data), "unexpected data for item {label}");
        self
    }

    pub fn expect_item_insert_text(mut self, label: &str, expected: &str) -> Self {
        let item = { self.item(label).clone() };
        assert_eq!(
            item.insert_text, expected,
            "unexpected insert_text for item {label}"
        );
        self
    }

    pub fn expect_item_detail(mut self, label: &str, expected: &str) -> Self {
        let item = { self.item(label).clone() };
        assert_eq!(
            item.detail.as_deref(),
            Some(expected),
            "unexpected detail for item {label}"
        );
        self
    }

    pub fn expect_contains_labels(mut self, expected: &[&str]) -> Self {
        let items = self.visible_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for label in expected {
            assert!(
                labels.contains(label),
                "expected to contain label {label}\nactual labels: {labels:?}"
            );
        }
        self
    }

    pub fn expect_not_contains_labels(mut self, expected: &[&str]) -> Self {
        let items = self.visible_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        for label in expected {
            assert!(
                !labels.contains(label),
                "expected NOT to contain label {label}\nactual labels: {labels:?}"
            );
        }
        self
    }

    pub fn expect_item_disabled(mut self, label: &str, reason: Option<&str>) -> Self {
        let item = { self.item(label).clone() };
        assert!(item.is_disabled, "expected item {label} to be disabled");
        assert_eq!(
            item.disabled_reason.as_deref(),
            reason,
            "disabled_reason mismatch for {label}"
        );
        self
    }

    pub fn expect_item_no_primary_edit(mut self, label: &str) -> Self {
        let item = { self.item(label).clone() };
        assert!(
            item.primary_edit.is_none(),
            "expected item {label} to have no primary_edit"
        );
        self
    }

    pub fn expect_item_cursor_after_lparen(mut self, label: &str) -> Self {
        let replace_start = self.ensure_run().replace.start;
        let item = { self.item(label).clone() };

        let lparen_idx = item
            .insert_text
            .find('(')
            .expect("completion insert_text must contain '('");
        assert!(
            item.cursor.is_some(),
            "completion item must provide an explicit cursor"
        );
        assert_eq!(
            item.cursor,
            Some(replace_start + (lparen_idx as u32) + 1),
            "cursor mismatch for {label}"
        );
        self
    }

    pub fn expect_postfix(mut self, func: Func) -> Self {
        let name = func.name();
        let label = format!(".{name}()");
        let item = { self.item(&label).clone() };
        assert_eq!(
            item.kind,
            func.kind(),
            "unexpected kind for postfix {label}"
        );
        assert_eq!(
            item.data,
            Some(CompletionData::PostfixMethod {
                name: name.to_string(),
            }),
            "unexpected data for postfix {label}"
        );
        self
    }

    pub fn expect_not_postfix(mut self, func: Func) -> Self {
        let name = func.name();
        let label = format!(".{name}()");
        let out = self.ensure_run();
        assert!(
            !out.items.iter().any(|item| {
                item.label == label
                    && item.kind == func.kind()
                    && item.data
                        == Some(CompletionData::PostfixMethod {
                            name: name.to_string(),
                        })
            }),
            "expected NOT to contain postfix {label}"
        );
        self
    }

    // ----- new DSL helpers for properties and functions -----

    pub fn expect_prop(mut self, prop: Prop) -> Self {
        let items = self.visible_items();
        let item = items
            .iter()
            .find(|i| i.label == prop.name())
            .unwrap_or_else(|| {
                let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
                panic!(
                    "missing completion item for property {:?}\nactual labels: {labels:?}",
                    prop.name()
                )
            });
        assert!(
            item.kind == CompletionKind::Property,
            "expected item to be Property, got {:?}",
            item.kind
        );
        assert_eq!(
            item.data,
            Some(prop.prop_expr_data()),
            "unexpected data for property {:?}",
            prop.name()
        );
        self
    }

    pub fn expect_func(mut self, name: &str) -> Self {
        let label = format!("{name}()");
        let items = self.visible_items();
        let item = items
            .iter()
            .find(|i| i.label == label.as_str())
            .unwrap_or_else(|| {
                let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
                panic!("missing completion item for function {name}\nactual labels: {labels:?}")
            });
        assert!(
            item.kind.is_function(),
            "expected item to be a function kind, got {:?}",
            item.kind
        );
        assert_eq!(
            item.data,
            Some(CompletionData::Function {
                name: name.to_string()
            }),
            "unexpected data for function {name}"
        );
        self
    }

    pub fn expect_top_labels(mut self, expected: &[&str]) -> Self {
        let items = self.visible_items();
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.len() >= expected.len(),
            "expected at least {} items, got {}\nactual labels: {labels:?}",
            expected.len(),
            labels.len()
        );
        for (idx, exp) in expected.iter().enumerate() {
            assert_eq!(
                labels[idx], *exp,
                "prefix mismatch at index {idx}\nactual labels: {labels:?}"
            );
        }
        self
    }

    // ----- signature help -----

    pub fn expect_sig_active(mut self, active_param: usize) -> Self {
        let out = self.ensure_run();
        let sig = out
            .signature_help
            .as_ref()
            .expect("expected signature help");
        assert_eq!(sig.active_parameter, active_param);
        self
    }

    pub fn expect_sig_active_param_name(mut self, expected: &str) -> Self {
        let out = self.ensure_run();
        let sig = out
            .signature_help
            .as_ref()
            .expect("expected signature help");
        let active_sig = sig
            .signatures
            .get(sig.active_signature)
            .or_else(|| sig.signatures.first())
            .expect("expected at least one signature");

        let active = active_sig
            .segments
            .iter()
            .find_map(|seg| match seg {
                crate::DisplaySegment::Param {
                    name,
                    param_index: Some(i),
                    ..
                } if (*i as usize) == sig.active_parameter => Some(name.as_str()),
                _ => None,
            })
            .expect("expected active parameter segment");

        assert_eq!(active, expected);
        self
    }

    pub fn expect_sig_label(mut self, label: &str) -> Self {
        let out = self.ensure_run();
        let sig = out
            .signature_help
            .as_ref()
            .expect("expected signature help");
        let active = sig
            .signatures
            .get(sig.active_signature)
            .or_else(|| sig.signatures.first())
            .expect("expected at least one signature");
        let mut rendered = String::new();
        for seg in &active.segments {
            use crate::DisplaySegment as S;
            match seg {
                S::Name { text }
                | S::Punct { text }
                | S::Separator { text }
                | S::Arrow { text }
                | S::ReturnType { text } => rendered.push_str(text.as_str()),
                S::Ellipsis => rendered.push_str("..."),
                S::Param { name, ty, .. } => {
                    rendered.push_str(name.as_str());
                    rendered.push_str(": ");
                    rendered.push_str(ty.as_str());
                }
            }
        }
        assert_eq!(rendered, label);
        self
    }

    pub fn expect_sig_label_not_contains(mut self, substr: &str) -> Self {
        let out = self.ensure_run();
        let sig = out
            .signature_help
            .as_ref()
            .expect("expected signature help");
        let active = sig
            .signatures
            .get(sig.active_signature)
            .or_else(|| sig.signatures.first())
            .expect("expected at least one signature");
        let mut rendered = String::new();
        for seg in &active.segments {
            use crate::DisplaySegment as S;
            match seg {
                S::Name { text }
                | S::Punct { text }
                | S::Separator { text }
                | S::Arrow { text }
                | S::ReturnType { text } => rendered.push_str(text.as_str()),
                S::Ellipsis => rendered.push_str("..."),
                S::Param { name, ty, .. } => {
                    rendered.push_str(name.as_str());
                    rendered.push_str(": ");
                    rendered.push_str(ty.as_str());
                }
            }
        }
        assert!(
            !rendered.contains(substr),
            "did not expect signature label to contain {substr:?}, got: {rendered}",
        );
        self
    }

    pub fn expect_no_signature_help(mut self) -> Self {
        let out = self.ensure_run();
        assert!(
            out.signature_help.is_none(),
            "expected no signature help, got: {:?}",
            out.signature_help
        );
        self
    }

    pub fn expect_preferred_indices(mut self, expected: &[usize]) -> Self {
        let out = self.ensure_run();
        assert_eq!(out.preferred_indices, expected);
        self
    }

    pub fn expect_preferred_indices_empty(self) -> Self {
        self.expect_preferred_indices(&[])
    }

    pub fn items_kinds_labels(mut self) -> Vec<(CompletionKind, String)> {
        let out = self.ensure_run();
        out.items
            .iter()
            .map(|i| (i.kind, i.label.clone()))
            .collect()
    }

    pub fn expect_items_kinds_labels(mut self, expected: &[(CompletionKind, String)]) -> Self {
        let out = self.ensure_run();
        let actual = out
            .items
            .iter()
            .map(|i| (i.kind, i.label.clone()))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        self
    }

    pub fn expect_kind_runs_not_fragmented(mut self) -> Self {
        let out = self.ensure_run();

        let mut runs = Vec::<CompletionKind>::new();
        let mut last = None;
        for item in &out.items {
            if last != Some(item.kind) {
                runs.push(item.kind);
                last = Some(item.kind);
            }
        }

        let mut seen = HashSet::<std::mem::Discriminant<CompletionKind>>::new();
        for kind in runs {
            let disc = std::mem::discriminant(&kind);
            assert!(
                seen.insert(disc),
                "completion kind run is fragmented; kind {:?} re-appeared after a different kind",
                kind
            );
        }
        self
    }

    // ----- typed DSL helpers -----

    fn find_item(&mut self, expected: Item) -> &CompletionItem {
        let items = self.visible_items();
        items
            .iter()
            .find(|i| expected.matches(i))
            .unwrap_or_else(|| {
                let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
                panic!(
                    "missing completion item for {:?}\nactual labels: {labels:?}",
                    expected.label()
                )
            })
    }

    pub fn expect_contains_items(mut self, expected: &[Item]) -> Self {
        for item in expected {
            self.find_item(*item);
        }
        self
    }

    pub fn expect_contains_props(mut self, expected: &[Prop]) -> Self {
        for prop in expected {
            self.find_item(Item::Prop(*prop));
        }
        self
    }

    pub fn expect_contains_funcs(mut self, expected: &[Func]) -> Self {
        for func in expected {
            self.find_item(Item::Func(*func));
        }
        self
    }

    pub fn expect_contains_builtins(mut self, expected: &[Builtin]) -> Self {
        for b in expected {
            self.find_item(Item::Builtin(*b));
        }
        self
    }

    pub fn expect_top_items(mut self, expected: &[Item]) -> Self {
        let items = self.visible_items();
        assert!(
            items.len() >= expected.len(),
            "expected at least {} items, got {}\nactual labels: {:?}",
            expected.len(),
            items.len(),
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>()
        );
        for (idx, exp) in expected.iter().enumerate() {
            assert!(
                exp.matches(items[idx]),
                "item mismatch at index {}: expected {:?}, got {:?}",
                idx,
                exp.label(),
                items[idx].label
            );
        }
        self
    }

    pub fn expect_order_items(mut self, a: Item, b: Item) -> Self {
        let items = self.visible_items();
        let a_idx = items.iter().position(|i| a.matches(i)).unwrap_or_else(|| {
            let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
            panic!(
                "missing completion item for {:?}\nactual labels: {labels:?}",
                a.label()
            )
        });
        let b_idx = items.iter().position(|i| b.matches(i)).unwrap_or_else(|| {
            let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
            panic!(
                "missing completion item for {:?}\nactual labels: {labels:?}",
                b.label()
            )
        });
        assert!(
            a_idx < b_idx,
            "expected {:?} before {:?}, but got {} >= {}",
            a.label(),
            b.label(),
            a_idx,
            b_idx
        );
        self
    }

    // ----- apply completion -----

    pub fn apply(mut self, label: &str) -> ApplyResult {
        let replaced = self.replaced.clone();
        let out = self.ensure_run();
        let item = out
            .items
            .iter()
            .find(|i| i.label == label)
            .unwrap_or_else(|| {
                let labels: Vec<&str> = out.items.iter().map(|i| i.label.as_str()).collect();
                panic!("missing completion item for label {label}\nactual labels: {labels:?}")
            });

        assert!(
            !item.is_disabled,
            "completion item {label} is disabled and must not be applicable"
        );
        let primary = item.primary_edit.as_ref().expect("expected primary edit");
        assert_eq!(
            primary.range, out.replace,
            "primary edit range must match output replace span"
        );

        let mut edits = Vec::with_capacity(1 + item.additional_edits.len());
        edits.push(primary.clone());
        edits.extend(item.additional_edits.iter().cloned());

        let updated = apply_text_edits(&replaced, &edits);
        let new_cursor = item.cursor.unwrap_or_else(|| {
            out.replace
                .start
                .saturating_add(primary.new_text.len() as u32)
        });
        assert!((new_cursor as usize) <= updated.len());

        ApplyResult {
            updated,
            cursor: new_cursor,
        }
    }
}

pub struct ApplyResult {
    pub updated: String,
    pub cursor: u32,
}

impl ApplyResult {
    /// If `expected` contains exactly one `$0`, this asserts both:
    /// - updated text equals expected with `$0` removed
    /// - cursor equals the byte index where `$0` was
    ///
    /// Otherwise, only asserts updated text.
    pub fn expect_text(self, expected: &str) -> Self {
        let mut idx = None;

        // fast scan: find + ensure exactly one marker
        if let Some(i) = expected.find("$0") {
            let count = expected.matches("$0").count();
            idx = Some(i);
            assert_eq!(
                count, 1,
                "expected_text must contain exactly one `$0` marker"
            );
        }

        if let Some(cursor_idx) = idx {
            let expected_text = expected.replace("$0", "");
            assert_eq!(self.updated, expected_text, "text mismatch");
            assert_eq!(self.cursor, cursor_idx as u32, "cursor mismatch");
        } else {
            assert_eq!(self.updated, expected, "text mismatch");
        }

        self
    }
}

fn apply_text_edits(original: &str, edits: &[TextEdit]) -> String {
    let mut edits_with_idx = edits.iter().enumerate().collect::<Vec<_>>();
    edits_with_idx.sort_by(|(a_idx, a), (b_idx, b)| {
        let a_key = (
            std::cmp::Reverse(a.range.start),
            std::cmp::Reverse(a.range.end),
            *a_idx,
        );
        let b_key = (
            std::cmp::Reverse(b.range.start),
            std::cmp::Reverse(b.range.end),
            *b_idx,
        );
        a_key.cmp(&b_key)
    });

    let mut updated = original.to_string();
    for (_, edit) in edits_with_idx {
        let start = edit.range.start as usize;
        let end = edit.range.end as usize;
        assert!(start <= end);
        assert!(end <= updated.len());
        assert!(updated.is_char_boundary(start));
        assert!(updated.is_char_boundary(end));

        let mut next = String::with_capacity(updated.len() - (end - start) + edit.new_text.len());
        next.push_str(&updated[..start]);
        next.push_str(&edit.new_text);
        next.push_str(&updated[end..]);
        updated = next;
    }
    updated
}
