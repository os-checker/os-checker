//! cargo-audit only needs Cargo.lock, and Cargo.lock is only
//! generated under workspace root dir (if we think single pkg
//! is its own workspace).
//!
//! But it's possible there is no Cargo.lock there for some reasons.
//!
//! So here are the steps to do audit check:
//! * for each workspace root dir, see if there is a Cargo.lock,
//! * if not, call cargo-generate-lockfile to get one,
//! * call cargo-audit with and without --json to get the results
//!   * both results will be displayed on WebUI
//!   * json result helps to identify the problematic dependencies,
//!     and we search each pkg denpendencies resolution for the
//!     problematic denpendencies,
//! * target is not used with cargo-audit, so the pkg audit result
//!   will repeat for each target.

use crate::{utils::cmd_run, Result, XString};
use camino::{Utf8Path, Utf8PathBuf};
use duct::cmd;
use eyre::Context;
use indexmap::{IndexMap, IndexSet};
use rustsec::{
    cargo_lock::{
        dependency::graph::{Graph, NodeIndex, Nodes},
        Dependency,
    },
    Report,
};
use std::rc::Rc;

#[instrument(level = "info")]
fn generate_lockfile(workspace_dir: &Utf8Path) -> Result<()> {
    _ = cmd!("cargo", "generate-lockfile")
        .dir(workspace_dir)
        .run()?;
    Ok(())
}

#[allow(dead_code)]
pub struct CargoAudit {
    problematic_pkgs: Vec<XString>,
    tree: String,
    json: String,
    output: String,
    /// parsed from json
    report: Report,
    lock_file: Utf8PathBuf,
}

impl std::fmt::Debug for CargoAudit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CargoAudit")
            .field("problematic_pkgs", &self.problematic_pkgs)
            .field("tree", &self.tree)
            .field("lock_file", &self.lock_file)
            .finish()
    }
}

impl CargoAudit {
    pub fn is_problematic(&self) -> bool {
        !self.problematic_pkgs.is_empty()
    }

    // FIXME: There might possibily be a Cargo error output, because cargo-audit itself
    // might fail due to like Cargo.lock format version changes.
    // So it'd be better to emit normal output or Cargo output.
    // See https://github.com/os-checker/os-checker/issues/42#issuecomment-2408453064
    pub fn output(&self) -> &str {
        &self.output
    }

    pub fn lock_file(&self) -> &Utf8Path {
        &self.lock_file
    }

    pub fn new(workspace_dir: &Utf8Path) -> Result<Rc<Self>> {
        cargo_audit(workspace_dir).map(Rc::new)
    }

    pub fn cmd(&self) -> String {
        "cargo audit".to_owned()
    }

    /// NOTE: this is not meaningful because cargo-audit only needs
    /// Cargo.lock, but members may not have it.
    pub fn cmd_expr(&self) -> duct::Expression {
        let mut path = self.lock_file.clone();
        path.pop();
        cmd!("cargo", "audit").dir(path)
    }

    /// returns the map where the key is pkg name and the value is audit result
    pub fn new_for_pkgs<'a>(
        dirs: impl Iterator<Item = &'a Utf8PathBuf>,
    ) -> Result<IndexMap<XString, Rc<Self>>> {
        let mut map = IndexMap::new();
        for dir in dirs {
            let audit = Self::new(dir)?;
            for pkg in &audit.problematic_pkgs {
                // NOTE: there is supposed to be no pkg name aliasing.
                map.insert(pkg.clone(), audit.clone());
            }
        }
        Ok(map)
    }
}

