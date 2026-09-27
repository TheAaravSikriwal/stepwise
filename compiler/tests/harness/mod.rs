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
