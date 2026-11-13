//! Headless acceptance of the same read-only adapter used by the native UI.
use divulge::{
    coordinates::ImageOffset,
    domain::Investigation,
    pi32::{self, RunId},
    storage,
    tool_runner::ToolConfig,
};
use std::{path::Path, sync::atomic::AtomicBool};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).ok_or(
        "Usage: cargo run --example disassemble -- <V15 file> [--save <investigation.json>] [--tool <host-executable>]",
    )?;
    let inspection = storage::inspect_with_image(Path::new(path))?;
    let image = inspection.image.ok_or("Exact V15 application required")?;
    let mut config = ToolConfig::default();
    if let Some(index) = args.iter().position(|arg| arg == "--tool") {
        config.executable = args
            .get(index + 1)
            .ok_or("--tool requires an executable")?
            .clone();
    }
    let report = pi32::disassemble(
        &image,
        RunId(1),
        ImageOffset(0),
        256,
        &config,
        &AtomicBool::new(false),
    )?;
    println!(
        "{:?} / {} rows / {} of {} bytes decoded / {:?} / validated profile {}",
        report.capture.outcome,
        report.instructions.len(),
        report.decoded_bytes(),
        report.scope.length,
        report.coverage,
        report.validated_semantics()
    );
    for instruction in report.instructions.iter().take(12) {
        println!(
            "{:08X} file+{:08X} {:2} B  {}  {:?}",
            instruction.address.0,
            instruction.source[0].offset.0,
            instruction.width(),
            instruction.text,
            instruction.flow
        );
    }
    let flow = pi32::control_flow(&report);
    println!(
        "{} blocks / {} static edges / {} gaps",
        flow.blocks.len(),
        flow.edges.len(),
        report.gaps.len()
    );
    for limitation in &report.capture.limitations {
        println!("Limit: {limitation}");
    }
    if !report.capture.stderr.is_empty() {
        println!(
            "Stderr: {}",
            String::from_utf8_lossy(&report.capture.stderr)
        );
    }
    let mut session = Investigation::from_artifact(inspection.artifact);
    session.append_disassembly(report)?;
    let bytes = serde_json::to_vec(&session).map_err(|e| e.to_string())?;
    let restored: Investigation = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    restored.validate()?;
    println!(
        "Validated serialized investigation: {} records, {} bytes",
        restored.records.len(),
        bytes.len()
    );
    if let Some(index) = args.iter().position(|arg| arg == "--save") {
        storage::save(
            Path::new(args.get(index + 1).ok_or("--save requires a destination")?),
            &session,
        )?;
    }
    Ok(())
}
