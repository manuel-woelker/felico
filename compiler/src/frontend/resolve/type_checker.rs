use crate::frontend::ast::types::{Type, TypeKind};

pub struct TypeChecker {}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn is_assignable_to(&self, source: &Type, destination: &Type) -> bool {
        if source == destination {
            return true;
        }
        // Bottom Type never is assignable to everything, since it never actually exists
        if matches!(source.kind(), TypeKind::Never) {
            return true;
        }
        // Any is assignable to anything
        if matches!(destination.kind(), TypeKind::Any) {
            return true;
        }
        // Resolution failed do not produce additional errors
        if matches!(destination.kind(), TypeKind::Unresolved) {
            return true;
        }
        // Resolution failed do not produce additional errors
        if matches!(source.kind(), TypeKind::Unresolved) {
            return true;
        }
        false
    }
}
