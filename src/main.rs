use std::path::Path;
use std::process;

use senbonzakura_core::compiler::Compiler;
use senbonzakura_core::emitter::python::PythonEmitter;
use senbonzakura_core::errors::CompileError;
use senbonzakura_core::lexer::lexer::Lexer;
use senbonzakura_core::parser::parser::Parser;
use senbonzakura_core::source::SourceFile;
use senbonzakura_core::typechecker::checker::TypeChecker;

fn compile_single(source: &SourceFile) -> Result<String, CompileError> {
    let tokens = Lexer::new(source).tokenize()?;
    let module = Parser::new(tokens, source).parse_module()?;
    TypeChecker::new().check_module(&module)?;
    Ok(PythonEmitter::new().emit_module(&module))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: senbonzakura <command> <file.sbz>");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  compile  Transpile .sbz to Python");
        eprintln!("  check    Type-check only");
        eprintln!("  run      Transpile and execute with Python");
        process::exit(1);
    }

    let command = &args[1];

    if command == "--help" || command == "-h" {
        eprintln!("Usage: senbonzakura <command> <file.sbz>");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  compile  Transpile .sbz to Python (writes .py files)");
        eprintln!("  check    Type-check only");
        eprintln!("  run      Transpile and execute with Python");
        process::exit(0);
    }

    if args.len() < 3 {
        eprintln!("Error: missing file argument");
        process::exit(1);
    }

    let file_path = &args[2];
    let path = Path::new(file_path);

    match command.as_str() {
        "compile" => {
            let mut compiler = Compiler::new();
            match compiler.compile_file(path) {
                Ok(outputs) => {
                    for (out_path, content) in &outputs {
                        if let Err(e) = std::fs::write(out_path, content) {
                            eprintln!("Error writing {}: {e}", out_path.display());
                            process::exit(1);
                        }
                        eprintln!("Compiled to {}", out_path.display());
                    }
                }
                Err(e) => {
                    eprintln!("{}", e.diagnostic.message);
                    process::exit(1);
                }
            }
        }
        "check" => {
            let source = match SourceFile::from_path(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {file_path}: {e}");
                    process::exit(1);
                }
            };
            match compile_single(&source) {
                Ok(_) => eprintln!("{file_path}: ok"),
                Err(e) => {
                    eprint!("{}", e.diagnostic.display(&source));
                    process::exit(1);
                }
            }
        }
        "run" => {
            let source = match SourceFile::from_path(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error reading {file_path}: {e}");
                    process::exit(1);
                }
            };
            match compile_single(&source) {
                Ok(output) => {
                    let status = process::Command::new("python3")
                        .arg("-c")
                        .arg(&output)
                        .status();
                    match status {
                        Ok(s) => process::exit(s.code().unwrap_or(1)),
                        Err(e) => {
                            eprintln!("Error executing Python: {e}");
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprint!("{}", e.diagnostic.display(&source));
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {command}");
            eprintln!("Available commands: compile, check, run");
            process::exit(1);
        }
    }
}
