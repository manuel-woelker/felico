use crate::frontend::ast::types::{Type, TypeKind};

pub struct TypeChecker {}

impl TypeChecker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_assignable_to(&self, source: &Type, destination: &Type) -> bool {
        if matches!(destination.kind(), TypeKind::Any) {
            return true;
        }
        if source == destination {
            return true;
        }
        false
    }
}
