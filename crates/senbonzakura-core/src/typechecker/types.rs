use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    Str,
    Bool,
    None,
    Function(FunctionType),
    Data(DataType),
    TypeVar(String),
    List(Box<Type>),
    Module(ModuleType),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub type_params: Vec<(String, Vec<String>)>,
    pub param_types: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataType {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct TypeclassDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<TypeclassMethodSig>,
}

#[derive(Debug, Clone)]
pub struct TypeclassMethodSig {
    pub name: String,
    pub param_types: Vec<Type>,
    pub return_type: Type,
}

#[derive(Debug, Clone)]
pub struct InstanceDef {
    pub typeclass_name: String,
    pub type_args: Vec<Type>,
    pub is_builtin: bool,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Str => write!(f, "Str"),
            Type::Bool => write!(f, "Bool"),
            Type::None => write!(f, "None"),
            Type::Function(func) => {
                write!(f, "(")?;
                for (i, p) in func.param_types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") -> {}", func.return_type)
            }
            Type::Data(data) => write!(f, "{}", data.name),
            Type::TypeVar(name) => write!(f, "{name}"),
            Type::List(inner) => write!(f, "List[{inner}]"),
            Type::Module(m) => write!(f, "module '{}'", m.name),
            Type::Unknown => write!(f, "Unknown"),
        }
    }
}
