use crate::frontend::ast::types::Type;
use crate::frontend::ast::AstData;
use crate::infra::source_span::SourceSpan;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AstNode<T: AstData> {
    pub data: Box<T>,
    pub ty: Type,
    pub location: SourceSpan,
}

impl<T: AstData> AstNode<T> {
    pub fn new(data: T, location: SourceSpan, ty: Type) -> Self {
        Self {
            data: Box::new(data),
            location,
            ty,
        }
    }
}
