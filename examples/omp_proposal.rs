use divulge::{domain::RecordId, omp, storage};
use serde_json::json;
fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let context = if let Some(path) = args.get(1).filter(|p| p.as_str() != "--synthetic") {
        let session = storage::load(std::path::Path::new(path))?;
        omp::context(
            &session,
            session.records.len(),
            session.records.last().unwrap().id,
        )?
    } else {
        json!({"objective":"Learn how a jump leads to the next block","image_length":512,"record_ids":[3],"evidence":[{"id":3,"observation":"Synthetic bytes at app+0 decode as goto +2; next PC is app+2, so target is app+4. No target bytes have been decoded."}],"limitations":"Synthetic fixture, no firmware."})
    };
    let job = omp::start(
        context.clone(),
        args.get(2).cloned().unwrap_or_default(),
        true,
        0,
    );
    let mut events = vec![];
    loop {
        let event = job
            .receiver
            .recv_timeout(std::time::Duration::from_secs(110))
            .map_err(|e| e.to_string())??;
        if event.kind == "ready" {
            println!(
                "{} / SDK {} / CLI {} / tools {}",
                event.data["selected"],
                event.data["sdk_version"],
                event.data["cli_version"],
                event.data["active_tools"]
            );
        } else if event.kind != "draft" {
            println!("{}: {}", event.kind, event.data);
        }
        let terminal = ["completed", "error", "cancelled"].contains(&event.kind.as_str());
        events.push(event);
        if terminal {
            break;
        }
    }
    let proposal = events
        .iter()
        .find(|e| e.kind == "proposal")
        .map(|e| serde_json::from_value::<omp::Proposal>(e.data.clone()))
        .transpose()
        .map_err(|e| e.to_string())?;
    let run = omp::AiRun {
        analysis_record: None,
        id: job.id.clone(),
        record: RecordId(1),
        context_sha256: divulge::fwsc::digest(&serde_json::to_vec(&context).unwrap()),
        context,
        events: events.clone(),
        proposal,
        outcome: events.last().map(|e| e.kind.clone()).unwrap_or_default(),
        diagnostic: String::new(),
    };
    run.validate()?;
    std::fs::create_dir_all("target").map_err(|e| e.to_string())?;
    std::fs::write(
        "target/omp-acceptance.json",
        serde_json::to_vec_pretty(&run).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    if run.proposal.is_none() {
        return Err("No validated proposal".into());
    }
    Ok(())
}
