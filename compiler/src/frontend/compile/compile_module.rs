use crate::frontend::parse::parser::Parser;
use crate::frontend::resolve::module_manifest::ModuleManifest;
use crate::frontend::resolve::resolver::Resolver;
use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFile;
use crate::model::workspace::Workspace;

pub fn compile_module<'ws>(
    source_file: SourceFile<'ws>,
    workspace: Workspace<'ws>,
) -> FelicoResult<ModuleManifest<'ws>> {
    let mut parser = Parser::new(source_file, workspace)?;
    let mut program = parser.parse_module()?;
    let mut resolver = Resolver::new(workspace);
    resolver.resolve_program(&mut program)?;

    resolver.get_module_manifest()
}
