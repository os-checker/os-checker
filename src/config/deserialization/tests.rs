use super::*;
use schemars::generate::SchemaSettings;
use std::io::Write;

#[test]
fn schema() -> Result<()> {
    let settings = SchemaSettings::draft07().with(|s| {
        s.option_nullable = true;
        s.option_add_null_type = false;
    });
    let generator = settings.into_generator();
    let schema = generator.into_root_schema_for::<IndexMap<String, RepoConfig>>();
    let json = serde_json::to_string_pretty(&schema)?;
    println!("{json}");
    std::fs::File::create("assets/schema.json")?.write_all(json.as_bytes())?;
    Ok(())
}
