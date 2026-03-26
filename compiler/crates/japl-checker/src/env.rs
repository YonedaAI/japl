//! Type environment: tracks bindings, type definitions, traits, and constructors.

use japl_types::{
    DefId, TypeId,
    TraitInfo, ImplInfo, TypeDefInfo, ConstructorInfo,
};
use smol_str::SmolStr;
use std::collections::HashMap;

/// The type environment used during type checking.
///
/// Maintains scoped bindings for variables, type definitions,
/// trait definitions, implementations, and data constructors.
pub struct TypeEnv {
    /// Term-level variable bindings: name -> TypeId.
    /// Organized as a scope stack for lexical scoping.
    scopes: Vec<HashMap<SmolStr, TypeId>>,

    /// Type definitions by name.
    pub type_defs: HashMap<SmolStr, TypeDefInfo>,

    /// Trait definitions by name.
    pub traits: HashMap<SmolStr, TraitInfo>,

    /// All known trait implementations.
    pub impls: Vec<ImplInfo>,

    /// Data constructors by name -> (type_def DefId, constructor info).
    pub constructors: HashMap<SmolStr, ConstructorInfo>,

    /// Next available DefId.
    next_def_id: u32,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            scopes: vec![HashMap::new()],
            type_defs: HashMap::new(),
            traits: HashMap::new(),
            impls: Vec::new(),
            constructors: HashMap::new(),
            next_def_id: 0,
        }
    }

    /// Allocate a fresh DefId.
    pub fn fresh_def_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    // -- Scope management --

    /// Push a new scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the innermost scope.
    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Bind a variable in the current scope.
    pub fn bind(&mut self, name: SmolStr, ty: TypeId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    /// Look up a variable by name, searching from innermost to outermost scope.
    pub fn lookup(&self, name: &str) -> Option<TypeId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    // -- Type definition management --

    /// Register a type definition.
    pub fn register_type_def(&mut self, info: TypeDefInfo) {
        // Also register each constructor.
        for ctor in &info.constructors {
            self.constructors.insert(ctor.name.clone(), ctor.clone());
        }
        self.type_defs.insert(info.name.clone(), info);
    }

    /// Look up a type definition by name.
    pub fn lookup_type_def(&self, name: &str) -> Option<&TypeDefInfo> {
        self.type_defs.get(name)
    }

    /// Look up a constructor by name.
    pub fn lookup_constructor(&self, name: &str) -> Option<&ConstructorInfo> {
        self.constructors.get(name)
    }

    // -- Trait management --

    /// Register a trait definition.
    pub fn register_trait(&mut self, info: TraitInfo) {
        self.traits.insert(info.name.clone(), info);
    }

    /// Register a trait implementation.
    pub fn register_impl(&mut self, info: ImplInfo) {
        self.impls.push(info);
    }

    /// Look up a trait by name.
    pub fn lookup_trait(&self, name: &str) -> Option<&TraitInfo> {
        self.traits.get(name)
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
