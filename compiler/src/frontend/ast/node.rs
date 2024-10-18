use crate::frontend::ast::AstData;
use crate::infra::source_span::SourceSpan;
use crate::model::types::Type;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AstNode<'a, T: AstData> {
    pub data: Box<T>,
    pub ty: Type<'a>,
    pub location: SourceSpan,
}

impl<'a, T: AstData> AstNode<'a, T> {
    pub fn new(data: T, location: SourceSpan, ty: Type<'a>) -> Self {
        Self {
            data: Box::new(data),
            location,
            ty,
        }
    }
}
