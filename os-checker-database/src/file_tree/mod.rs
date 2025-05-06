use crate::utils::{group_by, new_map_with_cap, pkg_cmdidx, target_cmdidx, vec_ref_to_vec_owned};
use os_checker_types::{out_json::file_tree::*, Data as RawData, JsonOutput};

#[cfg(test)]
mod tests;

pub fn all_targets(json: &JsonOutput) -> FileTree {
    let data: Vec<_> = json.data.iter().collect();
    inner(json, &data)
}

pub fn split_by_target(json: &JsonOutput) -> Vec<(&str, FileTree)> {
    let group_by_target = group_by(&json.data, |d| target_cmdidx(json, d.cmd_idx));
    let mut v = Vec::with_capacity(group_by_target.len());

    for (target, data) in group_by_target {
        v.push((target, inner(json, &data)));
    }
    v
}

fn inner<'a>(json: &'a JsonOutput, data: &[&'a RawData]) -> FileTree {
    let kinds_order = &json.env.kinds.order;

    let group_by_pkg = group_by(data, |d| pkg_cmdidx(json, d.cmd_idx));
    let mut v = Vec::with_capacity(group_by_pkg.len());

    for (pkg, data) in group_by_pkg {
        let count_pkg = data.len();

        let group_by_file_feat = group_by(&data, |d| (&*d.file, &*json.cmd[d.cmd_idx].features));
        let mut reports = Vec::with_capacity(group_by_file_feat.len());

        for ((file, feat), data_file) in group_by_file_feat {
            let group_by_kind = group_by(data_file, |d| d.kind);
            let mut kinds = new_map_with_cap(group_by_kind.len());

            for (kind, data_kind) in group_by_kind {
                kinds.insert(kind, data_kind.iter().map(|d| &*d.raw).collect());
            }

            let count = kinds.values().map(|v: &Vec<_>| v.len()).sum();
            let features = feat.join(" ");
            reports.push(RawReport {
                file: file.to_owned(),
                features,
                count,
                kinds: kinds
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().map(String::from).collect()))
                    .collect(),
            });
        }

        v.push(Data {
            pkg: pkg.into(),
            count: count_pkg,
            raw_reports: reports,
        });
    }

    // FIXME: sort by features?

    // 对 pkg 的计数排序
    v.sort_unstable_by(|a, b| (b.count, &*a.pkg.pkg).cmp(&(a.count, &*b.pkg.pkg)));
    // 对文件的计数和文件名排序
    for pkg in &mut v {
        pkg.raw_reports
            .sort_unstable_by(|a, b| (b.count, &*a.file).cmp(&(a.count, &*b.file)));
    }

    FileTree {
        data: v,
        kinds_order: kinds_order.to_owned(),
    }
}

pub fn split_by_repo(this: FileTree) -> Vec<FileTreeRepo> {
    let group_by_repo = group_by(&this.data, |d| d.pkg.to_repo());
    let mut v = Vec::with_capacity(group_by_repo.len());

    for (repo, data) in group_by_repo {
        v.push(FileTreeRepo {
            repo,
            data: vec_ref_to_vec_owned(data),
            kinds_order: this.kinds_order.clone(),
        });
    }

    v
}
