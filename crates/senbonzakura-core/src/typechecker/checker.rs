use crate::errors::{CompileError, Diagnostic};
use crate::parser::ast::*;
use crate::typechecker::environment::Environment;
use crate::typechecker::types::{
    DataType, FunctionType, InstanceDef, ModuleType, Type, TypeclassDef, TypeclassMethodSig,
};

pub struct TypeChecker {
    env: Environment,
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Int => "Int".to_string(),
        Type::Float => "Float".to_string(),
        Type::Str => "Str".to_string(),
        Type::Bool => "Bool".to_string(),
        Type::None => "None".to_string(),
        Type::Data(d) => d.name.clone(),
        Type::Function(_) => "Function".to_string(),
        Type::TypeVar(n) => n.clone(),
        Type::List(inner) => format!("List[{}]", type_name(inner)),
        Type::Module(m) => m.name.clone(),
        Type::Unknown => "Unknown".to_string(),
    }
}

fn op_to_typeclass(op: BinOp) -> Option<(&'static str, &'static str)> {
    match op {
        BinOp::Add => Some(("Add", "add")),
        BinOp::Sub => Some(("Sub", "sub")),
        BinOp::Mul => Some(("Mul", "mul")),
        BinOp::Div => Some(("Div", "div")),
        BinOp::Eq | BinOp::NotEq => Some(("Eq", "eq")),
        BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => Some(("Ord", "lt")),
        BinOp::And | BinOp::Or => None,
    }
}

fn register_builtin_typeclass(env: &mut Environment, name: &str, method_name: &str) {
    env.define_typeclass(
        name.to_string(),
        TypeclassDef {
            name: name.to_string(),
            type_params: vec!["L".to_string(), "R".to_string(), "Out".to_string()],
            methods: vec![TypeclassMethodSig {
                name: method_name.to_string(),
                param_types: vec![
                    Type::TypeVar("L".to_string()),
                    Type::TypeVar("R".to_string()),
                ],
                return_type: Type::TypeVar("Out".to_string()),
            }],
        },
    );
}

fn register_builtin_instance(
    env: &mut Environment,
    typeclass: &str,
    left: Type,
    right: Type,
    out: Type,
) {
    env.define_instance(
        typeclass.to_string(),
        InstanceDef {
            typeclass_name: typeclass.to_string(),
            type_args: vec![left, right, out],
            is_builtin: true,
        },
    );
}

