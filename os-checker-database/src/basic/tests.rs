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

impl Basic {
    fn print(&self) {
        info!("{}", serde_json::to_string_pretty(self).unwrap());
    }
}

impl UserRepo<'_> {
    fn print(self) {
        let Self { user, repo } = self;
        info!("{user}/{repo}");
    }
}
