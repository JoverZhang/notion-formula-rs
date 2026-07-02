use super::lexer::{Token, TokenKind, lex};
use super::{BuiltinSigParseError, BuiltinSigParseErrorKind, GenericKindRegistry};
use crate::{
    FunctionCategory, FunctionSig, GenericId, GenericParam, GenericParamKind, LambdaParam,
    ParamShape, ParamSig, Ty,
};
use std::collections::HashMap;

pub(crate) fn parse_signature(
    category: FunctionCategory,
    text: &str,
    registry: &GenericKindRegistry,
) -> Result<FunctionSig, BuiltinSigParseError> {
    let tokens = lex(text)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        _registry: registry,
    };
    let parsed = parser.parse_signature()?;
    parser.expect_eof()?;
    lower_signature(category, parsed, registry)
}

#[derive(Debug, Clone)]
struct ParsedSig {
    name: String,
    generics: Vec<ParsedGeneric>,
    params: Vec<ParsedParamItem>,
    ret: ParsedTy,
}

#[derive(Debug, Clone)]
struct ParsedGeneric {
    name: String,
    kind_name: Option<String>,
    position: usize,
}

#[derive(Debug, Clone)]
enum ParsedParamItem {
    Param(ParsedParam),
    Rest(ParsedParam),
    Ellipsis { position: usize },
}

#[derive(Debug, Clone)]
struct ParsedParam {
    name: String,
    ty: ParsedTy,
    optional: bool,
    position: usize,
}

#[derive(Debug, Clone)]
enum ParsedTy {
    Number,
    String,
    Boolean,
    Date,
    Null,
    Any,
    GenericRef {
        name: String,
        position: usize,
    },
    List(Box<ParsedTy>),
    Union(Vec<ParsedTy>),
    Fn {
        params: Vec<(String, ParsedTy)>,
        ret: Box<ParsedTy>,
    },
    Ident(Box<ParsedTy>),
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    _registry: &'a GenericKindRegistry,
}

