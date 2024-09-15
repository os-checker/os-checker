use super::*;
use crate::print;

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
