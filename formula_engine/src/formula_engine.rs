use std::collections::{BTreeMap, BTreeSet};

use analyzer::analysis::{self, Context, FunctionSig, Property, Ty};
use analyzer::ast::{Expr, ExprKind};
use analyzer::{LitKind, analyze_syntax};

include!("formula_engine.h.rs");

#[derive(Default)]
struct FormulaEngineInner {
    definitions: BTreeMap<PropertyId, PropertyDefinition>,
    dependencies: BTreeMap<PropertyId, BTreeSet<PropertyId>>,
    statuses: BTreeMap<PropertyId, FormulaStatus>,
    cycle_path: Vec<PropertyId>,
}

impl PropertyDefinition {
    fn id(&self) -> &PropertyId {
        match self {
            Self::Input { id, .. } => id,
            Self::Formula(formula) => &formula.id,
        }
    }
}

impl FormulaEngine {
    fn new_impl(schema: FormulaSchema) -> Result<Self, FormulaEngineInitError> {
        let mut definitions = BTreeMap::new();
        for property in schema.properties {
            let id = property.id();
            if id.0.is_empty() {
                return Err(FormulaEngineInitError::EmptyId);
            }
            if definitions.contains_key(id) {
                return Err(FormulaEngineInitError::DuplicateId(id.clone()));
            }
            definitions.insert(id.clone(), property);
        }

        let mut inner = FormulaEngineInner {
            definitions,
            ..FormulaEngineInner::default()
        };
        inner.reanalyze();
        Ok(Self { inner })
    }

    fn property_impl(&self, id: &PropertyId) -> Option<PropertyState> {
        self.inner.property_state(id)
    }

    fn properties_impl(&self) -> Vec<PropertyState> {
        self.inner
            .definitions
            .keys()
            .filter_map(|id| self.inner.property_state(id))
            .collect()
    }

    fn state_impl(&self) -> FormulaEngineState<'_> {
        if self.inner.cycle_path.is_empty()
            && self
                .inner
                .statuses
                .values()
                .all(|status| matches!(status, FormulaStatus::Ready { .. }))
        {
            FormulaEngineState::AllReady
        } else {
            FormulaEngineState::NotAllReady {
                cycle_path: &self.inner.cycle_path,
            }
        }
    }

    fn upsert_impl(
        &mut self,
        property: PropertyDefinition,
    ) -> Result<FormulaEngineChangeResult, EngineChangeError> {
        let id = property.id().clone();
        if id.0.is_empty() {
            return Err(EngineChangeError::EmptyId);
        }
        if self.inner.definitions.get(&id) == Some(&property) {
            return Ok(FormulaEngineChangeResult {
                affected_formulas: Vec::new(),
            });
        }

        let old_dependencies = self.inner.dependencies.clone();
        let changed_formula = matches!(property, PropertyDefinition::Formula(_));
        self.inner.definitions.insert(id.clone(), property);
        self.inner.reanalyze();
        let affected_formulas =
            self.inner
                .affected_formulas(&id, &old_dependencies, changed_formula);
        Ok(FormulaEngineChangeResult { affected_formulas })
    }

    fn remove_impl(&mut self, id: &PropertyId) -> Option<FormulaEngineChangeResult> {
        if !self.inner.definitions.contains_key(id) {
            return None;
        }
        let old_dependencies = self.inner.dependencies.clone();
        self.inner.definitions.remove(id);
        self.inner.reanalyze();
        let affected_formulas = self.inner.affected_formulas(id, &old_dependencies, false);
        Some(FormulaEngineChangeResult { affected_formulas })
    }
}

impl FormulaEngineInner {
    fn property_state(&self, id: &PropertyId) -> Option<PropertyState> {
        match self.definitions.get(id)? {
            PropertyDefinition::Input { id, ty } => Some(PropertyState::Input {
                id: id.clone(),
                ty: ty.clone(),
            }),
            PropertyDefinition::Formula(definition) => Some(PropertyState::Formula(FormulaState {
                definition: definition.clone(),
                status: self
                    .statuses
                    .get(id)
                    .expect("every formula has an analyzed status")
                    .clone(),
            })),
        }
    }