impl<'a> Parser<'a> {
    fn parse_signature(&mut self) -> Result<ParsedSig, BuiltinSigParseError> {
        let name = self.expect_ident("function name")?;
        let generics = if self.at(&TokenKind::Lt) {
            self.parse_generics()?
        } else {
            Vec::new()
        };

        self.expect_simple(TokenKind::LParen)?;
        let params = self.parse_params()?;
        self.expect_simple(TokenKind::RParen)?;

        if !self.consume(&TokenKind::Arrow) {
            return Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::MissingArrow,
                self.current().position,
            ));
        }

        let ret = self.parse_type()?;

        Ok(ParsedSig {
            name,
            generics,
            params,
            ret,
        })
    }

    fn parse_generics(&mut self) -> Result<Vec<ParsedGeneric>, BuiltinSigParseError> {
        self.expect_simple(TokenKind::Lt)?;
        let mut out = Vec::new();
        let mut seen = HashMap::<String, usize>::new();

        loop {
            let position = self.current().position;
            let name = self.expect_ident("generic name")?;
            if let Some(prev) = seen.insert(name.clone(), position) {
                let _ = prev;
                return Err(BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::DuplicateGenericName { name },
                    position,
                ));
            }

            let kind_name = if self.consume(&TokenKind::Colon) {
                Some(self.expect_ident("generic kind")?)
            } else {
                None
            };

            out.push(ParsedGeneric {
                name,
                kind_name,
                position,
            });

            if self.consume(&TokenKind::Comma) {
                continue;
            }
            self.expect_simple(TokenKind::Gt)?;
            break;
        }

        Ok(out)
    }

    fn parse_params(&mut self) -> Result<Vec<ParsedParamItem>, BuiltinSigParseError> {
        let mut out = Vec::new();
        if self.at(&TokenKind::RParen) {
            return Ok(out);
        }

        loop {
            out.push(self.parse_param_item()?);

            if !self.consume(&TokenKind::Comma) {
                break;
            }
            if self.at(&TokenKind::RParen) {
                return Err(BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::MalformedParameter,
                    self.current().position,
                ));
            }
        }

        Ok(out)
    }

    fn parse_param_item(&mut self) -> Result<ParsedParamItem, BuiltinSigParseError> {
        if self.consume(&TokenKind::Ellipsis) {
            let position = self.previous().position;
            if self.at_ident() {
                let name = self.expect_ident("rest parameter name")?;
                self.expect_simple(TokenKind::Colon)?;
                let ty = self.parse_type()?;
                return Ok(ParsedParamItem::Rest(ParsedParam {
                    name,
                    ty,
                    optional: false,
                    position,
                }));
            }
            return Ok(ParsedParamItem::Ellipsis { position });
        }

        let position = self.current().position;
        let name = self.expect_ident("parameter name")?;
        let optional = self.consume(&TokenKind::Question);
        self.expect_simple(TokenKind::Colon)?;
        let ty = self.parse_type()?;
        Ok(ParsedParamItem::Param(ParsedParam {
            name,
            ty,
            optional,
            position,
        }))
    }

    fn parse_type(&mut self) -> Result<ParsedTy, BuiltinSigParseError> {
        let mut members = vec![self.parse_postfix_type()?];
        while self.consume(&TokenKind::Pipe) {
            members.push(self.parse_postfix_type()?);
        }

        if members.len() == 1 {
            Ok(members.remove(0))
        } else {
            Ok(ParsedTy::Union(members))
        }
    }

    fn parse_postfix_type(&mut self) -> Result<ParsedTy, BuiltinSigParseError> {
        let mut ty = self.parse_primary_type()?;
        while self.consume(&TokenKind::LBracket) {
            self.expect_simple(TokenKind::RBracket)?;
            ty = ParsedTy::List(Box::new(ty));
        }
        Ok(ty)
    }

    fn parse_primary_type(&mut self) -> Result<ParsedTy, BuiltinSigParseError> {
        if self.at(&TokenKind::LParen) {
            if let Some(fn_ty) = self.try_parse_fn_type()? {
                return Ok(fn_ty);
            }

            self.expect_simple(TokenKind::LParen)?;
            let inner = self.parse_type()?;
            self.expect_simple(TokenKind::RParen)?;
            return Ok(inner);
        }

        let position = self.current().position;
        let ident = self.expect_ident("type")?;
        let mut ty = match ident.as_str() {
            "number" => ParsedTy::Number,
            "string" => ParsedTy::String,
            "boolean" => ParsedTy::Boolean,
            "date" => ParsedTy::Date,
            "null" => ParsedTy::Null,
            "any" => ParsedTy::Any,
            "Ident" | "ident" => {
                self.expect_simple(TokenKind::Lt)?;
                let inner = self.parse_type()?;
                self.expect_simple(TokenKind::Gt)?;
                ParsedTy::Ident(Box::new(inner))
            }
            _ => {
                if self.at(&TokenKind::Lt) {
                    return Err(BuiltinSigParseError::new(
                        BuiltinSigParseErrorKind::MalformedType,
                        position,
                    ));
                }
                ParsedTy::GenericRef {
                    name: ident,
                    position,
                }
            }
        };

        if matches!(ty, ParsedTy::Ident(_)) {
            return Ok(ty);
        }

        while self.consume(&TokenKind::LBracket) {
            self.expect_simple(TokenKind::RBracket)?;
            ty = ParsedTy::List(Box::new(ty));
        }

        Ok(ty)
    }

    fn try_parse_fn_type(&mut self) -> Result<Option<ParsedTy>, BuiltinSigParseError> {
        let mark = self.pos;

        self.expect_simple(TokenKind::LParen)?;
        let mut params = Vec::<(String, ParsedTy)>::new();

        if !self.consume(&TokenKind::RParen) {
            loop {
                if !self.at_ident() {
                    self.pos = mark;
                    return Ok(None);
                }

                let name = self.expect_ident("lambda parameter")?;
                if !self.consume(&TokenKind::Colon) {
                    self.pos = mark;
                    return Ok(None);
                }
                let ty = self.parse_type()?;
                params.push((name, ty));

                if self.consume(&TokenKind::Comma) {
                    continue;
                }

                self.expect_simple(TokenKind::RParen)?;
                break;
            }
        }

        if !self.consume(&TokenKind::Arrow) {
            self.pos = mark;
            return Ok(None);
        }

        let ret = self.parse_type()?;
        Ok(Some(ParsedTy::Fn {
            params,
            ret: Box::new(ret),
        }))
    }

    fn expect_ident(&mut self, expected: &str) -> Result<String, BuiltinSigParseError> {
        match &self.current().kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.bump();
                Ok(name)
            }
            other => Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::UnexpectedToken {
                    expected: expected.to_string(),
                    found: other.display_name(),
                },
                self.current().position,
            )),
        }
    }

    fn expect_simple(&mut self, expected: TokenKind) -> Result<(), BuiltinSigParseError> {
        if self.consume(&expected) {
            return Ok(());
        }
        Err(BuiltinSigParseError::new(
            BuiltinSigParseErrorKind::UnexpectedToken {
                expected: expected.display_name(),
                found: self.current().kind.display_name(),
            },
            self.current().position,
        ))
    }

    fn expect_eof(&self) -> Result<(), BuiltinSigParseError> {
        if matches!(self.current().kind, TokenKind::Eof) {
            return Ok(());
        }
        Err(BuiltinSigParseError::new(
            BuiltinSigParseErrorKind::UnexpectedToken {
                expected: "end of input".into(),
                found: self.current().kind.display_name(),
            },
            self.current().position,
        ))
    }

    fn consume(&mut self, expected: &TokenKind) -> bool {
        if self.at(expected) {
            self.bump();
            return true;
        }
        false
    }

    fn at(&self, expected: &TokenKind) -> bool {
        match (&self.current().kind, expected) {
            (TokenKind::Ident(_), TokenKind::Ident(_)) => true,
            (a, b) => a == b,
        }
    }

    fn at_ident(&self) -> bool {
        matches!(self.current().kind, TokenKind::Ident(_))
    }

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.pos.saturating_sub(1)]
    }

    fn bump(&mut self) {
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
    }
}

