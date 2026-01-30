macro_rules! p {
    ($name:literal, $ty:expr $(,)?) => {
        $crate::semantic::ParamSig {
            name: $name.into(),
            ty: $ty,
            optional: false,
        }
    };
}

macro_rules! opt {
    ($name:literal, $ty:expr $(,)?) => {
        $crate::semantic::ParamSig {
            name: $name.into(),
            ty: $ty,
            optional: true,
        }
    };
}

macro_rules! head {
    () => {
        Vec::<$crate::semantic::ParamSig>::new()
    };
    ($($param:expr),+ $(,)?) => {
        vec![$($param),+]
    };
}

macro_rules! repeat {
    () => {
        Vec::<$crate::semantic::ParamSig>::new()
    };
    ($($param:expr),+ $(,)?) => {
        vec![$($param),+]
    };
}

macro_rules! tail {
    () => {
        Vec::<$crate::semantic::ParamSig>::new()
    };
    ($($param:expr),+ $(,)?) => {
        vec![$($param),+]
    };
}

#[allow(unused_macros)]
macro_rules! shape {
    ($head:expr, $repeat:expr, $tail:expr $(,)?) => {
        $crate::semantic::ParamShape::new($head, $repeat, $tail)
    };
}

macro_rules! params {
    () => {
        $crate::semantic::ParamShape::new(vec![], vec![], vec![])
    };
    ($($param:expr),+ $(,)?) => {
        $crate::semantic::ParamShape::new(vec![$($param),+], vec![], vec![])
    };
}

macro_rules! repeat_params {
    ($head:expr, $repeat:expr, $tail:expr $(,)?) => {
        $crate::semantic::ParamShape::new($head, $repeat, $tail)
    };
}

macro_rules! repeat_params_with_tail {
    ($repeat:expr, $tail:expr $(,)?) => {
        $crate::semantic::ParamShape::new(vec![], $repeat, $tail)
    };
}

macro_rules! g {
    ($id:literal, Variant $(,)?) => {
        $crate::semantic::GenericParam {
            id: $crate::semantic::GenericId($id),
            kind: $crate::semantic::GenericParamKind::Variant,
        }
    };
    ($id:literal, Plain $(,)?) => {
        $crate::semantic::GenericParam {
            id: $crate::semantic::GenericId($id),
            kind: $crate::semantic::GenericParamKind::Plain,
        }
    };
}

macro_rules! generics {
    () => {
        Vec::<$crate::semantic::GenericParam>::new()
    };
    ($($param:expr),+ $(,)?) => {
        vec![$($param),+]
    };
}

macro_rules! func {
    ($category:expr, $detail:expr, $name:literal, $params:expr, $ret:expr $(,)?) => {
        $crate::semantic::FunctionSig::new_builtin($category, $detail, $name, $params, $ret, vec![])
    };
}

macro_rules! func_g {
    ($category:expr, $detail:expr, $generics:expr, $name:literal, $params:expr, $ret:expr $(,)?) => {
        $crate::semantic::FunctionSig::new_builtin(
            $category, $detail, $name, $params, $ret, $generics,
        )
    };
}
