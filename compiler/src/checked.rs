//! What the checker produces for codegen. **Shared contract (draft).**
//!
//! The AST is left untouched. Instead, the checker records its findings in
//! side tables keyed by the [`Span`] of the AST node they're about. Spans
//! are unique per node kind, so they work as node IDs.

use crate::debug_table::{DebugTable, TypeName};
use crate::span::Span;
use std::collections::HashMap;

pub type FnId = u32;
pub type VarId = u32;
pub type ScopeId = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Bool,
    /// The "type" of a function with no `-> type`. Not a value: you can't
    /// store it in a variable or pass it to a function.
    Unit,
}

impl Type {
    /// How the type is written in source, for error messages.
    pub fn name(self) -> &'static str {
        match self {
            Type::Int => "int",
            Type::Bool => "bool",
            Type::Unit => "nothing",
        }
    }

    /// The debug-table form. `None` for `Unit`, which never lives in a variable.
    pub fn type_name(self) -> Option<TypeName> {
        match self {
            Type::Int => Some(TypeName::Int),
            Type::Bool => Some(TypeName::Bool),
            Type::Unit => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FnSig {
    pub name: String,
    pub params: Vec<Type>,
    /// `Unit` when there's no `-> type`.
    pub ret: Type,
}

/// What a call expression calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Callee {
    Function(FnId),
    /// The built-in `print`, with the type of its argument (so codegen can
    /// pick `print` or `print_bool`).
    Print(Type),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// Indexed by `FnId`. Function `i` is `program.functions[i]`.
    pub functions: Vec<FnSig>,
    /// Every identifier that names a variable (the name in a `let`, a
    /// parameter name, a use in an expression, an assignment target), keyed
    /// by the identifier's span.
    pub vars: HashMap<Span, VarId>,
    /// Every call, keyed by the span of the callee's name.
    pub calls: HashMap<Span, Callee>,
    /// Every block, keyed by the block's span.
    pub scopes: HashMap<Span, ScopeId>,
    /// Type of every expression, keyed by the expression's span.
    pub types: HashMap<Span, Type>,
    /// `functions`, `vars` and `scopes` filled in; `steps` is left for codegen.
    pub debug: DebugTable,
}
