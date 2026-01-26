use crate::ast::{Expr, ExprKind};
use crate::diagnostics::{Diagnostic, DiagnosticKind};
use crate::token::{LitKind, Span};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,
    List(Box<Ty>),
    Union(Vec<Ty>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum FunctionCategory {
    General,
    Text,
    Number,
    Date,
    People,
    List,
    Special,
}

pub fn ty_accepts(expected: &Ty, actual: &Ty) -> bool {
    if matches!(expected, Ty::Unknown) || matches!(actual, Ty::Unknown) {
        return true;
    }
    match (expected, actual) {
        (Ty::Union(branches), actual) => branches.iter().any(|t| ty_accepts(t, actual)),
        (Ty::List(e), Ty::List(a)) => ty_accepts(e, a),
        _ => expected == actual,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionSig {
    pub name: String,
    pub params: Vec<ParamSig>,
    pub ret: Ty,
    pub detail: Option<String>,
    pub min_args: usize,
    pub category: FunctionCategory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParamSig {
    pub name: Option<String>,
    pub ty: Ty,
    pub optional: bool,
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: Ty,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub properties: Vec<Property>,
    pub functions: Vec<FunctionSig>,
}

impl FunctionSig {
    pub fn is_variadic(&self) -> bool {
        self.params.last().is_some_and(|p| p.variadic)
    }

    pub fn fixed_params_len(&self) -> usize {
        if self.is_variadic() {
            self.params.len().saturating_sub(1)
        } else {
            self.params.len()
        }
    }

    pub fn effective_min_args(&self) -> usize {
        if self.min_args > 0 {
            return self.min_args;
        }
        if self.is_variadic() {
            self.fixed_params_len()
        } else {
            self.params.len()
        }
    }

    pub fn param_for_arg_index(&self, idx: usize) -> Option<&ParamSig> {
        if idx < self.params.len() {
            return self.params.get(idx);
        }
        if self.is_variadic() {
            return self.params.last();
        }
        None
    }
}

pub fn builtins_functions() -> Vec<FunctionSig> {
    vec![
        // =========================
        // General / Logic
        // =========================
        FunctionSig {
            name: "if".into(),
            params: vec![
                ParamSig {
                    name: Some("condition".into()),
                    ty: Ty::Boolean,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("then".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("else".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Unknown,
            detail: Some("if(condition, then, else)".into()),
            min_args: 0,
            category: FunctionCategory::General,
        },
        FunctionSig {
            name: "ifs".into(),
            // Note: Notion's ifs is (cond1, value1, cond2, value2, ..., default)
            // ParamSig can't express pair-groups well; we model as variadic values.
            params: vec![ParamSig {
                name: Some("args".into()),
                ty: Ty::Unknown,
                optional: false,
                variadic: true,
            }],
            ret: Ty::Unknown,
            detail: Some("ifs(condition, value, ..., default)".into()),
            min_args: 3,
            category: FunctionCategory::General,
        },
        FunctionSig {
            name: "empty".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Unknown,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Boolean,
            detail: Some("empty(value)".into()),
            min_args: 1,
            category: FunctionCategory::General,
        },
        FunctionSig {
            name: "format".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Unknown,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("format(value)".into()),
            min_args: 1,
            category: FunctionCategory::General,
        },
        FunctionSig {
            name: "toNumber".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Unknown,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("toNumber(value)".into()),
            min_args: 1,
            category: FunctionCategory::General,
        },
        // =========================
        // Text
        // =========================
        FunctionSig {
            name: "length".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Union(vec![Ty::String, Ty::List(Box::new(Ty::Unknown))]),
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("length(text|any[])".into()),
            min_args: 1,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "substring".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("start".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("end".into()),
                    ty: Ty::Number,
                    optional: true,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("substring(text, start, end?)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "contains".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("search".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("contains(text, search)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "test".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("regex".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("test(text, regex)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "match".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("regex".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::List(Box::new(Ty::String)),
            detail: Some("match(text, regex)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "replace".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("regex".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("replacement".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("replace(text, regex, replacement)".into()),
            min_args: 3,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "replaceAll".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("regex".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("replacement".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("replaceAll(text, regex, replacement)".into()),
            min_args: 3,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "lower".into(),
            params: vec![ParamSig {
                name: Some("text".into()),
                ty: Ty::String,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("lower(text)".into()),
            min_args: 1,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "upper".into(),
            params: vec![ParamSig {
                name: Some("text".into()),
                ty: Ty::String,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("upper(text)".into()),
            min_args: 1,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "repeat".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("times".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("repeat(text, times)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "link".into(),
            params: vec![
                ParamSig {
                    name: Some("label".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("url".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("link(label, url)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "style".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("styles".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: true,
                },
            ],
            ret: Ty::String,
            detail: Some("style(text, styleOrColor, ...)".into()),
            min_args: 2,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "unstyle".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("styles".into()),
                    ty: Ty::String,
                    optional: true,
                    variadic: true,
                },
            ],
            ret: Ty::String,
            detail: Some("unstyle(text, style?)".into()),
            min_args: 1,
            category: FunctionCategory::Text,
        },
        FunctionSig {
            name: "trim".into(),
            params: vec![ParamSig {
                name: Some("text".into()),
                ty: Ty::String,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("trim(text)".into()),
            min_args: 1,
            category: FunctionCategory::Text,
        },
        // =========================
        // Number
        // =========================
        FunctionSig {
            name: "add".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("add(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "subtract".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("subtract(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "multiply".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("multiply(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "divide".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("divide(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "mod".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("mod(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "pow".into(),
            params: vec![
                ParamSig {
                    name: Some("base".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("exp".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("pow(base, exp)".into()),
            min_args: 2,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "min".into(),
            params: vec![ParamSig {
                name: Some("values".into()),
                ty: Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                optional: false,
                variadic: true,
            }],
            ret: Ty::Number,
            detail: Some("min(number|number[], ...)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "max".into(),
            params: vec![ParamSig {
                name: Some("values".into()),
                ty: Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                optional: false,
                variadic: true,
            }],
            ret: Ty::Number,
            detail: Some("max(number|number[], ...)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "sum".into(),
            params: vec![ParamSig {
                name: Some("values".into()),
                ty: Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                optional: false,
                variadic: true,
            }],
            ret: Ty::Number,
            detail: Some("sum(number|number[], ...)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "median".into(),
            params: vec![ParamSig {
                name: Some("values".into()),
                ty: Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                optional: false,
                variadic: true,
            }],
            ret: Ty::Number,
            detail: Some("median(number|number[], ...)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "mean".into(),
            params: vec![ParamSig {
                name: Some("values".into()),
                ty: Ty::Union(vec![Ty::Number, Ty::List(Box::new(Ty::Number))]),
                optional: false,
                variadic: true,
            }],
            ret: Ty::Number,
            detail: Some("mean(number|number[], ...)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "abs".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("abs(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "round".into(),
            params: vec![
                ParamSig {
                    name: Some("value".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("places".into()),
                    ty: Ty::Number,
                    optional: true,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("round(number, places?)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "ceil".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("ceil(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "floor".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("floor(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "sqrt".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("sqrt(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "cbrt".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("cbrt(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "exp".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("exp(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "ln".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("ln(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "log10".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("log10(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "log2".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("log2(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "sign".into(),
            params: vec![ParamSig {
                name: Some("value".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("sign(number)".into()),
            min_args: 1,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "pi".into(),
            params: vec![],
            ret: Ty::Number,
            detail: Some("pi()".into()),
            min_args: 0,
            category: FunctionCategory::Number,
        },
        FunctionSig {
            name: "e".into(),
            params: vec![],
            ret: Ty::Number,
            detail: Some("e()".into()),
            min_args: 0,
            category: FunctionCategory::Number,
        },
        // =========================
        // Date
        // =========================
        FunctionSig {
            name: "now".into(),
            params: vec![],
            ret: Ty::Date,
            detail: Some("now()".into()),
            min_args: 0,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "today".into(),
            params: vec![],
            ret: Ty::Date,
            detail: Some("today()".into()),
            min_args: 0,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "minute".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("minute(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "hour".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("hour(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "day".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("day(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "date".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("date(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "week".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("week(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "month".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("month(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "year".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("year(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateAdd".into(),
            params: vec![
                ParamSig {
                    name: Some("date".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("amount".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("unit".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Date,
            detail: Some("dateAdd(date, amount, unit)".into()),
            min_args: 3,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateSubtract".into(),
            params: vec![
                ParamSig {
                    name: Some("date".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("amount".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("unit".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Date,
            detail: Some("dateSubtract(date, amount, unit)".into()),
            min_args: 3,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateBetween".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("unit".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("dateBetween(a, b, unit)".into()),
            min_args: 3,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateRange".into(),
            params: vec![
                ParamSig {
                    name: Some("start".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("end".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
            ],
            // Note: if you have Ty::DateRange, use it instead of Ty::Date here.
            ret: Ty::Date,
            detail: Some("dateRange(start, end)".into()),
            min_args: 2,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateStart".into(),
            params: vec![ParamSig {
                name: Some("range".into()),
                // Note: if you have Ty::DateRange, use it here instead of Ty::Date.
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Date,
            detail: Some("dateStart(range)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "dateEnd".into(),
            params: vec![ParamSig {
                name: Some("range".into()),
                // Note: if you have Ty::DateRange, use it here instead of Ty::Date.
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Date,
            detail: Some("dateEnd(range)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "timestamp".into(),
            params: vec![ParamSig {
                name: Some("date".into()),
                ty: Ty::Date,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Number,
            detail: Some("timestamp(date)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "fromTimestamp".into(),
            params: vec![ParamSig {
                name: Some("timestamp".into()),
                ty: Ty::Number,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Date,
            detail: Some("fromTimestamp(timestampMs)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "formatDate".into(),
            params: vec![
                ParamSig {
                    name: Some("date".into()),
                    ty: Ty::Date,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("format".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("formatDate(date, format)".into()),
            min_args: 2,
            category: FunctionCategory::Date,
        },
        FunctionSig {
            name: "parseDate".into(),
            params: vec![ParamSig {
                name: Some("text".into()),
                ty: Ty::String,
                optional: false,
                variadic: false,
            }],
            ret: Ty::Date,
            detail: Some("parseDate(text)".into()),
            min_args: 1,
            category: FunctionCategory::Date,
        },
        // =========================
        // People
        // =========================
        FunctionSig {
            name: "name".into(),
            params: vec![ParamSig {
                name: Some("person".into()),
                // TODO: Notion's person type is more complex than this.
                ty: Ty::Unknown,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("name(person)".into()),
            min_args: 1,
            category: FunctionCategory::People,
        },
        FunctionSig {
            name: "email".into(),
            params: vec![ParamSig {
                name: Some("person".into()),
                // TODO: Notion's person type is more complex than this.
                ty: Ty::Unknown,
                optional: false,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("email(person)".into()),
            min_args: 1,
            category: FunctionCategory::People,
        },
        // =========================
        // List
        // =========================
        FunctionSig {
            name: "at".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("index".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Unknown,
            detail: Some("at(list, index)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "first".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::Unknown,
            detail: Some("first(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "last".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::Unknown,
            detail: Some("last(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "slice".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("start".into()),
                    ty: Ty::Number,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("end".into()),
                    ty: Ty::Number,
                    optional: true,
                    variadic: false,
                },
            ],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("slice(list, start, end?)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "concat".into(),
            params: vec![ParamSig {
                name: Some("lists".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: true,
            }],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("concat(list, ...)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "sort".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("sort(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "reverse".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("reverse(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "join".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("separator".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::String,
            detail: Some("join(list, separator)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "split".into(),
            params: vec![
                ParamSig {
                    name: Some("text".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("separator".into()),
                    ty: Ty::String,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::List(Box::new(Ty::String)),
            detail: Some("split(text, separator)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "unique".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("unique(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "includes".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("value".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("includes(list, value)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "find".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("predicate".into()),
                    ty: Ty::Unknown, // lambda expr (current/index) in Notion DSL
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Unknown,
            detail: Some("find(list, predicate)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "findIndex".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("predicate".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Number,
            detail: Some("findIndex(list, predicate)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "filter".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("predicate".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("filter(list, predicate)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "some".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("predicate".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("some(list, predicate)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "every".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("predicate".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("every(list, predicate)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "map".into(),
            params: vec![
                ParamSig {
                    name: Some("list".into()),
                    ty: Ty::List(Box::new(Ty::Unknown)),
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("mapper".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("map(list, mapper)".into()),
            min_args: 2,
            category: FunctionCategory::List,
        },
        FunctionSig {
            name: "flat".into(),
            params: vec![ParamSig {
                name: Some("list".into()),
                ty: Ty::List(Box::new(Ty::Unknown)),
                optional: false,
                variadic: false,
            }],
            ret: Ty::List(Box::new(Ty::Unknown)),
            detail: Some("flat(list)".into()),
            min_args: 1,
            category: FunctionCategory::List,
        },
        // =========================
        // Special / Utility
        // =========================
        FunctionSig {
            name: "id".into(),
            params: vec![ParamSig {
                name: Some("page".into()),
                ty: Ty::Unknown, // if you have Ty::Page, use it here
                optional: true,
                variadic: false,
            }],
            ret: Ty::String,
            detail: Some("id(page?)".into()),
            min_args: 0,
            category: FunctionCategory::Special,
        },
        FunctionSig {
            name: "equal".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("equal(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Special,
        },
        FunctionSig {
            name: "unequal".into(),
            params: vec![
                ParamSig {
                    name: Some("a".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("b".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Boolean,
            detail: Some("unequal(a, b)".into()),
            min_args: 2,
            category: FunctionCategory::Special,
        },
        FunctionSig {
            name: "let".into(),
            // let(var, value, expr)
            params: vec![
                ParamSig {
                    name: Some("var".into()),
                    ty: Ty::Unknown, // identifier slot
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("value".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
                ParamSig {
                    name: Some("expr".into()),
                    ty: Ty::Unknown,
                    optional: false,
                    variadic: false,
                },
            ],
            ret: Ty::Unknown,
            detail: Some("let(var, value, expr)".into()),
            min_args: 3,
            category: FunctionCategory::Special,
        },
        FunctionSig {
            name: "lets".into(),
            // lets(a, v1, b, v2, ..., expr)
            params: vec![ParamSig {
                name: Some("args".into()),
                ty: Ty::Unknown,
                optional: false,
                variadic: true,
            }],
            ret: Ty::Unknown,
            detail: Some("lets(var1, value1, ..., expr)".into()),
            min_args: 3,
            category: FunctionCategory::Special,
        },
    ]
}

impl Context {
    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.ty.clone())
    }
}

pub fn analyze_expr(expr: &Expr, ctx: &Context) -> (Ty, Vec<Diagnostic>) {
    let mut diags = Vec::new();
    let ty = analyze_expr_inner(expr, ctx, &mut diags);
    (ty, diags)
}

fn lookup_function<'a>(ctx: &'a Context, name: &str) -> Option<&'a FunctionSig> {
    ctx.functions.iter().find(|f| f.name == name)
}

fn analyze_expr_inner(expr: &Expr, ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    match &expr.kind {
        ExprKind::Lit(lit) => match lit.kind {
            LitKind::Number => Ty::Number,
            LitKind::String => Ty::String,
            LitKind::Bool => Ty::Boolean,
        },
        ExprKind::Ident(_) => Ty::Unknown,
        ExprKind::Group { inner } => analyze_expr_inner(inner, ctx, diags),
        ExprKind::MemberCall {
            receiver,
            method,
            args,
        } => {
            // Phase 8: minimal typing. For `.if(cond, otherwise)` we treat it like:
            // `condition.if(then, else)` is treated like `if(condition, then, else)` (receiver is the `condition`).
            if method.text == "if" && args.len() == 2 {
                if lookup_function(ctx, "if").is_none() {
                    let _ = analyze_expr_inner(receiver, ctx, diags);
                    for arg in args {
                        let _ = analyze_expr_inner(arg, ctx, diags);
                    }
                    emit_error(diags, expr.span, "unknown function: if");
                    return Ty::Unknown;
                }

                let cond_ty = analyze_expr_inner(receiver, ctx, diags);
                let then_ty = analyze_expr_inner(&args[0], ctx, diags);
                let otherwise_ty = analyze_expr_inner(&args[1], ctx, diags);
                if cond_ty != Ty::Unknown && cond_ty != Ty::Boolean {
                    emit_error(diags, receiver.span, "if() condition must be boolean");
                }
                join_types(then_ty, otherwise_ty)
            } else {
                let _ = analyze_expr_inner(receiver, ctx, diags);
                for arg in args {
                    let _ = analyze_expr_inner(arg, ctx, diags);
                }
                Ty::Unknown
            }
        }
        ExprKind::Unary { op, expr } => {
            let inner_ty = analyze_expr_inner(expr, ctx, diags);
            match op.node {
                crate::ast::UnOpKind::Not => match inner_ty {
                    Ty::Boolean => Ty::Boolean,
                    _ => Ty::Unknown,
                },
                crate::ast::UnOpKind::Neg => match inner_ty {
                    Ty::Number => Ty::Number,
                    _ => Ty::Unknown,
                },
            }
        }
        ExprKind::Binary { op, left, right } => {
            let left_ty = analyze_expr_inner(left, ctx, diags);
            let right_ty = analyze_expr_inner(right, ctx, diags);
            use crate::ast::BinOpKind::*;
            match op.node {
                Plus | Minus | Star | Slash | Percent | Caret => {
                    if left_ty == Ty::Number && right_ty == Ty::Number {
                        Ty::Number
                    } else {
                        Ty::Unknown
                    }
                }
                AndAnd | OrOr => {
                    if left_ty == Ty::Boolean && right_ty == Ty::Boolean {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                Lt | Le | Ge | Gt => {
                    if left_ty != Ty::Unknown && right_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
                EqEq | Ne => {
                    if left_ty == right_ty && left_ty != Ty::Unknown {
                        Ty::Boolean
                    } else {
                        Ty::Unknown
                    }
                }
            }
        }
        ExprKind::Ternary {
            cond,
            then,
            otherwise,
        } => {
            let _ = analyze_expr_inner(cond, ctx, diags);
            let then_ty = analyze_expr_inner(then, ctx, diags);
            let otherwise_ty = analyze_expr_inner(otherwise, ctx, diags);
            join_types(then_ty, otherwise_ty)
        }
        ExprKind::Call { callee, args } => match callee.text.as_str() {
            "prop" => analyze_prop(expr, args, ctx, diags),
            name => {
                let Some(sig) = lookup_function(ctx, name) else {
                    for arg in args {
                        let _ = analyze_expr_inner(arg, ctx, diags);
                    }
                    emit_error(diags, expr.span, format!("unknown function: {}", name));
                    return Ty::Unknown;
                };

                match name {
                    "if" => analyze_if(expr, args, ctx, diags),
                    _ => analyze_call(expr, sig, args, ctx, diags),
                }
            }
        },
        ExprKind::Error => Ty::Unknown,
    }
}

fn analyze_call(
    expr: &Expr,
    sig: &FunctionSig,
    args: &[Expr],
    ctx: &Context,
    diags: &mut Vec<Diagnostic>,
) -> Ty {
    debug_assert!(
        sig.params
            .iter()
            .take(sig.params.len().saturating_sub(1))
            .all(|p| !p.variadic),
        "only the last param may be variadic"
    );

    let mut arg_tys = Vec::with_capacity(args.len());
    for arg in args {
        arg_tys.push(analyze_expr_inner(arg, ctx, diags));
    }

    if sig.is_variadic() {
        let required = sig.effective_min_args().max(sig.fixed_params_len());
        if args.len() < required {
            let plural = if required == 1 { "" } else { "s" };
            emit_error(
                diags,
                expr.span,
                format!(
                    "{}() expects at least {} argument{}",
                    sig.name, required, plural
                ),
            );
        }
    } else if args.len() != sig.params.len() {
        let expected = sig.params.len();
        let plural = if expected == 1 { "" } else { "s" };
        emit_error(
            diags,
            expr.span,
            format!(
                "{}() expects exactly {} argument{}",
                sig.name, expected, plural
            ),
        );
    }

    for (idx, (arg, ty)) in args.iter().zip(arg_tys.iter()).enumerate() {
        let Some(param) = sig.param_for_arg_index(idx) else {
            continue;
        };
        if !ty_accepts(&param.ty, ty) {
            if sig.name == "sum" {
                emit_error(diags, arg.span, "sum() expects number arguments");
            } else {
                emit_error(
                    diags,
                    arg.span,
                    format!(
                        "argument type mismatch: expected {:?}, got {:?}",
                        param.ty, ty
                    ),
                );
            }
        }
    }

    sig.ret.clone()
}

fn analyze_prop(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    for arg in args {
        let _ = analyze_expr_inner(arg, ctx, diags);
    }

    if args.len() != 1 {
        emit_error(diags, expr.span, "prop() expects exactly 1 argument");
        return Ty::Unknown;
    }

    let arg = &args[0];
    let name = match &arg.kind {
        ExprKind::Lit(lit) if lit.kind == LitKind::String => lit.symbol.text.as_str(),
        _ => {
            emit_error(diags, arg.span, "prop() expects a string literal argument");
            return Ty::Unknown;
        }
    };

    match ctx.lookup(name) {
        Some(ty) => ty,
        None => {
            emit_error(diags, arg.span, format!("Unknown property: {}", name));
            Ty::Unknown
        }
    }
}

fn analyze_if(expr: &Expr, args: &[Expr], ctx: &Context, diags: &mut Vec<Diagnostic>) -> Ty {
    if args.len() != 3 {
        for arg in args {
            let _ = analyze_expr_inner(arg, ctx, diags);
        }
        emit_error(diags, expr.span, "if() expects exactly 3 arguments");
        return Ty::Unknown;
    }

    let cond_ty = analyze_expr_inner(&args[0], ctx, diags);
    let then_ty = analyze_expr_inner(&args[1], ctx, diags);
    let otherwise_ty = analyze_expr_inner(&args[2], ctx, diags);

    if cond_ty != Ty::Unknown && cond_ty != Ty::Boolean {
        emit_error(diags, args[0].span, "if() condition must be boolean");
    }

    join_types(then_ty, otherwise_ty)
}

fn join_types(a: Ty, b: Ty) -> Ty {
    if a == Ty::Unknown || b == Ty::Unknown {
        Ty::Unknown
    } else if a == b {
        a
    } else {
        Ty::Unknown
    }
}

fn emit_error(diags: &mut Vec<Diagnostic>, span: Span, message: impl Into<String>) {
    diags.push(Diagnostic {
        kind: DiagnosticKind::Error,
        message: message.into(),
        span,
        labels: vec![],
        notes: vec![],
    });
}
