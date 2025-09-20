use crate::ast_node::AstNode;
use crate::identifier::IdentifierNode;

pub struct FunDefinition<'source> {
    pub name: IdentifierNode<'source>,
}

pub type FunDefinitionNode<'source> = AstNode<'source, FunDefinition<'source>>;