    fn reanalyze(&mut self) {
        let mut parsed = BTreeMap::new();
        for (id, definition) in &self.definitions {
            if let PropertyDefinition::Formula(formula) = definition {
                parsed.insert(id.clone(), ParsedFormula::new(&formula.expression));
            }
        }
        self.dependencies = parsed
            .iter()
            .map(|(id, formula)| (id.clone(), formula.dependencies.clone()))
            .collect();

        let mut analysis = DependencyAnalysis {
            definitions: &self.definitions,
            parsed: &mut parsed,
            functions: analysis::builtins_functions(),
            resolved: BTreeMap::new(),
            cycle_path: Vec::new(),
        };
        for id in self.dependencies.keys() {
            analysis.resolve(id);
        }
        self.statuses = analysis
            .resolved
            .into_iter()
            .map(|(id, ty)| {
                let status = match ty {
                    Some(ty) => FormulaStatus::Ready {
                        output_type: from_analyzer_type(&ty),
                    },
                    None => FormulaStatus::NotReady,
                };
                (id, status)
            })
            .collect();
        self.cycle_path = analysis.cycle_path;
    }

    fn affected_formulas(
        &self,
        changed_id: &PropertyId,
        old_dependencies: &BTreeMap<PropertyId, BTreeSet<PropertyId>>,
        changed_formula: bool,
    ) -> Vec<PropertyId> {
        let mut affected = dependent_formulas(old_dependencies, changed_id);
        affected.extend(dependent_formulas(&self.dependencies, changed_id));
        if changed_formula {
            affected.insert(changed_id.clone());
        }
        affected
            .into_iter()
            .filter(|id| {
                matches!(
                    self.definitions.get(id),
                    Some(PropertyDefinition::Formula(_))
                )
            })
            .collect()
    }
}

struct ParsedFormula {
    expr: Expr,
    syntax_valid: bool,
    dependencies: BTreeSet<PropertyId>,
}

impl ParsedFormula {
    fn new(expression: &str) -> Self {
        let syntax = analyze_syntax(expression);
        let mut dependencies = BTreeSet::new();
        collect_dependencies(&syntax.expr, &mut dependencies);
        Self {
            expr: syntax.expr,
            syntax_valid: syntax.diagnostics.is_empty(),
            dependencies,
        }
    }
}

fn collect_dependencies(expr: &Expr, dependencies: &mut BTreeSet<PropertyId>) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if callee.text == "prop"
                && let [argument] = args.as_slice()
                && let ExprKind::Lit(literal) = &argument.kind
                && literal.kind == LitKind::String
            {
                dependencies.insert(PropertyId(literal.symbol.text.clone()));
            }
            for arg in args {
                collect_dependencies(arg, dependencies);
            }
        }
        ExprKind::Group { inner } | ExprKind::Unary { expr: inner, .. } => {
            collect_dependencies(inner, dependencies);
        }
        ExprKind::List { items } => {
            for item in items {
                collect_dependencies(item, dependencies);
            }
        }
        ExprKind::MemberCall { receiver, args, .. } => {
            collect_dependencies(receiver, dependencies);
            for arg in args {
                collect_dependencies(arg, dependencies);
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_dependencies(left, dependencies);
            collect_dependencies(right, dependencies);
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            collect_dependencies(cond, dependencies);
            collect_dependencies(then, dependencies);
            collect_dependencies(otherwise, dependencies);
        }
        ExprKind::ImplicitLambda { body, .. } => collect_dependencies(body, dependencies),
        ExprKind::Ident(_) | ExprKind::Lit(_) | ExprKind::Error => {}
    }
}

struct DependencyAnalysis<'a> {
    definitions: &'a BTreeMap<PropertyId, PropertyDefinition>,
    parsed: &'a mut BTreeMap<PropertyId, ParsedFormula>,
    functions: Vec<FunctionSig>,
    resolved: BTreeMap<PropertyId, Option<Ty>>,
    cycle_path: Vec<PropertyId>,
}

struct DependencyFrame {
    id: PropertyId,
    dependencies: Vec<PropertyId>,
    next: usize,
    properties: Vec<Property>,
    dependencies_ready: bool,
}

impl DependencyFrame {
    fn new(id: &PropertyId, parsed: &BTreeMap<PropertyId, ParsedFormula>) -> Self {
        let dependencies: Vec<_> = parsed[id].dependencies.iter().cloned().collect();
        Self {
            id: id.clone(),
            properties: Vec::with_capacity(dependencies.len()),
            dependencies,
            next: 0,
            dependencies_ready: true,
        }
    }

