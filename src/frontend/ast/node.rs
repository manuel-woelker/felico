use crate::frontend::ast::AstData;
use crate::infra::location::Location;
use crate::interpreter::core_definitions::TYPE_UNKNOWN;
use crate::interpreter::value::Type;
use std::fmt::Debug;

#[derive(Debug, Clone)]
pub struct AstNode<T: AstData>  {
    pub data: Box<T>,
    pub ty: Type,
    pub location: Location,
}

impl <T: AstData> AstNode<T> {
    pub fn new(data: T, location: Location)-> Self {
        Self {
            data: Box::new(data),
            location,
            ty: TYPE_UNKNOWN.clone(),
        }
    }
}
