//! The trace ABI: host functions every compiled program imports.
//!
//! This is the contract between codegen (which emits calls) and the runtime
//! (which records them). See DEVPLAN.md §3.2 for the meaning of each event.
//!
//! Every module imports **all** of these, in this order, from module `"sw"`,
//! so import `i` is always function index `i`. The first defined function
//! therefore has index `Import::ALL.len()`.

use wasm_encoder::{EntityType, ImportSection, TypeSection, ValType};

pub const MODULE: &str = "sw";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Import {
    /// `line(span_id)`: a statement or loop condition is about to run.
    Line,
    /// `declare(var_id, value)`: a `let` or parameter comes into scope.
    Declare,
    /// `assign(var_id, old, new)`: an existing variable changes.
    Assign,
    /// `call(fn_id)`: a function was entered. Parameters follow as `declare`s.
    Call,
    /// `ret(value)`: the current function is returning (0 for no value).
    Ret,
    /// `scope_exit(scope_id)`: a block's variables go out of scope.
    ScopeExit,
    /// `print(value)`: program output.
    Print,
    /// `print_bool(value)`: like `print`, but shows `true` / `false`.
    PrintBool,
}

impl Import {
    pub const ALL: [Import; 8] = [
        Import::Line,
        Import::Declare,
        Import::Assign,
        Import::Call,
        Import::Ret,
        Import::ScopeExit,
        Import::Print,
        Import::PrintBool,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Import::Line => "line",
            Import::Declare => "declare",
            Import::Assign => "assign",
            Import::Call => "call",
            Import::Ret => "ret",
            Import::ScopeExit => "scope_exit",
            Import::Print => "print",
            Import::PrintBool => "print_bool",
        }
    }

    pub fn param_count(self) -> usize {
        match self {
            Import::Assign => 3,
            Import::Declare => 2,
            _ => 1,
        }
    }

    /// The function index to use with `call` in generated code.
    pub fn func_index(self) -> u32 {
        Import::ALL.iter().position(|&i| i == self).unwrap() as u32
    }
}

/// Number of imported functions; the first defined function has this index.
pub const IMPORT_COUNT: u32 = Import::ALL.len() as u32;

/// Adds one function type per import to `types` and imports them all.
///
/// Call this first, on an empty `TypeSection`: import `i` uses type index `i`,
/// so your own function types start at [`IMPORT_COUNT`].
pub fn emit_imports(types: &mut TypeSection, imports: &mut ImportSection) {
    assert_eq!(
        types.len(),
        0,
        "emit_imports must run before any other types are added"
    );
    for (i, import) in Import::ALL.iter().enumerate() {
        types
            .ty()
            .function(vec![ValType::I32; import.param_count()], []);
        imports.import(MODULE, import.name(), EntityType::Function(i as u32));
    }
}
