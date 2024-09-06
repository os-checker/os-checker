use crate::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RustToolchains {
    // FIXME: 这个和 main.rs 中的 &'static 不一致
    pub host: Rustc,
    pub installed: Vec<RustToolchain>,
}

// [toolchain]
// channel = "nightly-2020-07-10"
// components = [ "rustfmt", "rustc-dev" ]
// targets = [ "wasm32-unknown-unknown", "thumbv2-none-eabi" ]
// profile = "minimal"
#[derive(Deserialize, Serialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct RustToolchain {
    pub channel: XString,
    pub profile: Option<XString>,
    pub targets: Option<Vec<String>>,
    pub components: Option<Vec<String>>,
    pub toml_path: Utf8PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rustc {
    pub version: String,
    pub commit_hash: String,
    pub commit_date: String,
    pub host: String,
    pub release: String,
    pub llvm_version: String,
}
