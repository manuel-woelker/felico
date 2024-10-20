use crate::infra::result::{bail, FelicoResult};
use directories::ProjectDirs;
use rand::random;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::read::read_zipfile_from_stream;

pub struct CCompiler {
    zig_binary: PathBuf,
}

const ZIG_VERSION: &str = "0.14.0-dev.1951+857383689";

impl CCompiler {
    pub fn new() -> FelicoResult<CCompiler> {
        let project_dirs =
            ProjectDirs::from("", "", "Felico").ok_or("Could not find home directory")?;
        let compiler_dir = project_dirs
            .cache_dir()
            .join("compiler")
            .join("zig-".to_string() + ZIG_VERSION);
        if !compiler_dir.exists() {
            let temp_dir = compiler_dir.join(format!("../tmp-{}", random::<u128>()));
            println!("Zig Compiler not found, downloading now to '{compiler_dir:?}'");
            let download_url =
                format!("https://ziglang.org/builds/zig-windows-x86_64-{ZIG_VERSION}.zip");
            let mut response = attohttpc::get(&download_url)
                .send()
                .map_err(|e| format!("Unable to download '{download_url}': {e}"))?;
            if !response.is_success() {
                bail!(
                    "Error downloading '{download_url}': Status code {}",
                    response.status()
                );
            }
            while let Some(mut zip_file) = read_zipfile_from_stream(&mut response)
                .map_err(|e| format!("Unable to read downloaded archive: {e}"))?
            {
                if !zip_file.is_file() {
                    continue;
                }
                let Some(filename) = zip_file.enclosed_name() else {
                    bail!("Could not read zipfile entry: {}", zip_file.name());
                };
                // strip root directory with version, etc
                let new_file_name: PathBuf = filename.components().skip(1).collect();
                let output_file = temp_dir.join(new_file_name);
                let Some(parent_directory) = output_file.parent() else {
                    bail!("Could not get parent directory of: {:?}", output_file);
                };
                std::fs::create_dir_all(parent_directory)?;
                let mut file = std::fs::File::create(output_file)?;
                std::io::copy(&mut zip_file, &mut file)?;
            }
            std::fs::rename(&temp_dir, &compiler_dir)?;
        }
        Ok(CCompiler {
            zig_binary: compiler_dir.join("zig.exe"),
        })
    }

    pub fn compile<S: AsRef<Path>, D: AsRef<Path>>(
        &self,
        source_file: S,
        destination_file: D,
    ) -> FelicoResult<()> {
        let mut command = Command::new(&self.zig_binary);
        command
            .arg("cc")
            .arg("-o")
            .arg(destination_file.as_ref())
            .arg(source_file.as_ref())
            .arg("-target")
            .arg("x86_64-windows-gnu");
        let status = command.status()?;
        if !status.success() {
            bail!("Compilation failed")
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use crate::backend::c::c_compiler::CCompiler;
    use rand::random;
    use std::env::temp_dir;
    use std::process::Command;

    #[test]
    pub fn test_initialization() {
        let _compiler = CCompiler::new().unwrap();
    }

    #[test]
    pub fn test_compilation() {
        let compiler = CCompiler::new().unwrap();
        let output_path = temp_dir().join(format!("felico_test/{}", random::<u128>()));
        std::fs::create_dir_all(&output_path).unwrap();
        let output_file = output_path.join("hello.exe");
        let input_file = std::path::absolute("tests/hello.c").unwrap();
        compiler.compile(input_file, &output_file).unwrap();
        let status = Command::new(output_file).status().unwrap();
        assert_eq!(status.code(), Some(123));
    }
}
