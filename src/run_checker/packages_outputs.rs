use super::{CheckerTool, Output};
use crate::repo::TOOLS;
use color_eyre::owo_colors::OwoColorize;
use indexmap::IndexMap;
use regex::Regex;
use std::sync::LazyLock;

pub type PackageName = String;

pub struct Outputs {
    inner: Vec<Output>,
}

impl Outputs {
    fn new() -> Self {
        Outputs {
            inner: Vec::with_capacity(TOOLS),
        }
    }

    pub fn count(&self) -> usize {
        self.inner.iter().map(|out| out.count).sum()
    }

    pub fn as_slice(&self) -> &[Output] {
        &self.inner
    }

    pub fn push(&mut self, output: Output) {
        self.inner.push(output);
    }
}

pub struct PackagesOutputs {
    // key 为 pkg_name, value 为 outputs
    map: IndexMap<PackageName, Outputs>,
}

impl PackagesOutputs {
    pub fn new() -> Self {
        PackagesOutputs {
            map: IndexMap::with_capacity(4),
        }
    }

    pub fn count(&self) -> usize {
        // 这里的计数应该包括 CheckerTool::Cargo
        self.values().map(Outputs::count).sum()
    }

    /// This should be called after all outputs of all packages finish.
    pub fn sort_by_name_and_checkers(&mut self) {
        self.sort_unstable_keys();
        for outputs in self.values_mut() {
            outputs.inner.sort_unstable_by_key(|o| o.resolve.checker);
        }
    }

    pub fn push_output_with_cargo(&mut self, output: Output) {
        let pkg_name = output.resolve.pkg_name.as_str();
        if let Some(v) = self.get_mut(pkg_name) {
            if let Some(stderr_parsed) = cargo_stderr_stripped(&output) {
                if let Some(cargo) = v
                    .inner
                    .iter_mut()
                    .find(|o| o.resolve.checker == CheckerTool::Cargo)
                {
                    cargo.update_cargo(stderr_parsed, &output);
                } else {
                    v.push(output.new_cargo(stderr_parsed));
                }
            }

            v.push(output);
        } else {
            let pkg_name = pkg_name.to_owned();
            let mut outputs = Outputs::new();

            if let Some(stderr_parsed) = cargo_stderr_stripped(&output) {
                outputs.push(output.new_cargo(stderr_parsed));
            }

            outputs.push(output);
            self.insert(pkg_name, outputs);
        }
    }
}

/// Some means there is a cargo erroneous output to be created or updated.
fn cargo_stderr_stripped(output: &Output) -> Option<String> {
    let resolve = &output.resolve;
    let raw_stderr = output.raw.stderr.as_slice();
    let stderr = String::from_utf8_lossy(raw_stderr);

    trace!(%resolve.pkg_name, %resolve.pkg_dir);
    trace!(
        success = %(if output.raw.status.success() {
            "true".bright_green().to_string()
        } else {
            "false".bright_red().to_string()
        }),
        resolve.cmd = %resolve.cmd.bright_black().italic()
    );
    trace!("stderr=\n{stderr}\n");

    let stderr_stripped = strip_ansi_escapes::strip(raw_stderr);
    let stderr = String::from_utf8_lossy(&stderr_stripped);
    // stderr 包含额外的 error: 信息，那么将所有 stderr 内容 作为 cargo 的检查结果
    RE.is_match(&stderr).then(|| stderr.into_owned())
}

static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new("\nerror: ").unwrap());

impl std::ops::Deref for PackagesOutputs {
    type Target = IndexMap<PackageName, Outputs>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl std::ops::DerefMut for PackagesOutputs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
