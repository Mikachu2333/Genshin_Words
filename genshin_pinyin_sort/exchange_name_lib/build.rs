extern crate embed_resource;
use std::path::PathBuf;

fn main() {
    #[cfg(windows)]
    {
        if std::env::var("CARGO_CFG_TARGET_ENV")
            .unwrap_or("".into())
            .to_lowercase()
            == "gnu"
        {
            panic!("GNU toolchain is not supported on Win. Please use the MSVC toolchain.");
        }

        let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is not set"));
        let generated_rc = prepare_resource_files(&out_dir).expect("Failed to prepare RC inputs");

        match embed_resource::compile(generated_rc, embed_resource::NONE) {
            embed_resource::CompilationResult::NotAttempted(x) => {
                panic!("{}", x)
            }
            embed_resource::CompilationResult::Failed(x) => {
                panic!("{}", x)
            }
            _ => {}
        };
    }
}

#[cfg(windows)]
fn prepare_resource_files(out_dir: &std::path::Path) -> Result<String, std::io::Error> {
    let version = env!("CARGO_PKG_VERSION");
    // "0.1.0" -> "0,1,0,0"
    let version_commas = version.replace('.', ",") + ",0";

    let header = format!(
        "#define VERSION_INT  {}\n#define VERSION_STR  \"{}\"\n",
        version_commas, version
    );

    let version_h_path = out_dir.join("version.h");
    std::fs::write(&version_h_path, header)?;

    let rc_template = std::fs::read_to_string("res.rc")?;
    let generated_rc_path = out_dir.join("res.generated.rc");
    std::fs::write(&generated_rc_path, rc_template)?;

    Ok(generated_rc_path.to_string_lossy().into_owned())
}
