use super::*;
use crate::{print, Result};

#[test]
fn all() {
    let json = &crate::utils::ui_json();
    print(&all_targets(json));
}

#[test]
fn by_target() {
    let json = &crate::utils::ui_json();
    print(&split_by_target(json));
}

#[test]
#[instrument(level = "trace")]
fn deser() -> Result<()> {
    let path = "new_ui/batch/home/split/All-Targets/batch_1.json";
    let home = std::fs::read(path)?;
    let node: Vec<NodeRepo> = serde_json::from_slice(&home)?;
    dbg!(node);
    Ok(())
}
