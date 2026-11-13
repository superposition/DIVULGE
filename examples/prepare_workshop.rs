//! Reproducible synthetic replay and portable review; no tools or models.
use divulge::{
    domain::{Investigation, ObservationId, RecordId},
    export,
    history::{History, ReviewKind},
    storage,
};
use std::path::Path;

fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if std::fs::read(path).ok().as_deref() != Some(bytes) {
        storage::atomic_write(path, bytes)?;
    }
    Ok(())
}
fn main() -> Result<(), String> {
    let destination = std::env::args()
        .nth(1)
        .unwrap_or("workshop/fixtures".into());
    let directory = Path::new(&destination);
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut case = Investigation::workshop();
    let mut history = History::new(
        &case,
        "Generated synthetic teaching baseline; no target, tool or model",
    )?;
    case.records[3].note =
        "Workshop checkpoint: the u16 view is reproducible; intended meaning is still unknown."
            .into();
    history.capture(&case)?;
    case.append_review(
        ReviewKind::Correction,
        RecordId(4),
        "Keep the chosen little-endian representation separate from intended meaning.".into(),
        "No consuming instruction or runtime behavior has been observed.".into(),
        vec![ObservationId::new(3)?],
    )?;
    history.capture(&case)?;
    let document = serde_json::to_vec_pretty(history.document()).map_err(|e| e.to_string())?;
    let review = serde_json::to_vec_pretty(&export::review(
        &case,
        export::Options {
            include_bytes: true,
            include_analyst_text: true,
            include_original_receipts: false,
        },
    )?)
    .map_err(|e| e.to_string())?;
    write_if_changed(&directory.join("synthetic-workshop.json"), &document)?;
    write_if_changed(&directory.join("synthetic-review.json"), &review)?;
    let restored = storage::load_history(&directory.join("synthetic-workshop.json"))?;
    if serde_json::to_value(restored.session()).unwrap() != serde_json::to_value(&case).unwrap() {
        return Err("Generated replay differs from its source records".into());
    }
    println!(
        "Synthetic replay: {} records / {} ordered events / no tools or models",
        case.records.len(),
        history.events().len()
    );
    Ok(())
}
