use super::*;
use crate::Result;

#[test]
fn print() -> Result<()> {
    let json = crate::utils::ui_json();

    all(&json).print();
    by_repo(&json).iter().for_each(|(r, b)| {
        r.print();
        b.print();
    });

    Ok(())
}
