//! japl-types: Core type system representation for JAPL.
//!
//! These are the *semantic* types used during type checking, distinct from
//! the *syntactic* TypeExpr in japl-ast.
//!
//! Types are interned via `TypeInterner` for O(1) equality checks and
//! minimal memory duplication.

use smol_str::SmolStr;
use std::collections::HashMap;
use std::fmt;

/// Unique ID for a definition in the program (function, type, constructor, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// Interned type ID for fast comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u32);

/// A type variable used in unification and polymorphism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar {
    pub id: u32,
    pub kind: TypeVarKind,
}

/// What kind of type variable this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeVarKind {
    /// Unification variable: can be solved.
    Unification,
    /// Rigid variable: bound by forall, cannot be solved.
    Rigid,
    /// Skolem variable: created during subsumption checks.
    Skolem,
}

/// The core type representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    // -- Primitives --
    Int,
    Float,
    Float32,
    Bool,
    Char,
    String,
    Bytes,
    Unit,
    Never,

    /// Type variable (unification variable or rigid).
    Var(TypeVar),

    /// Named type constructor applied to arguments: `List[Int]`.
    App {
        constructor: DefId,
        args: Vec<TypeId>,
    },

    /// Function type: `(A, B) -> C with E1, E2`.
    Fn {
        params: Vec<TypeId>,
        return_type: TypeId,
        effects: EffectRow,
    },

    /// Record type with row polymorphism.
    Record {
        fields: Vec<(SmolStr, TypeId)>,
        /// None = closed record; Some(var) = open record with row variable.
        row_var: Option<TypeVar>,
    },

    /// Tuple type.
    Tuple(Vec<TypeId>),

    /// Owned resource type.
    Owned(TypeId),

    /// Borrowed reference type.
    Ref(TypeId),

    /// Forall (polymorphic type scheme).
    Forall {
        vars: Vec<TypeVar>,
        constraints: Vec<TraitConstraint>,
        body: TypeId,
    },

    /// Type error placeholder (allows continued checking).
    Error,
}

/// A row of effects, modeled as a set with an optional row variable.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectRow {
    /// Known effects in the row.
    pub effects: Vec<Effect>,
    /// Optional tail variable for open effect rows.
    pub row_var: Option<TypeVar>,
}

impl EffectRow {
    pub fn pure() -> Self {
        EffectRow {
            effects: Vec::new(),
            row_var: None,
        }
    }

    pub fn with_effects(effects: Vec<Effect>) -> Self {
        EffectRow {
            effects,
            row_var: None,
        }
    }

    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() && self.row_var.is_none()
    }

    pub fn contains(&self, effect: &Effect) -> bool {
        self.effects.iter().any(|e| e == effect)
    }

    /// Merge two effect rows by taking the union of their concrete effects.
    pub fn union(&self, other: &EffectRow) -> EffectRow {
        let mut effects = self.effects.clone();
        for eff in &other.effects {
            if !effects.contains(eff) {
                effects.push(eff.clone());
            }
        }
        let row_var = self.row_var.or(other.row_var);
        EffectRow { effects, row_var }
    }
}

/// Individual effect types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    Pure,
    Io,
    Async,
    Net,
    State(TypeId),
    Process(TypeId),
    Fail(TypeId),
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Pure => write!(f, "Pure"),
            Effect::Io => write!(f, "Io"),
            Effect::Async => write!(f, "Async"),
            Effect::Net => write!(f, "Net"),
            Effect::State(t) => write!(f, "State[{:?}]", t),
            Effect::Process(t) => write!(f, "Process[{:?}]", t),
            Effect::Fail(t) => write!(f, "Fail[{:?}]", t),
        }
    }
}

/// A trait constraint: `Eq[Int]`, `Show[a]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitConstraint {
    pub trait_id: DefId,
    pub args: Vec<TypeId>,
}

/// A kind -- the "type of types".
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// The kind of concrete types: `*`.
    Type,
    /// Higher-kinded: `* -> *`.
    Arrow(Box<Kind>, Box<Kind>),
    /// Effect row kind.
    Effect,
}

// ---------------------------------------------------------------------------
// Type Interner
// ---------------------------------------------------------------------------

/// Type interner: deduplicates types and provides O(1) equality via TypeId.
pub struct TypeInterner {
    types: Vec<Type>,
    map: HashMap<Type, TypeId>,
}

impl TypeInterner {
    pub fn new() -> Self {
        Self {
            types: Vec::new(),
            map: HashMap::new(),
        }
    }

    /// Intern a type, returning its unique TypeId.
    pub fn intern(&mut self, ty: Type) -> TypeId {
        if let Some(&id) = self.map.get(&ty) {
            return id;
        }
        let id = TypeId(self.types.len() as u32);
        self.map.insert(ty.clone(), id);
        self.types.push(ty);
        id
    }

