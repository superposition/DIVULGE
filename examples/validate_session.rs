//! Offline validation only: this example cannot start an analysis tool or model.
fn main() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: validate_session <investigation.divulge.json>")?;
    let history = divulge::storage::load_history(std::path::Path::new(&path))?;
    let session = history.session();
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
    let replay = history.replay(history.events().len())?;
    if serde_json::to_value(&replay).map_err(|e| e.to_string())?
        != serde_json::to_value(session).map_err(|e| e.to_string())?
    {
        return Err("Replay projection mismatch".into());
    }
    println!(
        "{} ordered events / {} unfinished historical jobs / replay matches",
        history.events().len(),
        history.unfinished_jobs().len()
    );
    println!(
        "Validated and round-tripped offline. No fresh-byte capability, tool or model was created."
    );
    Ok(())
}