fn lower_signature(
    category: FunctionCategory,
    parsed: ParsedSig,
    registry: &GenericKindRegistry,
) -> Result<FunctionSig, BuiltinSigParseError> {
    let mut explicit_generics = HashMap::<String, GenericId>::new();
    let mut generics = Vec::<GenericParam>::new();

    for generic in &parsed.generics {
        let kind = match &generic.kind_name {
            Some(kind_name) => registry.resolve(kind_name).ok_or_else(|| {
                BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::UnknownGenericKind {
                        name: kind_name.clone(),
                    },
                    generic.position,
                )
            })?,
            None => GenericParamKind::Plain,
        };
        let id = GenericId(generics.len() as u32);
        explicit_generics.insert(generic.name.clone(), id);
        generics.push(GenericParam { id, kind });
    }

    let mut lower = LowerCtx {
        explicit_generics,
        generics,
        any_generic_id: None,
    };

    let params = lower_param_shape(parsed.params, &mut lower)?;
    let ret = lower.lower_ty(&parsed.ret)?;
    let detail = canonical_detail(&parsed.name, &params.detail_parts);

    let mut sig = FunctionSig::new_builtin(
        category,
        detail,
        parsed.name,
        params.shape,
        ret,
        lower.generics,
    );
    sig.params.repeat_min_groups = params.repeat_min_groups;
    Ok(sig)
}

struct LowerCtx {
    explicit_generics: HashMap<String, GenericId>,
    generics: Vec<GenericParam>,
    any_generic_id: Option<GenericId>,
}

