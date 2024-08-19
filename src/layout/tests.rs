use super::LayoutOwner;
use expect_test::{expect, expect_file};

#[test]
fn arceos_layout() {
    crate::logger::init();
    let excluded = &["tmp"];
    assert!(LayoutOwner::new("tmp", excluded).is_err());

    let arceos = LayoutOwner::new("./repos/arceos", excluded).unwrap();
    expect_file!["./snapshots/arceos.txt"].assert_debug_eq(&arceos);
    expect_file!["./snapshots/arceos-packages.txt"].assert_debug_eq(&arceos.packages());
}

#[test]
fn cargo_check_verbose() -> crate::Result<()> {
    use itertools::Itertools;
    let re = regex::Regex::new(r#"^\s+Running `CARGO="#)?;
    _ = duct::cmd!("cargo", "clean")
        .dir("/rust/tmp/os-checker/e1000-driver/examples/")
        .run()?;
    let output = duct::cmd!("cargo", "check", "-vv")
        .dir("/rust/tmp/os-checker/e1000-driver/examples/")
        .stderr_capture()
        .unchecked()
        .run()?;
    let messages = cargo_metadata::Message::parse_stream(output.stderr.as_slice())
        .filter_map(|parsed| {
            parsed.ok().and_then(|line| {
                if let cargo_metadata::Message::TextLine(line) = line {
                    if re.is_match(&line) {
                        return Some(line);
                    }
                }
                None
            })
        })
        .collect_vec();
    expect_file!["./snapshots/cargo-check-verbose.txt"].assert_debug_eq(&messages);

    let re_crate_name = regex::Regex::new(r#"CARGO_CRATE_NAME=(\S+)"#)?;
    let re_manifest_dir = regex::Regex::new(r#"CARGO_MANIFEST_DIR=(\S+)"#)?;
    let re_target_triple = regex::Regex::new(r#"--target\s+(\S+)"#)?;

    let krate = messages
        .iter()
        .filter_map(|mes| {
            let crate_name = re_crate_name
                .captures(mes)
                .and_then(|cap| Some(cap.get(1)?.as_str()));
            let manifest_dir = re_manifest_dir
                .captures(mes)
                .and_then(|cap| Some(cap.get(1)?.as_str()));
            let target_triple = re_target_triple
                .captures(mes)
                .and_then(|cap| Some(cap.get(1)?.as_str()));
            match (crate_name, manifest_dir, target_triple) {
                (Some(a), Some(b), Some(c)) => Some((a, b, c)),
                _ => None,
            }
        })
        .collect_vec();
    expect![[r#"
        [
            (
                "nb",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/nb-1.1.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "nb",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/nb-1.1.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "nb",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/nb-0.1.3",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "nb",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/nb-0.1.3",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "void",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/void-1.0.2",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "void",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/void-1.0.2",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "embedded_hal",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/embedded-hal-0.2.7",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "embedded_hal",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/embedded-hal-0.2.7",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "bit_field",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/bit_field-0.10.2",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "spin",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/spin-0.9.8",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "bare_metal",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/bare-metal-1.0.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "volatile",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/volatile-0.3.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "spin",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/spin-0.9.8",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "spin",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/spin-0.7.1",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "bit_field",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/bit_field-0.10.2",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "volatile",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/volatile-0.3.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "bare_metal",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/bare-metal-1.0.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "spin",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/spin-0.7.1",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "buddy_system_allocator",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/buddy_system_allocator-0.6.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "riscv",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/riscv-0.8.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "e1000_driver",
                "/rust/tmp/os-checker/e1000-driver",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "pci",
                "/root/.cargo/git/checkouts/pci-rs-93f3da506027dfc4/583a15b",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "buddy_system_allocator",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/buddy_system_allocator-0.6.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "lazy_static",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/lazy_static-1.5.0",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "e1000_driver",
                "/rust/tmp/os-checker/e1000-driver",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "riscv",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/riscv-0.8.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "pci",
                "/root/.cargo/git/checkouts/pci-rs-93f3da506027dfc4/583a15b",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "lazy_static",
                "/root/.cargo/registry/src/rsproxy.cn-0dccff568467c15b/lazy_static-1.5.0",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "device_tree",
                "/root/.cargo/git/checkouts/device_tree-rs-7c65dd35cede3b4e/2f2e55f",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "device_tree",
                "/root/.cargo/git/checkouts/device_tree-rs-7c65dd35cede3b4e/2f2e55f",
                "x86_64-unknown-linux-gnu",
            ),
            (
                "e1000_driver_test",
                "/rust/tmp/os-checker/e1000-driver/examples",
                "riscv64gc-unknown-none-elf",
            ),
            (
                "e1000_driver_test",
                "/rust/tmp/os-checker/e1000-driver/examples",
                "x86_64-unknown-linux-gnu",
            ),
        ]
    "#]]
    .assert_debug_eq(&krate);

    Ok(())
}
