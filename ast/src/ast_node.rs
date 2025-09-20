use felico_source::file_location::FileLocation;

pub struct AstNode<'source, T> {
    pub location: FileLocation<'source>,
    pub node: T,
}