    /// Resolve a TypeId to its Type.
    pub fn resolve(&self, id: TypeId) -> &Type {
        &self.types[id.0 as usize]
    }

    /// Intern common primitive types and return their IDs.
    pub fn intern_primitives(&mut self) -> PrimitiveTypes {
        PrimitiveTypes {
            int: self.intern(Type::Int),
            float: self.intern(Type::Float),
            float32: self.intern(Type::Float32),
            bool_ty: self.intern(Type::Bool),
            char_ty: self.intern(Type::Char),
            string: self.intern(Type::String),
            bytes: self.intern(Type::Bytes),
            unit: self.intern(Type::Unit),
            never: self.intern(Type::Never),
            error: self.intern(Type::Error),
        }
    }

    /// Return the number of interned types.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }
}

impl Default for TypeInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-interned IDs for primitive types.
#[derive(Debug, Clone, Copy)]
pub struct PrimitiveTypes {
    pub int: TypeId,
    pub float: TypeId,
    pub float32: TypeId,
    pub bool_ty: TypeId,
    pub char_ty: TypeId,
    pub string: TypeId,
    pub bytes: TypeId,
    pub unit: TypeId,
    pub never: TypeId,
    pub error: TypeId,
}

// ---------------------------------------------------------------------------
// Display utilities
// ---------------------------------------------------------------------------

/// Pretty-print a type given access to the interner.
pub fn display_type(id: TypeId, interner: &TypeInterner) -> String {
    match interner.resolve(id) {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Float32 => "Float32".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::Char => "Char".to_string(),
        Type::String => "String".to_string(),
        Type::Bytes => "Bytes".to_string(),
        Type::Unit => "Unit".to_string(),
        Type::Never => "Never".to_string(),
        Type::Var(v) => format!("?{}", v.id),
        Type::App { constructor, args } => {
            if args.is_empty() {
                format!("T{}", constructor.0)
            } else {
                let args_str: Vec<_> = args.iter().map(|a| display_type(*a, interner)).collect();
                format!("T{}[{}]", constructor.0, args_str.join(", "))
            }
        }
        Type::Fn {
            params,
            return_type,
            effects,
        } => {
            let params_str: Vec<_> = params.iter().map(|p| display_type(*p, interner)).collect();
            let ret = display_type(*return_type, interner);
            let eff = if effects.is_pure() {
                String::new()
            } else {
                let effs: Vec<_> = effects.effects.iter().map(|e| format!("{}", e)).collect();
                format!(" with {}", effs.join(", "))
            };
            format!("fn({}) -> {}{}", params_str.join(", "), ret, eff)
        }
        Type::Record { fields, row_var } => {
            let fields_str: Vec<_> = fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, display_type(*t, interner)))
                .collect();
            match row_var {
                Some(v) => format!("{{ {} | ?{} }}", fields_str.join(", "), v.id),
                None => format!("{{ {} }}", fields_str.join(", ")),
            }
        }
        Type::Tuple(elems) => {
            let elems_str: Vec<_> = elems.iter().map(|e| display_type(*e, interner)).collect();
            format!("({})", elems_str.join(", "))
        }
        Type::Owned(inner) => format!("own {}", display_type(*inner, interner)),
        Type::Ref(inner) => format!("ref {}", display_type(*inner, interner)),
        Type::Forall { vars, body, .. } => {
            let vars_str: Vec<_> = vars.iter().map(|v| format!("?{}", v.id)).collect();
            format!(
                "forall {}. {}",
                vars_str.join(" "),
                display_type(*body, interner)
            )
        }
        Type::Error => "<error>".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Trait and type definition info
// ---------------------------------------------------------------------------

/// Information about a trait definition.
#[derive(Debug, Clone)]
pub struct TraitInfo {
    pub id: DefId,
    pub name: SmolStr,
    pub params: Vec<TypeVar>,
    pub supertraits: Vec<TraitConstraint>,
    pub methods: HashMap<SmolStr, TypeId>,
}

/// Information about a trait implementation.
#[derive(Debug, Clone)]
pub struct ImplInfo {
    pub trait_id: DefId,
    pub type_args: Vec<TypeId>,
    pub methods: HashMap<SmolStr, DefId>,
}

/// Information about a type definition (ADT).
#[derive(Debug, Clone)]
pub struct TypeDefInfo {
    pub id: DefId,
    pub name: SmolStr,
    pub type_params: Vec<TypeVar>,
    pub constructors: Vec<ConstructorInfo>,
}

/// Information about a variant constructor.
#[derive(Debug, Clone)]
pub struct ConstructorInfo {
    pub id: DefId,
    pub name: SmolStr,
    pub type_id: DefId,
    pub field_types: Vec<TypeId>,
    pub result_type: TypeId,
}
