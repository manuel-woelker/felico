use crate::ast_node::AstNode;

pub struct Identifier {
    pub name: String,
}

pub type IdentifierNode<'source> = AstNode<'source, Identifier>;
