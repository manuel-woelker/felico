use crate::frontend::ast::AstData;
use crate::infra::source_span::SourceSpan;
use crate::model::types::Type;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AstNode<'ws, T: AstData> {
    pub data: Box<T>,
    pub ty: Type<'ws>,
    pub location: SourceSpan<'ws>,
}

impl<'ws, T: AstData> AstNode<'ws, T> {
    pub fn new(data: T, location: SourceSpan<'ws>, ty: Type<'ws>) -> Self {
        Self {
            data: Box::new(data),
            location,
            ty,
        }
    }
}
