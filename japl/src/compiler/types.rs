use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Byte,
    Unit,
    Never,
    Named(std::string::String, Vec<Type>),
    Fn(Vec<Type>, Box<Type>),
    Record(BTreeMap<std::string::String, Type>),
    Tuple(Vec<Type>),
    Var(u32), // unification variable
    TypeParam(std::string::String), // generic type parameter (e.g. T)
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::String => write!(f, "String"),
            Type::Bool => write!(f, "Bool"),
            Type::Byte => write!(f, "Byte"),
            Type::Unit => write!(f, "Unit"),
            Type::Never => write!(f, "Never"),
            Type::Named(name, params) => {
                write!(f, "{}", name)?;
                if !params.is_empty() {
                    write!(f, "<")?;
                    for (i, p) in params.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", p)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            Type::Fn(params, ret) => {
                write!(f, "fn(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            Type::Record(fields) => {
                write!(f, "{{")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, "}}")
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Var(id) => write!(f, "?{}", id),
            Type::TypeParam(name) => write!(f, "{}", name),
        }
    }
}
