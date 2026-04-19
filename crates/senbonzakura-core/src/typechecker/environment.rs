use std::collections::HashMap;

use crate::typechecker::types::{InstanceDef, Type, TypeclassDef};

#[derive(Debug, Clone)]
pub struct Environment {
    bindings: HashMap<String, Binding>,
    type_defs: HashMap<String, Type>,
    typeclasses: HashMap<String, TypeclassDef>,
    instances: HashMap<String, Vec<InstanceDef>>,
    methods: HashMap<(String, String), Type>,
    parent: Option<Box<Environment>>,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub ty: Type,
    pub mutable: bool,
}

impl Environment {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            type_defs: HashMap::new(),
            typeclasses: HashMap::new(),
            instances: HashMap::new(),
            methods: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: Environment) -> Self {
        Self {
            bindings: HashMap::new(),
            type_defs: HashMap::new(),
            typeclasses: HashMap::new(),
            instances: HashMap::new(),
            methods: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, ty: Type, mutable: bool) {
        self.bindings.insert(name, Binding { ty, mutable });
    }

    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        self.bindings
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    pub fn define_type(&mut self, name: String, ty: Type) {
        self.type_defs.insert(name, ty);
    }

    pub fn lookup_type(&self, name: &str) -> Option<&Type> {
        self.type_defs
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_type(name)))
    }

    pub fn define_typeclass(&mut self, name: String, def: TypeclassDef) {
        self.typeclasses.insert(name, def);
    }

    pub fn lookup_typeclass(&self, name: &str) -> Option<&TypeclassDef> {
        self.typeclasses
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup_typeclass(name)))
    }

    pub fn define_instance(&mut self, typeclass: String, def: InstanceDef) {
        self.instances
            .entry(typeclass)
            .or_default()
            .push(def);
    }

    pub fn lookup_instance(&self, typeclass: &str, type_args: &[Type]) -> Option<&InstanceDef> {
        if let Some(instances) = self.instances.get(typeclass) {
            for inst in instances {
                if inst.type_args.len() == type_args.len()
                    && inst.type_args.iter().zip(type_args).all(|(a, b)| a == b)
                {
                    return Some(inst);
                }
            }
        }
        self.parent
            .as_ref()
            .and_then(|p| p.lookup_instance(typeclass, type_args))
    }

    pub fn lookup_instance_by_prefix(
        &self,
        typeclass: &str,
        prefix: &[Type],
    ) -> Option<&InstanceDef> {
        if let Some(instances) = self.instances.get(typeclass) {
            for inst in instances {
                if inst.type_args.len() >= prefix.len()
                    && inst.type_args[..prefix.len()]
                        .iter()
                        .zip(prefix)
                        .all(|(a, b)| a == b)
                {
                    return Some(inst);
                }
            }
        }
        self.parent
            .as_ref()
            .and_then(|p| p.lookup_instance_by_prefix(typeclass, prefix))
    }

    pub fn define_method(&mut self, type_name: String, method_name: String, ty: Type) {
        self.methods.insert((type_name, method_name), ty);
    }

    pub fn lookup_method(&self, type_name: &str, method_name: &str) -> Option<&Type> {
        self.methods
            .get(&(type_name.to_string(), method_name.to_string()))
            .or_else(|| {
                self.parent
                    .as_ref()
                    .and_then(|p| p.lookup_method(type_name, method_name))
            })
    }

    pub fn into_parent(self) -> Option<Environment> {
        self.parent.map(|p| *p)
    }
}
