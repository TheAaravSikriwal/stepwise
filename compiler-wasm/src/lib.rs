//! Exposes the compiler to JavaScript.
//!
//! The one job of this layer besides marshalling: convert byte-offset spans
//! into UTF-16 offsets, which is what CodeMirror and JavaScript strings use.
//! Core compiler code never has to think about UTF-16.

use serde_json::{Value, json};
use stepwise_compiler::{Diagnostic, Span, normalize_newlines};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CompileResult {
    wasm: Option<Vec<u8>>,
    meta: String,
}

#[wasm_bindgen]
impl CompileResult {
    /// The compiled module, or `undefined` if there were errors.
    #[wasm_bindgen(getter)]
    pub fn wasm(&self) -> Option<Vec<u8>> {
        self.wasm.clone()
    }

    /// JSON: `{ diagnostics: [...], debug: {...} }`. See `web/src/compiler.ts`.
    #[wasm_bindgen(getter)]
    pub fn meta(&self) -> String {
        self.meta.clone()
    }
}

#[wasm_bindgen]
extern "C" {
    /// Set by the worker before loading the compiler. Receives the message of
    /// a compiler panic, since a panic otherwise surfaces in JS only as an
    /// opaque `unreachable` trap.
    #[wasm_bindgen(js_namespace = globalThis, js_name = __stepwisePanic)]
    fn report_panic(message: &str);
}

/// Runs automatically when the module is loaded.
#[wasm_bindgen(start)]
fn start() {
    std::panic::set_hook(Box::new(|info| report_panic(&info.to_string())));
}

/// Panics on purpose, so tests can check that panics reach JS with their
/// message (see `web/tests/panic.test.ts`).
#[wasm_bindgen]
pub fn test_panic() {
    panic!("test panic requested");
}

/// Which compiler stages are real (see `PIPELINE` in the compiler crate),
/// e.g. `"Stub"` or `"Codegen"`.
#[wasm_bindgen]
pub fn pipeline() -> String {
    format!("{:?}", stepwise_compiler::PIPELINE)
}

#[wasm_bindgen]
pub fn compile(source: &str) -> CompileResult {
    let source = normalize_newlines(source);
    let out = stepwise_compiler::compile(&source);
    let map = Utf16Map::new(&source);

    let diagnostics: Vec<Value> = out
        .diagnostics
        .iter()
        .map(|d| diagnostic_json(d, &map, &source))
        .collect();

    let debug = &out.debug;
    let meta = json!({
        "diagnostics": diagnostics,
        "debug": {
            "functions": debug.functions.iter().map(|f| json!({
                "name": f.name, "span": map.span(f.span),
            })).collect::<Vec<_>>(),
            "vars": debug.vars.iter().map(|v| json!({
                "name": v.name, "ty": v.ty, "fnId": v.fn_id, "scopeId": v.scope_id,
                "span": map.span(v.span),
            })).collect::<Vec<_>>(),
            "scopes": debug.scopes.iter().map(|s| json!({
                "fnId": s.fn_id, "parent": s.parent, "span": map.span(s.span),
            })).collect::<Vec<_>>(),
            "steps": debug.steps.iter().map(|&s| map.span(s)).collect::<Vec<_>>(),
        },
    });

    CompileResult {
        wasm: out.wasm,
        meta: meta.to_string(),
    }
}

fn diagnostic_json(d: &Diagnostic, map: &Utf16Map, source: &str) -> Value {
    json!({
        "severity": d.severity,
        "message": d.message,
        "span": map.span(d.span),
        "labels": d.labels.iter().map(|l| json!({
            "span": map.span(l.span), "message": l.message,
        })).collect::<Vec<_>>(),
        "notes": d.notes,
        "rendered": d.render(source, "main.step"),
    })
}

/// Byte offset → UTF-16 code unit offset.
struct Utf16Map {
    /// `(byte_offset, utf16_offset)` at every char boundary where the two
    /// start to differ; empty for pure-ASCII sources.
    checkpoints: Vec<(u32, u32)>,
}

impl Utf16Map {
    fn new(source: &str) -> Self {
        let mut checkpoints = Vec::new();
        let mut utf16 = 0u32;
        for (byte, ch) in source.char_indices() {
            if !ch.is_ascii() {
                checkpoints.push((byte as u32, utf16));
            }
            utf16 += ch.len_utf16() as u32;
            if !ch.is_ascii() {
                checkpoints.push(((byte + ch.len_utf8()) as u32, utf16));
            }
        }
        Utf16Map { checkpoints }
    }

    fn offset(&self, byte: u32) -> u32 {
        // Find the last checkpoint at or before `byte`; ASCII after it maps 1:1.
        match self.checkpoints.partition_point(|&(b, _)| b <= byte) {
            0 => byte,
            i => {
                let (b, u) = self.checkpoints[i - 1];
                u + (byte - b)
            }
        }
    }

    fn span(&self, span: Span) -> Value {
        json!({ "from": self.offset(span.start), "to": self.offset(span.end) })
    }
}

#[cfg(test)]
mod tests {
    use super::Utf16Map;

    #[test]
    fn ascii_maps_one_to_one() {
        let m = Utf16Map::new("fn main() {}");
        assert_eq!(m.offset(0), 0);
        assert_eq!(m.offset(7), 7);
    }

    #[test]
    fn non_ascii_shifts_later_offsets() {
        // 'é' is 2 bytes / 1 UTF-16 unit; '😀' is 4 bytes / 2 UTF-16 units.
        let src = "é😀x";
        let m = Utf16Map::new(src);
        assert_eq!(m.offset(0), 0);
        assert_eq!(m.offset(2), 1); // after é
        assert_eq!(m.offset(6), 3); // after 😀
        assert_eq!(m.offset(7), 4); // after x
        assert_eq!(src.encode_utf16().count(), 4);
    }
}
