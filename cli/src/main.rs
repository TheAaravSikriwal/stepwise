//! `stepc`: a dev tool for looking at each compiler stage's output.
//!
//! ```text
//! stepc tokens <file>   the lexer's tokens, one per line
//! stepc ast    <file>   the parse tree as an S-expression
//! stepc check  <file>   compile and print diagnostics
//! stepc wat    <file>   the compiled module as WebAssembly text
//! ```
//!
//! More commands (`run`) arrive with the stages they show.

use std::process::ExitCode;
use stepwise_compiler::span::LineIndex;
use stepwise_compiler::{Diagnostic, compile, lexer, normalize_newlines, parser};

const USAGE: &str = "usage: stepc <tokens|ast|check|wat> <file.step>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [command, path] = args.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let source = match std::fs::read_to_string(path) {
        Ok(s) => normalize_newlines(&s).into_owned(),
        Err(e) => {
            eprintln!("error: can't read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let file_name = std::path::Path::new(path)
        .file_name()
        .map_or(path.clone(), |n| n.to_string_lossy().into_owned());

    match command.as_str() {
        "tokens" => {
            let (tokens, diags) = lexer::lex(&source);
            let index = LineIndex::new(&source);
            for t in &tokens {
                let (line, col) = index.line_col(t.span.start);
                let text = t.span.text(&source);
                println!(
                    "{:>4}:{:<3} {:<14} {text:?}",
                    line,
                    col,
                    format!("{:?}", t.kind)
                );
            }
            report(&diags, &source, &file_name)
        }
        "ast" => {
            let (program, diags) = parser::parse(&source);
            println!("{}", program.to_sexpr());
            report(&diags, &source, &file_name)
        }
        "check" => {
            let out = compile(&source);
            let code = report(&out.diagnostics, &source, &file_name);
            if !out.has_errors() {
                println!("ok");
            }
            code
        }
        "wat" => {
            let out = compile(&source);
            let code = report(&out.diagnostics, &source, &file_name);
            if let Some(wasm) = out.wasm {
                match wasmprinter::print_bytes(&wasm) {
                    Ok(text) => println!("{text}"),
                    Err(e) => {
                        eprintln!("error: generated module can't be printed: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
            code
        }
        _ => {
            eprintln!("unknown command `{command}`\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Prints diagnostics to stderr; fails if any is an error.
fn report(diags: &[Diagnostic], source: &str, file_name: &str) -> ExitCode {
    for d in diags {
        eprintln!("{}", d.render(source, file_name));
    }
    if diags.iter().any(Diagnostic::is_error) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
