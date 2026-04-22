use crate::parser::ast::*;

pub struct PythonEmitter {
    output: String,
    indent: usize,
}

impl PythonEmitter {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    pub fn emit_module(mut self, module: &Module) -> String {
        for stmt in &module.body {
            self.emit_statement(stmt);
        }
        self.output
    }

    fn emit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(binding) => self.emit_let(binding),
            Statement::Assign(assign) => self.emit_assign(assign),
            Statement::FuncDef(func) => self.emit_func_def(func),
            Statement::DataDecl(data) => self.emit_data_decl(data),
            Statement::If(if_stmt) => self.emit_if(if_stmt),
            Statement::Return(ret) => self.emit_return(ret),
            Statement::Expr(expr_stmt) => {
                self.write_indent();
                self.emit_expr(&expr_stmt.expr);
                self.output.push('\n');
            }
            Statement::TypeclassDecl(_) => {
                    }
            Statement::Impl(impl_block) => self.emit_impl_block(impl_block),
            Statement::For(for_stmt) => self.emit_for(for_stmt),
            Statement::While(while_stmt) => self.emit_while(while_stmt),
            Statement::Import(import) => self.emit_import(import),
        }
    }

    fn emit_let(&mut self, binding: &LetBinding) {
        self.write_indent();
        self.output.push_str(&binding.name);
        self.output.push_str(" = ");
        self.emit_expr(&binding.value);
        self.output.push('\n');
    }

    fn emit_assign(&mut self, assign: &Assignment) {
        self.write_indent();
        self.emit_expr(&assign.target);
        self.output.push_str(" = ");
        self.emit_expr(&assign.value);
        self.output.push('\n');
    }

    fn emit_func_def(&mut self, func: &FuncDef) {
        self.write_indent();
        self.output.push_str("def ");
        self.output.push_str(&func.name);
        self.output.push('(');
        for (i, param) in func.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&param.name);
        }
        self.output.push_str("):\n");

        self.indent += 1;
        if func.body.is_empty() {
            self.write_indent();
            self.output.push_str("pass\n");
        } else {
            for stmt in &func.body {
                self.emit_statement(stmt);
            }
        }
        self.indent -= 1;
        self.output.push('\n');
    }

    fn emit_data_decl(&mut self, data: &DataDecl) {
        self.write_indent();
        self.output.push_str("class ");
        self.output.push_str(&data.name);
        self.output.push_str(":\n");

        self.indent += 1;

        self.write_indent();
        self.output.push_str("def __init__(self");
        for field in &data.fields {
            self.output.push_str(", ");
            self.output.push_str(&field.name);
        }
        self.output.push_str("):\n");
        self.indent += 1;
        if data.fields.is_empty() {
            self.write_indent();
            self.output.push_str("pass\n");
        } else {
            for field in &data.fields {
                self.write_indent();
                self.output.push_str("self.");
                self.output.push_str(&field.name);
                self.output.push_str(" = ");
                self.output.push_str(&field.name);
                self.output.push('\n');
            }
        }
        self.indent -= 1;
        self.output.push('\n');

        self.write_indent();
        self.output.push_str("def __eq__(self, other):\n");
        self.indent += 1;
        self.write_indent();
        self.output.push_str("if not isinstance(other, ");
        self.output.push_str(&data.name);
        self.output.push_str("):\n");
        self.indent += 1;
        self.write_indent();
        self.output.push_str("return NotImplemented\n");
        self.indent -= 1;
        self.write_indent();
        self.output.push_str("return ");
        if data.fields.is_empty() {
            self.output.push_str("True");
        } else {
            for (i, field) in data.fields.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(" and ");
                }
                self.output.push_str("self.");
                self.output.push_str(&field.name);
                self.output.push_str(" == other.");
                self.output.push_str(&field.name);
            }
        }
        self.output.push('\n');
        self.indent -= 1;
        self.output.push('\n');

        self.write_indent();
        self.output.push_str("def __repr__(self):\n");
        self.indent += 1;
        self.write_indent();
        self.output.push_str("return f\"");
        self.output.push_str(&data.name);
        self.output.push('(');
        for (i, field) in data.fields.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&field.name);
            self.output.push_str("={self.");
            self.output.push_str(&field.name);
            self.output.push_str("!r}");
        }
        self.output.push_str(")\"\n");
        self.indent -= 1;

        for method in &data.methods {
            self.output.push('\n');
            self.emit_func_def(method);
        }

        self.indent -= 1;
        self.output.push('\n');
    }

    fn emit_for(&mut self, for_stmt: &ForStmt) {
        self.write_indent();
        self.output.push_str("for ");
        self.output.push_str(&for_stmt.var_name);
        self.output.push_str(" in ");
        self.emit_expr(&for_stmt.iterable);
        self.output.push_str(":\n");

        self.indent += 1;
        for stmt in &for_stmt.body {
            self.emit_statement(stmt);
        }
        self.indent -= 1;
    }

    fn emit_while(&mut self, while_stmt: &WhileStmt) {
        self.write_indent();
        self.output.push_str("while ");
        self.emit_expr(&while_stmt.condition);
        self.output.push_str(":\n");

        self.indent += 1;
        for stmt in &while_stmt.body {
            self.emit_statement(stmt);
        }
        self.indent -= 1;
    }

    fn emit_import(&mut self, import: &ImportStmt) {
        self.write_indent();
        match &import.kind {
            ImportKind::Simple { module_path, alias } => {
                self.output.push_str("import ");
                self.output.push_str(&module_path.join("."));
                if let Some(alias) = alias {
                    self.output.push_str(" as ");
                    self.output.push_str(alias);
                }
                self.output.push('\n');
            }
            ImportKind::From { module_path, names } => {
                self.output.push_str("from ");
                self.output.push_str(&module_path.join("."));
                self.output.push_str(" import ");
                for (i, name) in names.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.output.push_str(&name.name);
                    if let Some(alias) = &name.alias {
                        self.output.push_str(" as ");
                        self.output.push_str(alias);
                    }
                }
                self.output.push('\n');
            }
        }
    }

    fn emit_impl_block(&mut self, impl_block: &ImplBlock) {
        let type_name = match &impl_block.kind {
            ImplKind::Instance { type_args, .. } => {
                match type_args.first() {
                    Some(TypeExpr::Named(name, _)) => name.as_str(),
                    _ => return,
                }
            }
            ImplKind::Inherent { target_type } => target_type.as_str(),
        };

        if matches!(&impl_block.kind, ImplKind::Instance { .. })
            && matches!(type_name, "Int" | "Float" | "Str" | "Bool" | "None")
        {
            return;
        }

        let dunder_map: &[(&str, &str)] = &[
            ("add", "__add__"),
            ("sub", "__sub__"),
            ("mul", "__mul__"),
            ("div", "__truediv__"),
            ("eq", "__eq__"),
            ("lt", "__lt__"),
        ];

        for method in &impl_block.methods {
            let python_name = match &impl_block.kind {
                ImplKind::Instance { .. } => {
                    dunder_map
                        .iter()
                        .find(|(name, _)| *name == method.name)
                        .map(|(_, d)| d.to_string())
                        .unwrap_or_else(|| method.name.clone())
                }
                ImplKind::Inherent { .. } => method.name.clone(),
            };

            let func_name = format!("__{type_name}_{}", method.name);
            self.write_indent();
            self.output.push_str("def ");
            self.output.push_str(&func_name);
            self.output.push('(');
            for (i, param) in method.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(&param.name);
            }
            self.output.push_str("):\n");

            self.indent += 1;
            if method.body.is_empty() {
                self.write_indent();
                self.output.push_str("pass\n");
            } else {
                for stmt in &method.body {
                    self.emit_statement(stmt);
                }
            }
            self.indent -= 1;
            self.output.push('\n');

            self.write_indent();
            self.output
                .push_str(&format!("{type_name}.{python_name} = {func_name}\n"));
        }
    }

    fn emit_if(&mut self, if_stmt: &IfStmt) {
        self.write_indent();
        self.output.push_str("if ");
        self.emit_expr(&if_stmt.condition);
        self.output.push_str(":\n");

        self.indent += 1;
        for stmt in &if_stmt.then_body {
            self.emit_statement(stmt);
        }
        self.indent -= 1;

        for (cond, body) in &if_stmt.elif_clauses {
            self.write_indent();
            self.output.push_str("elif ");
            self.emit_expr(cond);
            self.output.push_str(":\n");
            self.indent += 1;
            for stmt in body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
        }

        if let Some(else_body) = &if_stmt.else_body {
            self.write_indent();
            self.output.push_str("else:\n");
            self.indent += 1;
            for stmt in else_body {
                self.emit_statement(stmt);
            }
            self.indent -= 1;
        }
    }

    fn emit_return(&mut self, ret: &ReturnStmt) {
        self.write_indent();
        if let Some(value) = &ret.value {
            self.output.push_str("return ");
            self.emit_expr(value);
        } else {
            self.output.push_str("return");
        }
        self.output.push('\n');
    }

    fn emit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::IntLiteral(v, _) => self.output.push_str(&v.to_string()),
            Expr::FloatLiteral(v, _) => {
                let s = v.to_string();
                self.output.push_str(&s);
                if !s.contains('.') {
                    self.output.push_str(".0");
                }
            }
            Expr::StringLiteral(s, _) => {
                self.output.push('"');
                for ch in s.chars() {
                    match ch {
                        '"' => self.output.push_str("\\\""),
                        '\\' => self.output.push_str("\\\\"),
                        '\n' => self.output.push_str("\\n"),
                        '\t' => self.output.push_str("\\t"),
                        c => self.output.push(c),
                    }
                }
                self.output.push('"');
            }
            Expr::BoolLiteral(v, _) => {
                self.output.push_str(if *v { "True" } else { "False" });
            }
            Expr::NoneLiteral(_) => self.output.push_str("None"),
            Expr::Name(name, _) => self.output.push_str(name),
            Expr::BinOp(binop) => {
                self.output.push('(');
                self.emit_expr(&binop.left);
                let op_str = match binop.op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Eq => " == ",
                    BinOp::NotEq => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::LtEq => " <= ",
                    BinOp::GtEq => " >= ",
                    BinOp::And => " and ",
                    BinOp::Or => " or ",
                };
                self.output.push_str(op_str);
                self.emit_expr(&binop.right);
                self.output.push(')');
            }
            Expr::UnaryOp(unary) => {
                match unary.op {
                    UnaryOp::Neg => self.output.push_str("(-"),
                    UnaryOp::Not => self.output.push_str("(not "),
                }
                self.emit_expr(&unary.operand);
                self.output.push(')');
            }
            Expr::Call(call) => {
                self.emit_expr(&call.callee);
                self.output.push('(');
                for (i, arg) in call.args.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expr(arg);
                }
                self.output.push(')');
            }
            Expr::Attr(attr) => {
                self.emit_expr(&attr.object);
                self.output.push('.');
                self.output.push_str(&attr.attr);
            }
            Expr::ListLiteral(elements, _) => {
                self.output.push('[');
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.emit_expr(elem);
                }
                self.output.push(']');
            }
            Expr::Index(idx) => {
                self.emit_expr(&idx.object);
                self.output.push('[');
                self.emit_expr(&idx.index);
                self.output.push(']');
            }
        }
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer::Lexer;
    use crate::parser::parser::Parser;
    use crate::source::SourceFile;

    fn compile(input: &str) -> String {
        let source = SourceFile::from_string(input);
        let tokens = Lexer::new(&source).tokenize().unwrap();
        let module = Parser::new(tokens, &source).parse_module().unwrap();
        PythonEmitter::new().emit_module(&module)
    }

    #[test]
    fn test_let_binding() {
        let output = compile("let x: Int = 42\n");
        assert_eq!(output.trim(), "x = 42");
    }

    #[test]
    fn test_func_def() {
        let output = compile("def add(a: Int, b: Int) -> Int:\n    return a + b\n");
        assert!(output.contains("def add(a, b):"));
        assert!(output.contains("return (a + b)"));
    }

    #[test]
    fn test_data_decl() {
        let output = compile("data Point:\n    x: Int\n    y: Int\n");
        assert!(output.contains("class Point:"));
        assert!(output.contains("def __init__(self, x, y):"));
        assert!(output.contains("self.x = x"));
        assert!(output.contains("self.y = y"));
        assert!(output.contains("def __eq__(self, other):"));
        assert!(output.contains("def __repr__(self):"));
    }

    #[test]
    fn test_if_else() {
        let output = compile("if True:\n    let x: Int = 1\nelse:\n    let x: Int = 2\n");
        assert!(output.contains("if True:"));
        assert!(output.contains("else:"));
    }

    #[test]
    fn test_string_literal() {
        let output = compile("let x: Str = \"hello\\nworld\"\n");
        assert!(output.contains(r#"x = "hello\nworld""#));
    }

    #[test]
    fn test_while_loop_translation() {
        let input = "while True:\n    let x: Int = 42\n";
                let output = compile(input);

        assert!(output.contains("while True:"));
        assert!(output.contains("    x = 42"));
    }

    #[test]
    fn test_function_call() {
        let output = compile("print(\"hello\")\n");
        assert_eq!(output.trim(), "print(\"hello\")");
    }

    #[test]
    fn test_import_simple() {
        let output = compile("import os\n");
        assert_eq!(output.trim(), "import os");
    }

    #[test]
    fn test_import_alias() {
        let output = compile("import numpy as np\n");
        assert_eq!(output.trim(), "import numpy as np");
    }

    #[test]
    fn test_from_import() {
        let output = compile("from os.path import join, exists\n");
        assert_eq!(output.trim(), "from os.path import join, exists");
    }

    #[test]
    fn test_from_import_alias() {
        let output = compile("from os.path import join as j\n");
        assert_eq!(output.trim(), "from os.path import join as j");
    }
}
