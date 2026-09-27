//! Shared test harness: validate a compiled module and run it natively under
//! wasmtime, recording every trace event the same way the browser runtime does.

#![allow(dead_code)]

use stepwise_compiler::abi::{Import, MODULE};
use wasmtime::{Caller, Engine, Linker, Module, Store};

/// One recorded host call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Line(i32),
    Declare { var: i32, value: i32 },
    Assign { var: i32, old: i32, new: i32 },
    Call(i32),
    Ret(i32),
    ScopeExit(i32),
    Print(String),
}

#[derive(Debug)]
pub struct Run {
    pub events: Vec<Event>,
    /// Printed lines, in order.
    pub output: Vec<String>,
    /// `Some(message)` if the program trapped or hit the event limit.
    pub error: Option<String>,
}

pub const EVENT_LIMIT: usize = 1_000_000;

/// Panics with the validator's message if `wasm` is not a valid module.
pub fn validate(wasm: &[u8]) {
    let mut validator = wasmparser::Validator::new();
    if let Err(e) = validator.validate_all(wasm) {
        panic!("generated module is invalid WebAssembly: {e}");
    }
}

/// Checks that a trace is internally consistent: the invariants the replay
/// engine relies on. Returns a description of the first problem found.
///
/// - `call` / `ret` balance, and a frame has no live variables at `ret`
///   (every scope was exited, including on early returns)
/// - `declare` introduces a variable of the current function that isn't
///   already live
/// - `assign` targets a live variable, and its `old` is the current value
/// - `scope_exit` belongs to the current function; its variables die
/// - every `line` has an entry in `debug.steps`
pub fn check_trace(
    events: &[Event],
    debug: &stepwise_compiler::DebugTable,
    completed: bool,
) -> Result<(), String> {
    use std::collections::HashMap;
    struct Frame {
        fn_id: i32,
        vars: HashMap<i32, i32>,
    }
    let mut frames: Vec<Frame> = Vec::new();
    for (i, e) in events.iter().enumerate() {
        let fail = |msg: String| Err(format!("event {i} ({e:?}): {msg}"));
        let var_info = |v: i32| debug.vars.get(v as usize);
        match *e {
            Event::Call(f) => {
                if f < 0 || f as usize >= debug.functions.len() {
                    return fail("unknown fn_id".into());
                }
                frames.push(Frame {
                    fn_id: f,
                    vars: HashMap::new(),
                });
            }
            Event::Ret(_) => {
                let Some(frame) = frames.pop() else {
                    return fail("ret with no open call".into());
                };
                if !frame.vars.is_empty() {
                    let mut live: Vec<_> = frame.vars.keys().collect();
                    live.sort();
                    return fail(format!(
                        "variables {live:?} still live at return (missing scope_exit?)"
                    ));
                }
            }
            Event::Declare { var, value } => {
                let Some(frame) = frames.last_mut() else {
                    return fail("declare outside any call".into());
                };
                let Some(info) = var_info(var) else {
                    return fail("unknown var_id".into());
                };
                if info.fn_id as i32 != frame.fn_id {
                    return fail(format!(
                        "var belongs to fn {}, not the current one",
                        info.fn_id
                    ));
                }
                if frame.vars.insert(var, value).is_some() {
                    return fail("declared while already live (missing scope_exit?)".into());
                }
            }
            Event::Assign { var, old, new } => {
                let Some(frame) = frames.last_mut() else {
                    return fail("assign outside any call".into());
                };
                match frame.vars.get_mut(&var) {
                    None => return fail("assign to a variable that isn't live".into()),
                    Some(cur) if *cur != old => {
                        return fail(format!("old value is {old} but the variable holds {cur}"));
                    }
                    Some(cur) => *cur = new,
                }
            }
            Event::ScopeExit(s) => {
                let Some(frame) = frames.last_mut() else {
                    return fail("scope_exit outside any call".into());
                };
                let Some(scope) = debug.scopes.get(s as usize) else {
                    return fail("unknown scope_id".into());
                };
                if scope.fn_id as i32 != frame.fn_id {
                    return fail("scope belongs to another function".into());
                }
                frame
                    .vars
                    .retain(|v, _| debug.vars[*v as usize].scope_id as i32 != s);
            }
            Event::Line(s) => {
                if s < 0 || s as usize >= debug.steps.len() {
                    return fail("span_id has no entry in debug.steps".into());
                }
            }
            Event::Print(_) => {}
        }
    }
    if completed && !frames.is_empty() {
        return Err(format!("{} call(s) never returned", frames.len()));
    }
    Ok(())
}

pub fn run(wasm: &[u8]) -> Run {
    validate(wasm);
    let engine = Engine::default();
    let module = Module::new(&engine, wasm).expect("wasmtime rejected the module");
    let mut linker = Linker::<Vec<Event>>::new(&engine);

    fn record(caller: &mut Caller<'_, Vec<Event>>, event: Event) -> wasmtime::Result<()> {
        let events = caller.data_mut();
        if events.len() >= EVENT_LIMIT {
            return Err(wasmtime::Error::msg(format!(
                "stopped after {EVENT_LIMIT} events"
            )));
        }
        events.push(event);
        Ok(())
    }

    for import in Import::ALL {
        let name = import.name();
        let result = match import {
            Import::Line => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, s: i32| {
                record(&mut c, Event::Line(s))
            }),
            Import::Declare => linker.func_wrap(
                MODULE,
                name,
                |mut c: Caller<'_, _>, var: i32, value: i32| {
                    record(&mut c, Event::Declare { var, value })
                },
            ),
            Import::Assign => linker.func_wrap(
                MODULE,
                name,
                |mut c: Caller<'_, _>, var: i32, old: i32, new: i32| {
                    record(&mut c, Event::Assign { var, old, new })
                },
            ),
            Import::Call => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, f: i32| {
                record(&mut c, Event::Call(f))
            }),
            Import::Ret => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, v: i32| {
                record(&mut c, Event::Ret(v))
            }),
            Import::ScopeExit => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, s: i32| {
                record(&mut c, Event::ScopeExit(s))
            }),
            Import::Print => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, v: i32| {
                record(&mut c, Event::Print(v.to_string()))
            }),
            Import::PrintBool => linker.func_wrap(MODULE, name, |mut c: Caller<'_, _>, v: i32| {
                record(&mut c, Event::Print((v != 0).to_string()))
            }),
        };
        result.expect("failed to define host function");
    }

    let mut store = Store::new(&engine, Vec::new());
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("failed to instantiate module");
    let main = instance
        .get_typed_func::<(), ()>(&mut store, "main")
        .expect("module must export `main: () -> ()`");
    let error = main.call(&mut store, ()).err().map(|e| format!("{e:#}"));

    let events = store.into_data();
    let output = events
        .iter()
        .filter_map(|e| match e {
            Event::Print(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    Run {
        events,
        output,
        error,
    }
}
