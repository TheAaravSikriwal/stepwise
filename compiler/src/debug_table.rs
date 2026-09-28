//! Metadata that lets the debugger turn trace events back into source code.
//!
//! Trace events only contain numbers (`line(3)`, `assign(7, 1, 2)`). This table
//! says what those numbers mean. IDs are indices into the vectors, assigned by
//! the resolver (variables, scopes, functions) and codegen (step spans).

use crate::span::Span;
use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DebugTable {
    /// Indexed by `fn_id`, as passed to `call(fn_id)`.
    pub functions: Vec<FunctionInfo>,
    /// Indexed by `var_id`, as passed to `declare` / `assign`.
    pub vars: Vec<VarInfo>,
    /// Indexed by `scope_id`, as passed to `scope_exit`.
    pub scopes: Vec<ScopeInfo>,
    /// Indexed by `span_id`, as passed to `line(span_id)`: the source range
    /// highlighted while that step is current.
    pub steps: Vec<Span>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FunctionInfo {
    pub name: String,
    /// The function's name in its declaration.
    pub span: Span,
    /// What it gives back, for showing its return value; `None` if nothing.
    pub returns: Option<TypeName>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VarInfo {
    pub name: String,
    pub ty: TypeName,
    pub fn_id: u32,
    pub scope_id: u32,
    /// The variable's name where it is declared.
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScopeInfo {
    pub fn_id: u32,
    /// `None` for a function's outermost scope.
    pub parent: Option<u32>,
    pub span: Span,
}

/// How the debugger should display a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TypeName {
    Int,
    Bool,
}