fn types_compatible(a: &Type, b: &Type) -> bool {
    matches!(a, Type::Unknown) || matches!(b, Type::Unknown) || a == b
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut env = Environment::new();

        env.define(
            "print".to_string(),
            Type::Function(FunctionType {
                type_params: vec![],
                param_types: vec![Type::Unknown],
                return_type: Box::new(Type::None),
            }),
            false,
        );

        env.define(
            "len".to_string(),
            Type::Function(FunctionType {
                type_params: vec![],
                param_types: vec![Type::Unknown],
                return_type: Box::new(Type::Int),
            }),
            false,
        );

        env.define(
            "range".to_string(),
            Type::Function(FunctionType {
                type_params: vec![],
                param_types: vec![Type::Int],
                return_type: Box::new(Type::List(Box::new(Type::Int))),
            }),
            false,
        );

        register_builtin_typeclass(&mut env, "Add", "add");
        register_builtin_typeclass(&mut env, "Sub", "sub");
        register_builtin_typeclass(&mut env, "Mul", "mul");
        register_builtin_typeclass(&mut env, "Div", "div");
        register_builtin_typeclass(&mut env, "Eq", "eq");
        register_builtin_typeclass(&mut env, "Ord", "lt");

        for tc in &["Add", "Sub", "Mul", "Div"] {
            register_builtin_instance(&mut env, tc, Type::Int, Type::Int, Type::Int);
            register_builtin_instance(&mut env, tc, Type::Float, Type::Float, Type::Float);
        }
        register_builtin_instance(&mut env, "Add", Type::Str, Type::Str, Type::Str);

        for tc in &["Eq"] {
            register_builtin_instance(&mut env, tc, Type::Int, Type::Int, Type::Bool);
            register_builtin_instance(&mut env, tc, Type::Float, Type::Float, Type::Bool);
            register_builtin_instance(&mut env, tc, Type::Str, Type::Str, Type::Bool);
            register_builtin_instance(&mut env, tc, Type::Bool, Type::Bool, Type::Bool);
        }

        for tc in &["Ord"] {
            register_builtin_instance(&mut env, tc, Type::Int, Type::Int, Type::Bool);
            register_builtin_instance(&mut env, tc, Type::Float, Type::Float, Type::Bool);
        }

        Self { env }
    }

    pub fn check_module(&mut self, module: &Module) -> crate::errors::Result<()> {
        for stmt in &module.body {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> crate::errors::Result<()> {
        match stmt {
            Statement::Let(binding) => self.check_let_binding(binding),
            Statement::Assign(assign) => self.check_assignment(assign),
            Statement::FuncDef(func) => self.check_func_def(func),
            Statement::DataDecl(data) => self.check_data_decl(data),
            Statement::If(if_stmt) => self.check_if_stmt(if_stmt),
            Statement::Return(ret) => self.check_return(ret),
            Statement::Expr(expr_stmt) => {
                self.synth(&expr_stmt.expr)?;
                Ok(())
            }
            Statement::TypeclassDecl(tc) => self.check_typeclass_decl(tc),
            Statement::Impl(impl_block) => self.check_impl_block(impl_block),
            Statement::For(for_stmt) => self.check_for_stmt(for_stmt),
            Statement::Import(import) => self.check_import(import),
        }
    }

    fn check_let_binding(&mut self, binding: &LetBinding) -> crate::errors::Result<()> {
        let value_type = self.synth(&binding.value)?;

        if let Some(ann) = &binding.type_annotation {
            let expected = self.resolve_type_expr(ann)?;
            if !types_compatible(&value_type, &expected) {
                return Err(CompileError::new(Diagnostic::error(
                    format!("type mismatch: expected {expected}, found {value_type}"),
                    binding.value.span(),
                )));
            }
        }

        let ty = if let Some(ann) = &binding.type_annotation {
            self.resolve_type_expr(ann)?
        } else {
            value_type
        };

        self.env.define(binding.name.clone(), ty, binding.mutable);
        Ok(())
    }

    fn check_assignment(&mut self, assign: &Assignment) -> crate::errors::Result<()> {
        let target_name = match &assign.target {
            Expr::Name(name, _) => name,
            _ => {
                return Err(CompileError::new(Diagnostic::error(
                    "can only assign to variables",
                    assign.target.span(),
                )));
            }
        };

        let binding = self.env.lookup(target_name).ok_or_else(|| {
            CompileError::new(Diagnostic::error(
                format!("undefined variable: {target_name}"),
                assign.target.span(),
            ))
        })?;

        if !binding.mutable {
            return Err(CompileError::new(Diagnostic::error(
                format!("cannot assign to immutable variable '{target_name}'"),
                assign.target.span(),
            )));
        }

        let expected = binding.ty.clone();
        let value_type = self.synth(&assign.value)?;

        if value_type != expected {
            return Err(CompileError::new(Diagnostic::error(
                format!("type mismatch: expected {expected}, found {value_type}"),
                assign.value.span(),
            )));
        }

        Ok(())
    }

    fn check_func_def(&mut self, func: &FuncDef) -> crate::errors::Result<()> {
        let has_type_params = !func.type_params.is_empty();

        let tp_info: Vec<(String, Vec<String>)> = func
            .type_params
            .iter()
            .map(|tp| (tp.name.clone(), tp.constraints.clone()))
            .collect();

        let mut param_types = Vec::new();
        for param in &func.params {
            let ty = match &param.type_annotation {
                Some(ann) => {
                    if has_type_params {
                        self.resolve_type_expr_with_typevars(ann, &func.type_params)?
                    } else {
                        self.resolve_type_expr(ann)?
                    }
                }
                None => {
                    return Err(CompileError::new(Diagnostic::error(
                        format!("parameter '{}' requires a type annotation", param.name),
                        param.span,
                    )));
                }
            };
            param_types.push(ty);
        }

        let return_type = match &func.return_type {
            Some(ann) => {
                if has_type_params {
                    self.resolve_type_expr_with_typevars(ann, &func.type_params)?
                } else {
                    self.resolve_type_expr(ann)?
                }
            }
            None => Type::None,
        };

        let func_type = Type::Function(FunctionType {
            type_params: tp_info.clone(),
            param_types: param_types.clone(),
            return_type: Box::new(return_type.clone()),
        });

        self.env.define(func.name.clone(), func_type, false);

        let parent = std::mem::replace(&mut self.env, Environment::new());
        self.env = Environment::child(parent);

        for tp in &func.type_params {
            self.env
                .define(tp.name.clone(), Type::TypeVar(tp.name.clone()), false);

            // T: Add registers a temporary Add[T, T, T] instance for constraint checking
            for constraint in &tp.constraints {
                let tc = self.env.lookup_typeclass(constraint).ok_or_else(|| {
                    CompileError::new(Diagnostic::error(
                        format!("unknown typeclass: {constraint}"),
                        tp.span,
                    ))
                })?;
                let n_params = tc.type_params.len();
                let type_args = vec![Type::TypeVar(tp.name.clone()); n_params];
                self.env.define_instance(
                    constraint.clone(),
                    InstanceDef {
                        typeclass_name: constraint.clone(),
                        type_args,
                        is_builtin: false,
                    },
                );
            }
        }

        for (param, ty) in func.params.iter().zip(param_types.iter()) {
            self.env.define(param.name.clone(), ty.clone(), false);
        }

        for stmt in &func.body {
            self.check_statement(stmt)?;
        }

        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = child.into_parent().unwrap();

        Ok(())
    }

    fn check_data_decl(&mut self, data: &DataDecl) -> crate::errors::Result<()> {
        let mut fields = Vec::new();
        for field in &data.fields {
            let ty = self.resolve_type_expr(&field.type_annotation)?;
            fields.push((field.name.clone(), ty));
        }

        let data_type = Type::Data(DataType {
            name: data.name.clone(),
            fields: fields.clone(),
        });

        self.env.define_type(data.name.clone(), data_type.clone());

        let field_types: Vec<Type> = fields.iter().map(|(_, ty)| ty.clone()).collect();
        let constructor = Type::Function(FunctionType {
            type_params: vec![],
            param_types: field_types,
            return_type: Box::new(data_type),
        });
        self.env.define(data.name.clone(), constructor, false);

        Ok(())
    }

    fn check_typeclass_decl(&mut self, tc: &TypeclassDecl) -> crate::errors::Result<()> {
        let mut methods = Vec::new();
        for sig in &tc.methods {
            let mut param_types = Vec::new();
            for param in &sig.params {
                let ty = match &param.type_annotation {
                    Some(ann) => self.resolve_type_expr_with_tc_params(ann, &tc.type_params)?,
                    None => {
                        return Err(CompileError::new(Diagnostic::error(
                            "typeclass method parameters require type annotations",
                            param.span,
                        )));
                    }
                };
                param_types.push(ty);
            }
            let return_type = self.resolve_type_expr_with_tc_params(&sig.return_type, &tc.type_params)?;
            methods.push(TypeclassMethodSig {
                name: sig.name.clone(),
                param_types,
                return_type,
            });
        }

        self.env.define_typeclass(
            tc.name.clone(),
            TypeclassDef {
                name: tc.name.clone(),
                type_params: tc.type_params.clone(),
                methods,
            },
        );
        Ok(())
    }

    fn check_impl_block(&mut self, impl_block: &ImplBlock) -> crate::errors::Result<()> {
        match &impl_block.kind {
            ImplKind::Instance { typeclass_name, type_args } => {
                let tc_def = self
                    .env
                    .lookup_typeclass(typeclass_name)
                    .ok_or_else(|| {
                        CompileError::new(Diagnostic::error(
                            format!("unknown typeclass: {typeclass_name}"),
                            impl_block.span,
                        ))
                    })?
                    .clone();

                let mut resolved_type_args = Vec::new();
                for type_arg in type_args {
                    resolved_type_args.push(self.resolve_type_expr(type_arg)?);
                }

                if resolved_type_args.len() != tc_def.type_params.len() {
                    return Err(CompileError::new(Diagnostic::error(
                        format!(
                            "typeclass {typeclass_name} expects {} type arguments, found {}",
                            tc_def.type_params.len(),
                            resolved_type_args.len()
                        ),
                        impl_block.span,
                    )));
                }

                self.check_impl_methods(&impl_block.methods)?;

                self.env.define_instance(
                    typeclass_name.clone(),
                    InstanceDef {
                        typeclass_name: typeclass_name.clone(),
                        type_args: resolved_type_args,
                        is_builtin: false,
                    },
                );
            }
            ImplKind::Inherent { target_type } => {
                if self.env.lookup_type(target_type).is_none() {
                    return Err(CompileError::new(Diagnostic::error(
                        format!("unknown type: {target_type}"),
                        impl_block.span,
                    )));
                }

                for method in &impl_block.methods {
                    self.check_impl_methods(&[method.clone()])?;

                    if let Some(first_param) = method.params.first() {
                        if first_param.name == "self" {
                            let mut param_types = Vec::new();
                            for param in method.params.iter().skip(1) {
                                if let Some(ann) = &param.type_annotation {
                                    param_types.push(self.resolve_type_expr(ann)?);
                                }
                            }
                            let return_type = match &method.return_type {
                                Some(ann) => self.resolve_type_expr(ann)?,
                                None => Type::None,
                            };
                            self.env.define_method(
                                target_type.clone(),
                                method.name.clone(),
                                Type::Function(FunctionType {
                                    type_params: vec![],
                                    param_types,
                                    return_type: Box::new(return_type),
                                }),
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn check_impl_methods(&mut self, methods: &[FuncDef]) -> crate::errors::Result<()> {
        for method in methods {
            let parent = std::mem::replace(&mut self.env, Environment::new());
            self.env = Environment::child(parent);

            for param in &method.params {
                if let Some(ann) = &param.type_annotation {
                    let ty = self.resolve_type_expr(ann)?;
                    self.env.define(param.name.clone(), ty, false);
                }
            }

            for stmt in &method.body {
                self.check_statement(stmt)?;
            }

            let child = std::mem::replace(&mut self.env, Environment::new());
            self.env = child.into_parent().unwrap();
        }
        Ok(())
    }

    fn check_for_stmt(&mut self, for_stmt: &ForStmt) -> crate::errors::Result<()> {
        let iter_ty = self.synth(&for_stmt.iterable)?;
        let elem_ty = match &iter_ty {
            Type::List(inner) => *inner.clone(),
            Type::Unknown => Type::Unknown,
            _ => {
                return Err(CompileError::new(Diagnostic::error(
                    format!("cannot iterate over {iter_ty}"),
                    for_stmt.iterable.span(),
                )));
            }
        };

        let parent = std::mem::replace(&mut self.env, Environment::new());
        self.env = Environment::child(parent);
        self.env.define(for_stmt.var_name.clone(), elem_ty, false);

        for stmt in &for_stmt.body {
            self.check_statement(stmt)?;
        }

        let child = std::mem::replace(&mut self.env, Environment::new());
        self.env = child.into_parent().unwrap();
        Ok(())
    }

    fn check_import(&mut self, import: &ImportStmt) -> crate::errors::Result<()> {
        match &import.kind {
            ImportKind::Simple { module_path, alias } => {
                let name = alias.as_deref().unwrap_or(&module_path[0]);
                self.env.define(
                    name.to_string(),
                    Type::Module(ModuleType {
                        name: module_path.join("."),
                    }),
                    false,
                );
            }
            ImportKind::From { names, .. } => {
                for imp_name in names {
                    let bind_name = imp_name.alias.as_deref().unwrap_or(&imp_name.name);
                    self.env.define(bind_name.to_string(), Type::Unknown, false);
                }
            }
        }
        Ok(())
    }

    fn check_if_stmt(&mut self, if_stmt: &IfStmt) -> crate::errors::Result<()> {
        let cond_type = self.synth(&if_stmt.condition)?;
        if cond_type != Type::Bool {
            return Err(CompileError::new(Diagnostic::error(
                format!("condition must be Bool, found {cond_type}"),
                if_stmt.condition.span(),
            )));
        }

        for stmt in &if_stmt.then_body {
            self.check_statement(stmt)?;
        }

        for (cond, body) in &if_stmt.elif_clauses {
            let ct = self.synth(cond)?;
            if ct != Type::Bool {
                return Err(CompileError::new(Diagnostic::error(
                    format!("condition must be Bool, found {ct}"),
                    cond.span(),
                )));
            }
            for stmt in body {
                self.check_statement(stmt)?;
            }
        }

        if let Some(else_body) = &if_stmt.else_body {
            for stmt in else_body {
                self.check_statement(stmt)?;
            }
        }

        Ok(())
    }

    fn check_return(&mut self, ret: &ReturnStmt) -> crate::errors::Result<()> {
        if let Some(value) = &ret.value {
            self.synth(value)?;
        }
        Ok(())
    }

    fn synth(&mut self, expr: &Expr) -> crate::errors::Result<Type> {
        match expr {
            Expr::IntLiteral(_, _) => Ok(Type::Int),
            Expr::FloatLiteral(_, _) => Ok(Type::Float),
            Expr::StringLiteral(_, _) => Ok(Type::Str),
            Expr::BoolLiteral(_, _) => Ok(Type::Bool),
            Expr::NoneLiteral(_) => Ok(Type::None),

            Expr::Name(name, span) => self
                .env
                .lookup(name)
                .map(|b| b.ty.clone())
                .ok_or_else(|| {
                    CompileError::new(Diagnostic::error(
                        format!("undefined variable: {name}"),
                        *span,
                    ))
                }),

            Expr::BinOp(binop) => self.synth_binop(binop),
            Expr::UnaryOp(unary) => self.synth_unary(unary),
            Expr::Call(call) => self.synth_call(call),
            Expr::Attr(attr) => self.synth_attr(attr),
            Expr::ListLiteral(elements, span) => {
                if elements.is_empty() {
                    return Err(CompileError::new(Diagnostic::error(
                        "cannot infer type of empty list; use a type annotation",
                        *span,
                    )));
                }
                let first_ty = self.synth(&elements[0])?;
                for elem in &elements[1..] {
                    let elem_ty = self.synth(elem)?;
                    if !types_compatible(&elem_ty, &first_ty) {
                        return Err(CompileError::new(Diagnostic::error(
                            format!("list element type mismatch: expected {first_ty}, found {elem_ty}"),
                            elem.span(),
                        )));
                    }
                }
                Ok(Type::List(Box::new(first_ty)))
            }
            Expr::Index(idx) => {
                let obj_ty = self.synth(&idx.object)?;
                let index_ty = self.synth(&idx.index)?;
                match &obj_ty {
                    Type::List(inner) => {
                        if !types_compatible(&index_ty, &Type::Int) {
                            return Err(CompileError::new(Diagnostic::error(
                                format!("list index must be Int, found {index_ty}"),
                                idx.index.span(),
                            )));
                        }
                        Ok(*inner.clone())
                    }
                    Type::Unknown => Ok(Type::Unknown),
                    _ => Err(CompileError::new(Diagnostic::error(
                        format!("cannot index into {obj_ty}"),
                        idx.span,
                    ))),
                }
            }
        }
    }

    fn synth_binop(&mut self, binop: &BinOpExpr) -> crate::errors::Result<Type> {
        let left_ty = self.synth(&binop.left)?;
        let right_ty = self.synth(&binop.right)?;

        if matches!(left_ty, Type::Unknown) || matches!(right_ty, Type::Unknown) {
            return Ok(Type::Unknown);
        }

        // and/or stay hardcoded — they are control flow, not overloadable
        if matches!(binop.op, BinOp::And | BinOp::Or) {
            if left_ty != Type::Bool || right_ty != Type::Bool {
                return Err(CompileError::new(Diagnostic::error(
                    format!(
                        "logical operators require Bool, found {left_ty} and {right_ty}"
                    ),
                    binop.span,
                )));
            }
            return Ok(Type::Bool);
        }

        let (tc_name, _method) = op_to_typeclass(binop.op).unwrap();

        let prefix = [left_ty.clone(), right_ty.clone()];
        let instance = self
            .env
            .lookup_instance_by_prefix(tc_name, &prefix)
            .ok_or_else(|| {
                CompileError::new(Diagnostic::error(
                    format!("no instance of {tc_name} for {left_ty} and {right_ty}"),
                    binop.span,
                ))
            })?;

        let result_type = instance.type_args.last().cloned().unwrap_or(Type::None);

        Ok(result_type)
    }

    fn synth_unary(&mut self, unary: &UnaryOpExpr) -> crate::errors::Result<Type> {
        let operand_ty = self.synth(&unary.operand)?;
        if matches!(operand_ty, Type::Unknown) {
            return Ok(Type::Unknown);
        }
        match unary.op {
            UnaryOp::Neg => {
                if operand_ty == Type::Int || operand_ty == Type::Float {
                    Ok(operand_ty)
                } else {
                    Err(CompileError::new(Diagnostic::error(
                        format!("cannot negate {operand_ty}"),
                        unary.span,
                    )))
                }
            }
            UnaryOp::Not => {
                if operand_ty == Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(CompileError::new(Diagnostic::error(
                        format!("'not' requires Bool, found {operand_ty}"),
                        unary.span,
                    )))
                }
            }
        }
    }

    fn synth_call(&mut self, call: &CallExpr) -> crate::errors::Result<Type> {
        let callee_ty = self.synth(&call.callee)?;

        match callee_ty {
            Type::Function(func_ty) => {
                if call.args.len() != func_ty.param_types.len() {
                    return Err(CompileError::new(Diagnostic::error(
                        format!(
                            "expected {} arguments, found {}",
                            func_ty.param_types.len(),
                            call.args.len()
                        ),
                        call.span,
                    )));
                }

                let mut type_bindings: Vec<(String, Type)> = Vec::new();
                for (tp_name, _) in &func_ty.type_params {
                    type_bindings.push((tp_name.clone(), Type::None));
                }

                let mut arg_types = Vec::new();
                for arg in &call.args {
                    arg_types.push(self.synth(arg)?);
                }

                for (arg_ty, param_ty) in arg_types.iter().zip(func_ty.param_types.iter()) {
                    self.infer_typevar_binding(param_ty, arg_ty, &mut type_bindings);
                }

                for (tp_name, inferred) in &type_bindings {
                    if *inferred == Type::None {
                        return Err(CompileError::new(Diagnostic::error(
                            format!("could not infer type for type parameter {tp_name}"),
                            call.span,
                        )));
                    }
                }

                for (tp_name, constraints) in &func_ty.type_params {
                    let concrete = type_bindings
                        .iter()
                        .find(|(n, _)| n == tp_name)
                        .map(|(_, t)| t)
                        .unwrap();

                    for constraint in constraints {
                        let tc = self.env.lookup_typeclass(constraint).cloned();
                        let n_params = tc.map_or(3, |tc| tc.type_params.len());
                        let args = vec![concrete.clone(); n_params];
                        if self.env.lookup_instance(constraint, &args).is_none() {
                            return Err(CompileError::new(Diagnostic::error(
                                format!(
                                    "type {concrete} does not satisfy constraint {constraint}"
                                ),
                                call.span,
                            )));
                        }
                    }
                }

                let mut result_type = *func_ty.return_type.clone();
                for (tp_name, concrete) in &type_bindings {
                    result_type = self.substitute_typevar(&result_type, tp_name, concrete);
                }

                if func_ty.type_params.is_empty() {
                    for (arg_ty, expected_ty) in arg_types.iter().zip(func_ty.param_types.iter()) {
                        if !types_compatible(arg_ty, expected_ty) {
                            return Err(CompileError::new(Diagnostic::error(
                                format!(
                                    "argument type mismatch: expected {expected_ty}, found {arg_ty}"
                                ),
                                call.span,
                            )));
                        }
                    }
                }

                Ok(result_type)
            }
            Type::Unknown => {
                for arg in &call.args {
                    self.synth(arg)?;
                }
                Ok(Type::Unknown)
            }
            _ => Err(CompileError::new(Diagnostic::error(
                format!("{callee_ty} is not callable"),
                call.callee.span(),
            ))),
        }
    }

    fn infer_typevar_binding(
        &self,
        param_ty: &Type,
        arg_ty: &Type,
        bindings: &mut Vec<(String, Type)>,
    ) {
        if let Type::TypeVar(name) = param_ty {
            for (n, ty) in bindings.iter_mut() {
                if n == name && *ty == Type::None {
                    *ty = arg_ty.clone();
                    return;
                }
            }
        }
    }

    fn synth_attr(&mut self, attr: &AttrExpr) -> crate::errors::Result<Type> {
        let obj_ty = self.synth(&attr.object)?;
        match &obj_ty {
            Type::Data(data) => {
                for (name, ty) in &data.fields {
                    if name == &attr.attr {
                        return Ok(ty.clone());
                    }
                }
                if let Some(method_ty) = self.env.lookup_method(&data.name, &attr.attr) {
                    return Ok(method_ty.clone());
                }
                Err(CompileError::new(Diagnostic::error(
                    format!("type {} has no field or method '{}'", data.name, attr.attr),
                    attr.span,
                )))
            }
            Type::Module(_) | Type::Unknown => Ok(Type::Unknown),
            _ => Err(CompileError::new(Diagnostic::error(
                format!("cannot access attribute on {obj_ty}"),
                attr.span,
            ))),
        }
    }

    fn resolve_type_expr(&self, type_expr: &TypeExpr) -> crate::errors::Result<Type> {
        match type_expr {
            TypeExpr::Named(name, span) => match name.as_str() {
                "Int" => Ok(Type::Int),
                "Float" => Ok(Type::Float),
                "Str" => Ok(Type::Str),
                "Bool" => Ok(Type::Bool),
                "None" => Ok(Type::None),
                _ => self.env.lookup_type(name).cloned().ok_or_else(|| {
                    CompileError::new(Diagnostic::error(format!("unknown type: {name}"), *span))
                }),
            },
            TypeExpr::Parameterized(name, args, span) => {
                if name == "List" && args.len() == 1 {
                    let inner = self.resolve_type_expr(&args[0])?;
                    Ok(Type::List(Box::new(inner)))
                } else {
                    Err(CompileError::new(Diagnostic::error(
                        format!("parameterized types not yet supported: {name}"),
                        *span,
                    )))
                }
            }
            TypeExpr::Function(_, _, span) => Err(CompileError::new(Diagnostic::error(
                "function types not yet supported in annotations",
                *span,
            ))),
        }
    }

    fn resolve_type_expr_with_typevars(
        &self,
        type_expr: &TypeExpr,
        type_params: &[TypeParamDef],
    ) -> crate::errors::Result<Type> {
        match type_expr {
            TypeExpr::Named(name, _)
                if type_params.iter().any(|tp| tp.name == *name) =>
            {
                Ok(Type::TypeVar(name.clone()))
            }
            _ => self.resolve_type_expr(type_expr),
        }
    }

    fn resolve_type_expr_with_tc_params(
        &self,
        type_expr: &TypeExpr,
        tc_params: &[String],
    ) -> crate::errors::Result<Type> {
        match type_expr {
            TypeExpr::Named(name, _) if tc_params.contains(name) => {
                Ok(Type::TypeVar(name.clone()))
            }
            _ => self.resolve_type_expr(type_expr),
        }
    }

    fn substitute_typevar(&self, ty: &Type, param: &str, concrete: &Type) -> Type {
        match ty {
            Type::TypeVar(name) if name == param => concrete.clone(),
            Type::Function(f) => Type::Function(FunctionType {
                type_params: f.type_params.clone(),
                param_types: f
                    .param_types
                    .iter()
                    .map(|t| self.substitute_typevar(t, param, concrete))
                    .collect(),
                return_type: Box::new(self.substitute_typevar(&f.return_type, param, concrete)),
            }),
            other => other.clone(),
        }
    }

    pub fn lookup_type(&self, name: &str) -> Option<String> {
        self.env.lookup(name).map(|b| format!("{}", b.ty))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lexer::Lexer;
    use crate::parser::parser::Parser;
    use crate::source::SourceFile;

    fn check(input: &str) -> crate::errors::Result<()> {
        let source = SourceFile::from_string(input);
        let tokens = Lexer::new(&source).tokenize()?;
        let module = Parser::new(tokens, &source).parse_module()?;
        TypeChecker::new().check_module(&module)
    }

    #[test]
    fn test_let_int() {
        assert!(check("let x: Int = 42\n").is_ok());
    }

    #[test]
    fn test_let_type_mismatch() {
        let result = check("let x: Int = \"hello\"\n");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.diagnostic.message.contains("type mismatch"));
    }

    #[test]
    fn test_let_inferred() {
        assert!(check("let x = 42\n").is_ok());
    }

    #[test]
    fn test_undefined_variable() {
        let result = check("let x: Int = y\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_func_def_and_call() {
        let input = "def add(a: Int, b: Int) -> Int:\n    return a + b\nlet x: Int = add(1, 2)\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_func_arg_mismatch() {
        let input = "def foo(a: Int) -> Int:\n    return a\nfoo(\"hello\")\n";
        let result = check(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_immutable_assign() {
        let input = "let x: Int = 42\nx = 10\n";
        let result = check(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().diagnostic.message.contains("immutable"));
    }

    #[test]
    fn test_mutable_assign() {
        let input = "mut x: Int = 42\nx = 10\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_data_decl_and_construct() {
        let input = "data Point:\n    x: Int\n    y: Int\nlet p: Point = Point(1, 2)\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_data_field_access() {
        let input = "data Point:\n    x: Int\n    y: Int\nlet p: Point = Point(1, 2)\nlet a: Int = p.x\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_arithmetic_type_checking() {
        assert!(check("let x: Int = 1 + 2\n").is_ok());
        assert!(check("let x: Float = 1.0 + 2.0\n").is_ok());
        assert!(check("let x: Str = \"a\" + \"b\"\n").is_ok());
    }

    #[test]
    fn test_arithmetic_type_mismatch() {
        let result = check("let x: Int = 1 + \"hello\"\n");
        assert!(result.is_err());
    }

    #[test]
    fn test_comparison() {
        assert!(check("let x: Bool = 1 == 2\n").is_ok());
        assert!(check("let x: Bool = 1 < 2\n").is_ok());
    }

    #[test]
    fn test_boolean_logic() {
        assert!(check("let x: Bool = True and False\n").is_ok());
        assert!(check("let x: Bool = not True\n").is_ok());
    }

    #[test]
    fn test_no_instance_error() {
        let input = "data Foo:\n    x: Int\nlet a: Foo = Foo(1)\nlet b: Foo = a + a\n";
        let result = check(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().diagnostic.message.contains("no instance of Add"));
    }

    #[test]
    fn test_user_typeclass_and_instance() {
        let input = "data Vec2:\n    x: Int\n    y: Int\nimpl Add[Vec2, Vec2, Vec2]:\n    def add(self: Vec2, other: Vec2) -> Vec2:\n        return Vec2(self.x + other.x, self.y + other.y)\nlet a: Vec2 = Vec2(1, 2)\nlet b: Vec2 = Vec2(3, 4)\nlet c: Vec2 = a + b\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_typeclass_decl_and_instance() {
        let input = "typeclass Show[T]:\n    def show(self: T, other: T) -> Str\ndata Num:\n    val: Int\nimpl Show[Num]:\n    def show(self: Num, other: Num) -> Str:\n        return \"num\"\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_constrained_generic_function() {
        let input = "def double[T: Add](x: T) -> T:\n    return x + x\nlet a: Int = double(5)\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_constrained_generic_with_float() {
        let input = "def double[T: Add](x: T) -> T:\n    return x + x\nlet a: Float = double(3.14)\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_constrained_generic_missing_instance() {
        // Bool has no Add instance, so calling double(True) should fail
        let input = "def double[T: Add](x: T) -> T:\n    return x + x\ndouble(True)\n";
        let result = check(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().diagnostic.message.contains("does not satisfy"));
    }

    #[test]
    fn test_unconstrained_generic_rejects_operator() {
        // T without Add constraint cannot use +
        let input = "def double[T](x: T) -> T:\n    return x + x\n";
        let result = check(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().diagnostic.message.contains("no instance of Add"));
    }

    #[test]
    fn test_cross_type_operator() {
        // Str * Int -> Str (different input and output types)
        let input = "typeclass Repeat[T, N, Out]:\n    def repeat(self: T, n: N) -> Out\nimpl Repeat[Str, Int, Str]:\n    def repeat(self: Str, n: Int) -> Str:\n        return self\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_constrained_generic_with_user_type() {
        let input = "data Vec2:\n    x: Int\n    y: Int\nimpl Add[Vec2, Vec2, Vec2]:\n    def add(self: Vec2, other: Vec2) -> Vec2:\n        return Vec2(self.x + other.x, self.y + other.y)\ndef double[T: Add](x: T) -> T:\n    return x + x\nlet v: Vec2 = double(Vec2(1, 2))\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_python_import() {
        assert!(check("import os\n").is_ok());
    }

    #[test]
    fn test_python_import_attr_access() {
        let input = "import os\nlet p = os.path\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_python_from_import() {
        let input = "from os.path import join\nlet x = join(\"a\", \"b\")\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_python_import_alias() {
        let input = "import os as operating_system\nlet p = operating_system.getcwd\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_inherent_method() {
        let input = "data Point:\n    x: Int\n    y: Int\nimpl Point:\n    def sum(self: Point) -> Int:\n        return self.x + self.y\nlet p: Point = Point(1, 2)\nlet s: Int = p.sum()\n";
        assert!(check(input).is_ok());
    }

    #[test]
    fn test_inherent_method_unknown_type() {
        let input = "impl Foo:\n    def bar(self: Int) -> Int:\n        return 1\n";
        let result = check(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().diagnostic.message.contains("unknown type"));
    }
}