impl LowerCtx {
    fn lower_ty(&mut self, parsed: &ParsedTy) -> Result<Ty, BuiltinSigParseError> {
        match parsed {
            ParsedTy::Number => Ok(Ty::Number),
            ParsedTy::String => Ok(Ty::String),
            ParsedTy::Boolean => Ok(Ty::Boolean),
            ParsedTy::Date => Ok(Ty::Date),
            ParsedTy::Null => Ok(Ty::Null),
            ParsedTy::Any => Ok(Ty::Generic(self.any_generic())),
            ParsedTy::GenericRef { name, position } => self
                .explicit_generics
                .get(name)
                .copied()
                .map(Ty::Generic)
                .ok_or_else(|| {
                    BuiltinSigParseError::new(
                        BuiltinSigParseErrorKind::UnknownGenericReference { name: name.clone() },
                        *position,
                    )
                }),
            ParsedTy::List(inner) => Ok(Ty::List(Box::new(self.lower_ty(inner)?))),
            ParsedTy::Union(members) => {
                let mut out = Vec::with_capacity(members.len());
                for member in members {
                    out.push(self.lower_ty(member)?);
                }
                Ok(Ty::Union(out))
            }
            ParsedTy::Fn { params, ret } => {
                let mut lowered_params = Vec::with_capacity(params.len());
                for (name, ty) in params {
                    let lambda_param = if name == "current" {
                        LambdaParam::Current
                    } else {
                        LambdaParam::ParamRef(name.clone())
                    };
                    lowered_params.push((lambda_param, self.lower_ty(ty)?));
                }
                Ok(Ty::Fn {
                    params: lowered_params,
                    ret: Box::new(self.lower_ty(ret)?),
                })
            }
            ParsedTy::Ident(inner) => Ok(Ty::Ident(Box::new(self.lower_ty(inner)?))),
        }
    }

    fn any_generic(&mut self) -> GenericId {
        if let Some(id) = self.any_generic_id {
            return id;
        }

        let id = GenericId(self.generics.len() as u32);
        self.generics.push(GenericParam {
            id,
            kind: GenericParamKind::Plain,
        });
        self.any_generic_id = Some(id);
        id
    }
}

struct LoweredShape {
    shape: ParamShape,
    detail_parts: Vec<String>,
    repeat_min_groups: usize,
}

