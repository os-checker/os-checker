use crate::utils::{group_by, new_map_with_cap, pkg_cmdidx, repo_pkgidx, UserRepo};
use camino::Utf8Path;
use indexmap::IndexMap;
use os_checker_types::{Cmd, JsonOutput, Kind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod inner;
use inner::{Checkers, FeaturesSets, Pkgs, Targets};

#[cfg(test)]
mod tests;

/// 读取 src_dir 的所有 JSON，合并成一个新的 JSON，并写到 target_dir。
/// 新 JSON 的文件名为 basic.json。
pub fn write_batch(src_dir: &Utf8Path, target_dir: &Utf8Path) -> crate::Result<()> {
    let vec_bytes = crate::json_paths(src_dir.as_str())?
        .into_iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()?;

    let mut batch = Vec::<Basic>::with_capacity(24);
    for bytes in &vec_bytes {
        batch.push(serde_json::from_slice::<Basic>(bytes)?);
    }

    if batch.is_empty() {
        info!("无 batch basic 可合并");
        return Ok(());
    }

    let kinds = batch[0].kinds.clone();

    let len = batch.len();
    let (mut batch_pkgs, mut batch_checkers) = (Vec::with_capacity(len), Vec::with_capacity(len));
    let (mut batch_targets, mut batch_features_sets) =
        (Vec::with_capacity(len), Vec::with_capacity(len));
    for b in batch {
        batch_pkgs.push(b.pkgs);
        batch_checkers.push(b.checkers);
        batch_targets.push(b.targets);
        batch_features_sets.push(b.features_sets);
    }
    let pkgs = Pkgs::merge_batch(batch_pkgs);
    let checkers = Checkers::merge_batch(batch_checkers);
    let targets = Targets::merge_batch(batch_targets);
    let features_sets = FeaturesSets::merge_batch(batch_features_sets);
    let merged = Basic {
        pkgs,
        checkers,
        targets,
        features_sets,
        kinds,
    };

    let path = target_dir.join("basic.json");
    let file = std::fs::File::create(&path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &merged)?;

    info!("成功把 batch config 合并: src_dir={src_dir} merged={path}");
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Basic {
    pkgs: Pkgs,
    checkers: Checkers,
    targets: Targets,
    features_sets: FeaturesSets,
    kinds: Kinds,
}

impl Basic {
    pub fn print(&self) {
        info!("{}", serde_json::to_string_pretty(self).unwrap());
    }
}

/// 所有仓库的架构统计
pub fn all(json: &JsonOutput) -> Basic {
    let kinds = Kinds::new(json);
    let pkgs = Pkgs::new(&json.cmd, json);
    let checkers = Checkers::new(&json.cmd);
    let targets = Targets::new(&json.cmd);
    let features_sets = FeaturesSets::new(&json.cmd);
    Basic {
        pkgs,
        checkers,
        targets,
        features_sets,
        kinds,
    }
}

/// 按仓库的架构统计
pub fn by_repo(json: &JsonOutput) -> Vec<(UserRepo, Basic)> {
    let kinds = Kinds::new(json);
    let map = group_by(&json.cmd, |cmd| repo_pkgidx(json, cmd.package_idx));
    let mut v = Vec::<(UserRepo, Basic)>::with_capacity(map.len());

    for (user_repo, cmds) in map {
        let iter = cmds.iter().copied();
        let pkgs = Pkgs::new(iter.clone(), json);
        let checkers = Checkers::new(iter.clone());
        let targets = Targets::new(iter.clone());
        let features_sets = FeaturesSets::new(iter);

        v.push((
            user_repo,
            Basic {
                pkgs,
                checkers,
                targets,
                features_sets,
                kinds: kinds.clone(),
            },
        ));
    }

    v
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Kinds {
    // #[serde(flatten)]
    // pub raw: RawKinds,
    pub columns: Vec<Column>,
}

impl Kinds {
    fn new(json: &JsonOutput) -> Self {
        Kinds {
            columns: columns(&json.env.kinds.order),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Column {
    pub field: Kind,
    pub header: String,
}

fn columns(order: &[Kind]) -> Vec<Column> {
    let rename = indexmap::indexmap! {
        Kind::Unformatted => "Unformatted",
        Kind::ClippyWarn => "Clippy (Warn)",
        Kind::ClippyError => "Clippy (Error)",
        Kind::LockbudPossibly => "Lockbud (Possibly)",
        Kind::LockbudProbably => "Lockbud (Probably)",
    };
    let mut checkers = IndexMap::<_, _, ahash::RandomState>::from_iter(
        order.iter().map(|kind| (*kind, kind.as_str())),
    );
    checkers.extend(rename);
    checkers
        .into_iter()
        .map(|(field, header)| Column {
            field,
            header: header.to_owned(),
        })
        .collect()
}
