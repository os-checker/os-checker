use super::{
    CargoMessage, ClippyMessage, ClippyTag, FmtMessage, Output as RawOutput, OutputParsed,
};
use crate::output::{Cmd, Data, Kind};
use cargo_metadata::camino::Utf8Path;
use std::fmt::Write;

/// 将一次工具的检查命令推入一次 `Vec<Idx>`，并把原始输出全部推入 `Vec<Data>`。
pub fn push_idx_and_data(
    package_idx: usize,
    raw: &RawOutput,
    cmds: &mut Vec<Cmd>,
    data: &mut Vec<Data>,
) {
    // TODO: 这些等会解决
    let (cmd, arch, target_triple, features, flags) = Default::default();
    let idx_item = Cmd {
        package_idx,
        tool: raw.checker,
        count: raw.count,
        duration_ms: raw.duration_ms,
        cmd,
        arch,
        target_triple,
        features,
        flags,
    };
    let cmd_idx = cmds.len();
    cmds.push(idx_item);

    let with = WithData {
        data,
        cmd_idx,
        root: &raw.package_root,
    };
    push_data(raw, with);
}

fn push_data(out: &RawOutput, with: WithData) {
    // 由于路径的唯一性在这变得重要，需要提前归一化路径；两条思路：
    // * package_name 暗含了库的根目录，因此需要把路径的根目录去掉（选择了这条）
    // * 如果能保证都是绝对路径，那么不需要处理路径
    match &out.parsed {
        OutputParsed::Fmt(v) => push_unformatted(v, with),
        OutputParsed::Clippy(v) => push_clippy(v, with),
    };
}

struct WithData<'data, 'root> {
    data: &'data mut Vec<Data>,
    cmd_idx: usize,
    root: &'root Utf8Path,
}

/// 尽可能缩短绝对路径到相对路径
fn strip_prefix<'f>(file: &'f Utf8Path, root: &Utf8Path) -> &'f Utf8Path {
    file.strip_prefix(root).unwrap_or(file)
}

fn push_unformatted(v: &[FmtMessage], with: WithData) {
    for mes in v {
        // NOTE: 该路径似乎是绝对路径
        let file = strip_prefix(&mes.name, with.root);

        with.data.extend(raw_message_fmt(mes).map(|raw| Data {
            cmd_idx: with.cmd_idx,
            file: file.to_owned(),
            kind: Kind::Unformatted,
            raw,
        }));
    }
}

fn raw_message_fmt(mes: &FmtMessage) -> impl '_ + ExactSizeIterator<Item = String> {
    mes.mismatches.iter().map(|mis| {
        let add = "+";
        let minus = "-";
        let mut buf = String::with_capacity(128);
        _ = writeln!(
            &mut buf,
            "file: {} (original lines from {} to {})",
            mes.name, mis.original_begin_line, mis.original_end_line
        );
        for diff in prettydiff::diff_lines(&mis.original, &mis.expected).diff() {
            match diff {
                prettydiff::basic::DiffOp::Insert(s) => {
                    for line in s {
                        _ = writeln!(&mut buf, "{add}{line}");
                    }
                }
                prettydiff::basic::DiffOp::Replace(a, b) => {
                    for line in a {
                        _ = writeln!(&mut buf, "{minus}{line}");
                    }
                    for line in b {
                        _ = writeln!(&mut buf, "{add}{line}");
                    }
                    // println!("~{a:?}#{b:?}")
                }
                prettydiff::basic::DiffOp::Remove(s) => {
                    for line in s {
                        _ = writeln!(&mut buf, "{minus}{line}");
                    }
                }
                prettydiff::basic::DiffOp::Equal(s) => {
                    for line in s {
                        _ = writeln!(&mut buf, " {line}");
                    }
                }
            }
        }
        buf
    })
}

fn push_clippy(v: &[ClippyMessage], with: WithData) {
    fn raw_message_clippy(mes: &ClippyMessage) -> Option<String> {
        if let CargoMessage::CompilerMessage(cmes) = &mes.inner {
            if let Some(render) = &cmes.message.rendered {
                return Some(render.clone());
            }
        }
        None
    }

    for mes in v {
        // NOTE: 该路径似乎是相对路径，但为了防止意外的绝对路径，统一去除前缀。
        // 虽然指定了 --no-deps，但如果错误发生在依赖中，那么这个路径为绝对路径，并且可能无法缩短，
        // 因为它们不处于同一个前缀。因此，我们需要根据处理后的路径是绝对还是相对路径来判断该文件位于
        // package 内部还是外部。
        // NOTE: --no-deps 目前有 bug，见 https://github.com/os-checker/bug-MRE-clippy-no-deps
        match &mes.tag {
            ClippyTag::WarnDetailed(paths) => {
                for path in paths {
                    let file = strip_prefix(path, with.root);
                    if let Some(raw) = raw_message_clippy(mes) {
                        with.data.push(Data {
                            cmd_idx: with.cmd_idx,
                            file: file.to_owned(),
                            kind: Kind::ClippyWarn,
                            raw,
                        });
                    };
                }
            }
            ClippyTag::ErrorDetailed(paths) => {
                for path in paths {
                    let file = strip_prefix(path, with.root);
                    if let Some(raw) = raw_message_clippy(mes) {
                        with.data.push(Data {
                            cmd_idx: with.cmd_idx,
                            file: file.to_owned(),
                            kind: Kind::ClippyError,
                            raw,
                        });
                    };
                }
            }
            _ => (),
        }
    }
}