fn lower_param_shape(
    items: Vec<ParsedParamItem>,
    lower: &mut LowerCtx,
) -> Result<LoweredShape, BuiltinSigParseError> {
    let mut detail_parts = Vec::<String>::new();
    let mut lowered_items = Vec::<LoweredParamItem>::new();
    let mut ellipsis_index = None;
    let mut rest_index = None;

    for item in items {
        match item {
            ParsedParamItem::Ellipsis { position } => {
                detail_parts.push("...".into());
                if ellipsis_index.replace(lowered_items.len()).is_some() {
                    return Err(BuiltinSigParseError::new(
                        BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                        position,
                    ));
                }
                lowered_items.push(LoweredParamItem::Ellipsis { position });
            }
            ParsedParamItem::Param(param) => {
                detail_parts.push(if param.optional {
                    format!("{}?", param.name)
                } else {
                    param.name.clone()
                });
                lowered_items.push(LoweredParamItem::Param(lower_param(param, lower)?));
            }
            ParsedParamItem::Rest(param) => {
                detail_parts.push(format!("...{}", param.name));
                if rest_index.replace(lowered_items.len()).is_some() {
                    return Err(BuiltinSigParseError::new(
                        BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                        param.position,
                    ));
                }
                lowered_items.push(LoweredParamItem::Rest {
                    param: lower_param(param, lower)?,
                });
            }
        }
    }

    if ellipsis_index.is_some() && rest_index.is_some() {
        let position = lowered_items
            .iter()
            .find_map(|item| match item {
                LoweredParamItem::Ellipsis { position } => Some(*position),
                LoweredParamItem::Rest { param } => Some(param.position),
                LoweredParamItem::Param(_) => None,
            })
            .unwrap_or(0);
        return Err(BuiltinSigParseError::new(
            BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
            position,
        ));
    }

    if let Some(rest_idx) = rest_index {
        if rest_idx + 1 != lowered_items.len() {
            let position = match &lowered_items[rest_idx] {
                LoweredParamItem::Rest { param } => param.position,
                _ => 0,
            };
            return Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                position,
            ));
        }

        let mut head = Vec::<ParamSig>::new();
        let mut repeat = Vec::<ParamSig>::new();
        for item in lowered_items {
            match item {
                LoweredParamItem::Param(param) => head.push(param.sig),
                LoweredParamItem::Rest { param } => {
                    let Ty::List(inner) = param.sig.ty else {
                        return Err(BuiltinSigParseError::new(
                            BuiltinSigParseErrorKind::RestParamMustUseListType,
                            param.position,
                        ));
                    };
                    repeat.push(ParamSig {
                        name: param.sig.name,
                        ty: *inner,
                        optional: false,
                    });
                }
                LoweredParamItem::Ellipsis { position } => {
                    return Err(BuiltinSigParseError::new(
                        BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                        position,
                    ));
                }
            }
        }

        let shape = ParamShape::new(head, repeat, vec![]).with_repeat_min_groups(0);
        return Ok(LoweredShape {
            shape,
            detail_parts,
            repeat_min_groups: 0,
        });
    }

    if let Some(ellipsis_idx) = ellipsis_index {
        if ellipsis_idx == 0 {
            return Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                lowered_items[ellipsis_idx].position(),
            ));
        }

        let before = lowered_items[..ellipsis_idx]
            .iter()
            .map(LoweredParamItem::as_param)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                    lowered_items[ellipsis_idx].position(),
                )
            })?;

        let after = lowered_items[ellipsis_idx + 1..]
            .iter()
            .map(LoweredParamItem::as_param)
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                BuiltinSigParseError::new(
                    BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                    lowered_items[ellipsis_idx].position(),
                )
            })?;

        let shape = if after.is_empty() && before.len() == 2 && before[1].name.ends_with('N') {
            ParamShape::new(vec![before[0].clone()], vec![before[1].clone()], vec![])
        } else {
            ParamShape::new(vec![], before, after)
        };

        return Ok(LoweredShape {
            shape,
            detail_parts,
            repeat_min_groups: 1,
        });
    }

    let head = lowered_items
        .into_iter()
        .map(|item| match item {
            LoweredParamItem::Param(param) => Ok(param.sig),
            LoweredParamItem::Ellipsis { position } => Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                position,
            )),
            LoweredParamItem::Rest { param } => Err(BuiltinSigParseError::new(
                BuiltinSigParseErrorKind::InvalidRepeatGroupPlacement,
                param.position,
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(LoweredShape {
        shape: ParamShape::new(head, vec![], vec![]),
        detail_parts,
        repeat_min_groups: 1,
    })
}

#[derive(Debug, Clone)]
struct LoweredParam {
    sig: ParamSig,
    position: usize,
}

#[derive(Debug, Clone)]
enum LoweredParamItem {
    Param(LoweredParam),
    Rest { param: LoweredParam },
    Ellipsis { position: usize },
}

impl LoweredParamItem {
    fn as_param(&self) -> Option<ParamSig> {
        match self {
            LoweredParamItem::Param(param) => Some(param.sig.clone()),
            _ => None,
        }
    }

    fn position(&self) -> usize {
        match self {
            LoweredParamItem::Param(param) | LoweredParamItem::Rest { param } => param.position,
            LoweredParamItem::Ellipsis { position } => *position,
        }
    }
}

fn lower_param(
    param: ParsedParam,
    lower: &mut LowerCtx,
) -> Result<LoweredParam, BuiltinSigParseError> {
    Ok(LoweredParam {
        sig: ParamSig {
            name: param.name,
            ty: lower.lower_ty(&param.ty)?,
            optional: param.optional,
        },
        position: param.position,
    })
}

fn canonical_detail(name: &str, params: &[String]) -> String {
    format!("{name}({})", params.join(", "))
}