    fn accept(&mut self, dependency: PropertyId, ty: Option<Ty>) {
        self.next += 1;
        match ty {
            Some(ty) => self.properties.push(Property {
                name: dependency.0,
                ty,
                disabled_reason: None,
            }),
            None => self.dependencies_ready = false,
        }
    }
}

impl DependencyAnalysis<'_> {
    fn resolve(&mut self, root: &PropertyId) {
        if self.resolved.contains_key(root) {
            return;
        }

        // Keep dependency frames on the heap: a long, valid formula chain must
        // not consume one native call frame per formula.
        let mut frames = vec![DependencyFrame::new(root, self.parsed)];
        let mut active = BTreeMap::from([(root.clone(), 0)]);
        while let Some(current) = frames.len().checked_sub(1) {
            if frames[current].next == frames[current].dependencies.len() {
                let frame = frames.pop().expect("current frame exists");
                active.remove(&frame.id);
                let parsed = self
                    .parsed
                    .get_mut(&frame.id)
                    .expect("every formula has a parsed expression");
                let ty = if frame.dependencies_ready && parsed.syntax_valid {
                    let context = Context {
                        properties: frame.properties,
                        functions: self.functions.clone(),
                    };
                    let (ty, diagnostics) = analysis::analyze_expr(&mut parsed.expr, &context);
                    diagnostics.is_empty().then_some(ty)
                } else {
                    None
                };
                self.resolved.insert(frame.id, ty);
                continue;
            }

            let dependency = frames[current].dependencies[frames[current].next].clone();
            let ty = match self.definitions.get(&dependency) {
                Some(PropertyDefinition::Input { ty, .. }) => Some(to_analyzer_type(ty)),
                Some(PropertyDefinition::Formula(_)) => {
                    if let Some(ty) = self.resolved.get(&dependency) {
                        ty.clone()
                    } else if let Some(&start) = active.get(&dependency) {
                        if self.cycle_path.is_empty() {
                            self.cycle_path = frames[start..]
                                .iter()
                                .map(|frame| frame.id.clone())
                                .chain(std::iter::once(dependency.clone()))
                                .collect();
                        }
                        None
                    } else {
                        active.insert(dependency.clone(), frames.len());
                        frames.push(DependencyFrame::new(&dependency, self.parsed));
                        continue;
                    }
                }
                None => None,
            };
            frames[current].accept(dependency, ty);
        }
    }
}

fn dependent_formulas(
    dependencies: &BTreeMap<PropertyId, BTreeSet<PropertyId>>,
    id: &PropertyId,
) -> BTreeSet<PropertyId> {
    let mut reverse: BTreeMap<&PropertyId, Vec<&PropertyId>> = BTreeMap::new();
    for (formula, referenced) in dependencies {
        for dependency in referenced {
            reverse.entry(dependency).or_default().push(formula);
        }
    }
    let mut affected = BTreeSet::new();
    let mut pending = vec![id.clone()];
    while let Some(current) = pending.pop() {
        for dependent in reverse.get(&current).into_iter().flatten() {
            if affected.insert((*dependent).clone()) {
                pending.push((*dependent).clone());
            }
        }
    }
    affected
}

fn to_analyzer_type(ty: &ValueType) -> Ty {
    match ty {
        ValueType::Number => Ty::Number,
        ValueType::String => Ty::String,
        ValueType::Boolean => Ty::Boolean,
        ValueType::Date => Ty::Date,
        ValueType::Unknown => Ty::Unknown,
        ValueType::List(inner) => Ty::List(Box::new(to_analyzer_type(inner))),
        ValueType::Union(members) => Ty::Union(members.iter().map(to_analyzer_type).collect()),
    }
}

fn from_analyzer_type(ty: &Ty) -> ValueType {
    match ty {
        Ty::Number => ValueType::Number,
        Ty::String => ValueType::String,
        Ty::Boolean => ValueType::Boolean,
        Ty::Date => ValueType::Date,
        Ty::List(inner) => ValueType::List(Box::new(from_analyzer_type(inner))),
        Ty::Union(members) => ValueType::Union(members.iter().map(from_analyzer_type).collect()),
        // These analyzer-only types have no public ValueType representation.
        // Resolution passes the original Ty to dependents; this fallback affects snapshots only.
        Ty::Null | Ty::Unknown | Ty::Generic(_) | Ty::Fn { .. } | Ty::Ident(_) => {
            ValueType::Unknown
        }
    }
}
