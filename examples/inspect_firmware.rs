//! Read-only local verification. No firmware is bundled or written to disk.
use divulge::{domain::Investigation, storage};

fn main() -> Result<(), String> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Usage: cargo run --example inspect_firmware -- <local file>")?;
    let artifact = storage::inspect_artifact(std::path::Path::new(&path))?;
    let session = Investigation::from_artifact(artifact);
    session.validate()?;
    println!("{} records", session.records.len());
    if let Some(report) = session.artifact.as_ref().and_then(|a| a.firmware.as_ref()) {
        if report.header.chip != "AC791N" || report.entries.len() != 8 {
            return Err("V15 acceptance: unexpected chip or entry count".into());
        }
        let accepted_app = report
            .application
            .as_ref()
            .ok_or("V15 acceptance: application missing")?;
        if accepted_app.entry().length != 581_564
            || accepted_app.flash_offset().0 != 0x4120
            || accepted_app.mapped_base().0 != 0x02000120
            || accepted_app.sha256
                != "306e47065f35d7a7a05ada7f5dd092f6e770952054f33f0a86b75fd10ffe3203"
        {
            return Err("V15 acceptance: application identity or mapping changed".into());
        }
        println!(
            "{} / {} entries / {} logical bytes",
            report.header.chip,
            report.entries.len(),
            report.header.length
        );
        for entry in &report.entries {
            println!(
                "{:16} type {:3} package {:08X} file {:08X} size {:7} CRC {:04X}/{:04X}",
                entry.name,
                entry.kind,
                entry.offset.0,
                divulge::fwsc::file_offset(entry.offset)?.0,
                entry.length,
                entry.checksum.stored,
                entry.checksum.calculated
            );
        }
        if let Some(app) = &report.application {
            println!(
                "app.bin / flash {:08X} / mapped {:08X} / {} bytes / SHA256 {}",
                app.flash_offset().0,
                app.mapped_base().0,
                app.entry().length,
                app.sha256
            );
            for entry in &app.directory {
                println!(
                    "JLFS {:16} offset {:08X} length {:7} flags {:04X}",
                    entry.name, entry.offset, entry.length, entry.attributes
                );
            }
        } else {
            println!("Application: {:?}", report.application_error);
        }
        let restored: Investigation =
            serde_json::from_slice(&serde_json::to_vec(&session).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        restored.validate()?;
    }
    Ok(())
}
