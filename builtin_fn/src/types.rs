use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier for a generic type parameter in [`Ty::Generic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GenericId(pub u32);

/// Describes the origin of an implicit lambda parameter binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LambdaParam {
    Current,
    ParamRef(String),
}

/// Formula type used by inference, validation, and editor tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Ty {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Unknown,
    Generic(GenericId),
    List(Box<Ty>),
    Union(Vec<Ty>),
    Fn {
        params: Vec<(LambdaParam, Ty)>,
        ret: Box<Ty>,
    },
    Ident(Box<Ty>),
}

impl Ty {
    fn precedence(&self) -> u8 {
        match self {
            Ty::Fn { .. } => 0,
            Ty::Union(_) => 1,
            Ty::List(_) => 2,
            Ty::Number
            | Ty::String
            | Ty::Boolean
            | Ty::Date
            | Ty::Null
            | Ty::Unknown
            | Ty::Generic(_)
            | Ty::Ident(_) => 3,
        }
    }

    fn fmt_with_prec(&self, f: &mut fmt::Formatter<'_>, parent_prec: u8) -> fmt::Result {
        let my_prec = self.precedence();
        let needs_parens = my_prec < parent_prec;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Ty::Number => f.write_str("number")?,
            Ty::String => f.write_str("string")?,
            Ty::Boolean => f.write_str("boolean")?,
            Ty::Date => f.write_str("date")?,
            Ty::Null => f.write_str("null")?,
            Ty::Unknown => f.write_str("unknown")?,
            Ty::Generic(id) => write!(f, "T{}", id.0)?,
            Ty::List(inner) => {
                inner.fmt_with_prec(f, my_prec)?;
                f.write_str("[]")?;
            }
            Ty::Union(members) => {
                for (idx, member) in members.iter().enumerate() {
                    if idx > 0 {
                        f.write_str(" | ")?;
                    }
                    member.fmt_with_prec(f, my_prec)?;
                }
            }
            Ty::Fn { params, ret } => {
                f.write_str("(")?;
                for (idx, (lp, lp_ty)) in params.iter().enumerate() {
                    if idx > 0 {
                        f.write_str(", ")?;
                    }
                    let name = match lp {
                        LambdaParam::Current => "current",
                        LambdaParam::ParamRef(name) => name.as_str(),
                    };
                    write!(f, "{name}: ")?;
                    lp_ty.fmt_with_prec(f, 0)?;
                }
                f.write_str(") -> ")?;
                ret.fmt_with_prec(f, 0)?;
            }
            Ty::Ident(inner) => {
                f.write_str("ident<")?;
                inner.fmt_with_prec(f, 0)?;
                f.write_str(">")?;
            }
        }

        if needs_parens {
            f.write_str(")")?;
        }
        Ok(())
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_with_prec(f, 0)
    }
}

/// Category bucket for builtin functions.
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
