//! Offline validation only: this example cannot start an analysis tool or model.
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: validate_session <investigation.divulge.json>")?;
    let session = divulge::storage::load(std::path::Path::new(&path))?;
    println!(
        "Schema {} / {} records / {} disassemblies / {} OMP receipts / {} REA receipts",
        session.schema_version,
        session.records.len(),
        session.disassemblies.len(),
        session.ai_runs.len(),
        session.rea_runs.len()
    );
    let restored: divulge::domain::Investigation =
        serde_json::from_slice(&serde_json::to_vec(&session).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    restored.validate()?;
    println!(
        "Validated and round-tripped offline. No fresh-byte capability, tool or model was created."
    );
    Ok(())
}