fn cargo_audit(workspace_dir: &Utf8Path) -> Result<CargoAudit> {
    let mut lock_file = workspace_dir.to_owned();
    lock_file.push("Cargo.lock");

    let _span = error_span!("cargo_audit", lock_file = ?lock_file.canonicalize_utf8()).entered();

    if !lock_file.exists() {
        generate_lockfile(workspace_dir)?;
    }

    let json = cmd_run(
        "cargo",
        &["audit", "--json", "-c", "never"],
        workspace_dir,
        true,
    )?;

    let report: rustsec::Report = serde_json::from_str(&json)
        .with_context(|| format!("Fail to parse json as a rustsec::Report:\n{json}"))?;
    if !report.vulnerabilities.found && report.warnings.is_empty() {
        return Ok(CargoAudit {
            problematic_pkgs: vec![],
            tree: String::new(),
            json,
            output: String::new(),
            report,
            lock_file,
        });
    }

    let tree = cmd_run("cargo", &["audit", "-c", "never"], workspace_dir, true)?;

    let mut problematic = IndexSet::<Dependency>::new();
    let vulnerable = &report.vulnerabilities.list;
    problematic.extend(vulnerable.iter().map(|vul| Dependency::from(&vul.package)));
    let warnings = report.warnings.values();
    problematic.extend(warnings.flat_map(|v| v.iter().map(|w| Dependency::from(&w.package))));

    let problematic_pkgs = parse_cargo_lock(&lock_file, &problematic)?;

    let output = {
        let json = match jsonxf::pretty_print(&json) {
            Ok(json) => json,
            Err(json) => json,
        };
        format!("{tree}\n{json}")
    };

    Ok(CargoAudit {
        problematic_pkgs,
        tree,
        json,
        output,
        report,
        lock_file,
    })
}

fn parse_cargo_lock(
    lock_file: &Utf8Path,
    problematic: &IndexSet<Dependency>,
) -> Result<Vec<XString>> {
    let lockfile = rustsec::Lockfile::load(lock_file)?;

    let tree = lockfile.dependency_tree()?;
    let graph = tree.graph();
    let nodes = tree.nodes();

    // suppose local pkgs without source and  checksum
    let local_pkgs: Vec<_> = lockfile
        .packages
        .iter()
        .filter(|pkg| pkg.source.is_none())
        .collect();
    let mut problematic_local_pkgs = Vec::new();

    let mut map = IndexSet::<&Dependency>::new();
    let mut n = 0;
    for pkg in &local_pkgs {
        map.clear();
        n = 0;
        for dep in &pkg.dependencies {
            let idx = *nodes.get(dep).unwrap();
            recursive_dependencies(&mut map, idx, graph, nodes, &mut n);
        }
        // TODO: maybe we could point out which dependencies are problematic,
        // though they are in output and json.
        for dep in problematic {
            // local pkg contains a problematic dependency in the graph
            if map.contains(&dep) {
                problematic_local_pkgs.push(XString::from(pkg.name.as_str()));
            }
        }
    }

    Ok(problematic_local_pkgs)
}

fn recursive_dependencies<'a>(
    map: &mut IndexSet<&'a Dependency>,
    idx: NodeIndex,
    graph: &'a Graph,
    nodes: &Nodes,
    n: &mut usize,
) {
    *n += 1;
    for (direct_dep, dep_idx) in graph.edges(idx).map(|edge| {
        let dep = edge.weight();
        (dep, *nodes.get(dep).unwrap())
    }) {
        println!("[n={n} dep_idx={}] {}", dep_idx.index(), direct_dep.name);
        map.insert(direct_dep);
        recursive_dependencies(map, dep_idx, graph, nodes, n);
    }
}

#[test]
fn test_cargo_audit() {
    // ---- layout::audit::test_cargo_audit stdout ----
    // thread 'layout::audit::test_cargo_audit' panicked at src\layout\audit.rs:170:14:
    // called `Result::unwrap()` on an `Err` value: error: couldn't fetch advisory database: git
    //  operation failed: failed to prepare fetch: An IO error occurred when talking to the serv
    // er
    //
    //
    // Location:
    //     src\utils\mod.rs:98:9
    crate::logger::init();
    let dir = Utf8PathBuf::from_iter(["src", "layout", "tests"]);
    dbg!(cargo_audit(&dir).unwrap().problematic_pkgs);
}
