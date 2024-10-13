use crate::frontend::parse::parser::Parser;
use crate::frontend::resolve::module_manifest::ModuleManifest;
use crate::frontend::resolve::resolver::Resolver;
use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFile;
use crate::interpret::core_definitions::TypeFactory;

pub fn compile_module(source_file: SourceFile) -> FelicoResult<ModuleManifest> {
    let type_factory = &TypeFactory::new();
    let mut parser = Parser::new(source_file, type_factory)?;
    let mut program = parser.parse_module()?;
    let mut resolver = Resolver::new(type_factory.clone());
    resolver.resolve_program(&mut program)?;

    resolver.get_module_manifest()
}
