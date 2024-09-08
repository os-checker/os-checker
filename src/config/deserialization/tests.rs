use super::*;

#[test]
fn schema() -> Result<()> {
    gen_schema("assets/schema.json".into())?;
    Ok(())
}
