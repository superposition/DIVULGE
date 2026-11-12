//! Real read-only REA acceptance run. The selected file is never executed.
use divulge::{
    domain::{Investigation, RecordId},
    rea::{self, Config, ReaRun, Request},
    storage,
};
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
fn main() -> Result<(), String> {
    let path = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("Usage: rea_evidence <artifact> [file-offset] [length]")?,
    );
    let path = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let artifact = storage::inspect_artifact(&path)?;
    let offset = std::env::args()
        .nth(2)
        .unwrap_or("0".into())
        .parse()
        .map_err(|_| "Invalid offset")?;
    let length = std::env::args()
        .nth(3)
        .map(|s| s.parse())
        .transpose()
        .map_err(|_| "Invalid length")?
        .unwrap_or(4096.min(artifact.length));
    let request = Request::entropy(path, &artifact, offset, length, 256)?;
    let receipt = rea::run(
        &Config::discover(),
        &request,
        Arc::new(AtomicBool::new(false)),
    );
    println!(
        "REA {} / {} / {}",
        receipt.rea_version, receipt.kind, receipt.error
    );
    receipt.validate(&artifact)?;
    if receipt.kind != "completed" {
        return Err("REA did not complete".into());
    }
    let run = ReaRun {
        record: RecordId(0),
        request,
        receipt,
    };
    let mut session = Investigation::from_artifact(artifact);
    session.append_rea(run)?;
    storage::save(
        std::path::Path::new("target/rea-acceptance.divulge.json"),
        &session,
    )?;
    let restored = storage::load(std::path::Path::new("target/rea-acceptance.divulge.json"))?;
    let bundle = restored.rea_runs[0].receipt.bundle()?.unwrap();
    for record in bundle["records"].as_array().unwrap() {
        println!(
            "{} / window {} / entropy {}",
            record["evidence_id"],
            record["normalized_result"]["window"],
            record["normalized_result"]["file_entropy"]
        );
    }
    println!(
        "Canonical bundle and offline save/reopen validated; local receipt retained in target."
    );
    Ok(())
}
