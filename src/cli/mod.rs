use argh::FromArgs;
use cargo_metadata::camino::Utf8PathBuf;

pub fn args() -> Args {
    argh::from_env()
}

#[derive(FromArgs, Debug)]
/// Reach new heights.
pub struct Args {
    /// an optional height
    #[argh(option, default = r#"Utf8PathBuf::from("repos.yaml")"#)]
    pub config: Utf8PathBuf,
}
