use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::emitter::python::PythonEmitter;
use crate::errors::{CompileError, Diagnostic};
use crate::lexer::lexer::Lexer;
use crate::parser::ast::*;
use crate::parser::parser::Parser;
use crate::source::{SourceFile, Span};
use crate::typechecker::checker::TypeChecker;

pub struct Compiler {
    compiled: HashMap<PathBuf, String>,
    in_progress: HashSet<PathBuf>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            compiled: HashMap::new(),
            in_progress: HashSet::new(),
        }
    }

    pub fn compile_file(&mut self, path: &Path) -> Result<HashMap<PathBuf, String>, CompileError> {
        let canonical = path
            .canonicalize()
            .map_err(|e| self.io_error(path, &e.to_string()))?;

        self.compile_module(&canonical)?;

        let mut outputs = HashMap::new();
        for (src_path, python) in &self.compiled {
            let py_path = src_path.with_extension("py");
            outputs.insert(py_path, python.clone());
        }
        Ok(outputs)
    }

    fn compile_module(&mut self, canonical_path: &Path) -> Result<(), CompileError> {
        if self.compiled.contains_key(canonical_path) {
            return Ok(());
        }

        if self.in_progress.contains(canonical_path) {
            return Err(CompileError::new(Diagnostic::error(
                format!("circular import: {}", canonical_path.display()),
                Span::dummy(),
            )));
        }

        self.in_progress.insert(canonical_path.to_path_buf());

        let source = SourceFile::from_path(canonical_path)
            .map_err(|e| self.io_error(canonical_path, &e.to_string()))?;
        let tokens = Lexer::new(&source).tokenize()?;
        let module = Parser::new(tokens, &source).parse_module()?;

        self.compile_dependencies(&module, canonical_path)?;
        TypeChecker::new().check_module(&module)?;
        let python_output = PythonEmitter::new().emit_module(&module);

        self.in_progress.remove(canonical_path);
        self.compiled
            .insert(canonical_path.to_path_buf(), python_output);

        Ok(())
    }

    fn compile_dependencies(
        &mut self,
        module: &Module,
        importing_file: &Path,
    ) -> Result<(), CompileError> {
        let dir = importing_file.parent().unwrap_or(Path::new("."));

        for stmt in &module.body {
            if let Statement::Import(import) = stmt {
                let module_path = match &import.kind {
                    ImportKind::Simple { module_path, .. } => module_path,
                    ImportKind::From { module_path, .. } => module_path,
                };

                if module_path.len() == 1 {
                    let sbz_path = dir.join(format!("{}.sbz", module_path[0]));
                    if sbz_path.exists() {
                        let canonical = sbz_path
                            .canonicalize()
                            .map_err(|e| self.io_error(&sbz_path, &e.to_string()))?;
                        self.compile_module(&canonical)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn io_error(&self, path: &Path, msg: &str) -> CompileError {
        CompileError::new(Diagnostic::error(
            format!("error reading {}: {msg}", path.display()),
            Span::dummy(),
        ))
    }
}
