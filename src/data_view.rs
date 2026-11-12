//! Compact views over actual parser results. Selection is shared with the graph.
use crate::graph::{ACCENT, MUTED};
use divulge::{
    domain::{Investigation, RecordData},
    fwsc::{self, FirmwareReport, FirmwareView, PackageOffset},
};
use eframe::egui::{self, Color32, RichText};

fn record_index(session: &Investigation, view: FirmwareView, visible: usize) -> Option<usize> {
    session
        .records
        .iter()
        .take(visible)
        .position(|r| matches!(r.data, RecordData::Firmware(v) if v == view))
}

pub fn table(
    ui: &mut egui::Ui,
    session: &Investigation,
    visible: usize,
    selected: usize,
) -> Option<usize> {
    let mut selection = None;
    let Some(report) = session.artifact.as_ref().and_then(|a| a.firmware.as_ref()) else {
        ui.label("Open the exact FM-1 V15 file to populate the package data view.");
        return None;
    };
    let Some(header_index) = record_index(session, FirmwareView::Package, visible) else {
        ui.label("Package results have not been reached in this replay.");
        return None;
    };
    egui::ScrollArea::both().id_salt("firmware_data").auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Package contents");
            if ui.selectable_label(selected == header_index, "Inspect header").clicked() { selection = Some(header_index); }
        });
        ui.label(RichText::new(format!("{}  /  {} entries  /  {} logical bytes  /  20 marker bytes", report.header.chip, report.header.count, report.header.length)).monospace().color(ACCENT));
        ui.label(RichText::new("Header CRC: match    Encrypted directory CRC: match").small().color(MUTED));
        ui.add_space(12.0);
        egui::Grid::new("package_entries").striped(true).spacing([12.0, 8.0]).show(ui, |ui| {
            for title in ["MEMBER", "TYPE", "PACKAGE+", "FILE+", "BYTES", "RAW CRC"] { ui.label(RichText::new(title).small().color(MUTED)); }
            ui.end_row();
            for (index, entry) in report.entries.iter().enumerate() {
                let Some(record) = record_index(session, FirmwareView::Entry(index), visible) else { continue; };
                if ui.selectable_label(selected == record, RichText::new(&entry.name).monospace()).clicked() { selection = Some(record); }
                cell(ui, entry.kind.to_string());
                cell(ui, format!("{:08X}", entry.offset.0));
                cell(ui, format!("{:08X}", fwsc::file_offset(entry.offset).unwrap().0));
                cell(ui, entry.length.to_string());
                ui.label(RichText::new(if entry.checksum.matches() { "match" } else { "unresolved" })
                    .monospace().color(if entry.checksum.matches() { ACCENT } else { Color32::from_rgb(224, 177, 113) }))
                    .on_hover_text(format!("Stored {:04X}; raw calculated {:04X}. A difference may require an entry-specific transform.", entry.checksum.stored, entry.checksum.calculated));
                ui.end_row();
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new("Offsets are hexadecimal. Select a row for field locations, sizes, hashes and bytes.").small().color(MUTED));
        if let Some(app_index) = record_index(session, FirmwareView::Application, visible)
            && let Some(app) = &report.application
        {
            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.heading("Application map");
                if ui.selectable_label(selected == app_index, "Inspect app.bin").clicked() { selection = Some(app_index); }
            });
            let package = PackageOffset(report.flash().unwrap().offset.0 + app.flash_offset().0);
            let original = fwsc::file_offset(package).unwrap();
            ui.label(RichText::new(format!("FILE +{:08X}  >  FLASH +{:08X}  >  DECLARED {:08X}", original.0, app.flash_offset().0, app.mapped_base().0)).monospace().color(ACCENT));
            ui.label(RichText::new(format!("{} bytes  /  SFC key 0x980F from reference profile  /  app CRC: match", app.entry().length)).small());
            ui.add_space(12.0);
            egui::Grid::new("jlfs_entries").striped(true).spacing([14.0, 7.0]).show(ui, |ui| {
                for title in ["JLFS RECORD", "DECLARED OFFSET", "BYTES", "ATTR", "HEADER CRC"] { ui.label(RichText::new(title).small().color(MUTED)); }
                ui.end_row();
                for entry in &app.directory {
                    cell(ui, &entry.name);
                    cell(ui, format!("{:08X}", entry.offset));
                    cell(ui, entry.length.to_string());
                    cell(ui, format!("{:04X}", entry.attributes));
                    ui.label(RichText::new("match").monospace().color(ACCENT));
                    ui.end_row();
                }
            });
            ui.add_space(10.0);
            ui.label(RichText::new("JLFS offsets are declarations: app.bin is relative to the application area; partition records use other semantics. Header CRCs do not validate partition contents.").small().color(MUTED));
            ui.add_space(10.0);
            ui.label(RichText::new("Next evidence: pi32v2 instructions, functions and references. Disassembly is not connected yet.").small().color(MUTED));
        }
    });
    selection
}

fn cell(ui: &mut egui::Ui, value: impl Into<String>) {
    ui.add(egui::Label::new(RichText::new(value).monospace().size(12.0)).selectable(true));
}

pub fn fields(ui: &mut egui::Ui, report: &FirmwareReport, view: FirmwareView) {
    match view {
        FirmwareView::Package => {
            ui.label(
                RichText::new("[decoded header / little endian]")
                    .monospace()
                    .color(ACCENT),
            );
            field(
                ui,
                "+00 u16 header_crc",
                &format!("{:04X} / match", report.header.checksum.stored),
            );
            field(
                ui,
                "+02 u16 dir_crc",
                &format!("{:04X} / match", report.header.directory_checksum.stored),
            );
            field(
                ui,
                "+04 u32 length",
                &format!("{} / 0x{:X}", report.header.length, report.header.length),
            );
            field(ui, "+08 u16 entries", &report.header.count.to_string());
            field(ui, "+0A u16 format", &report.header.format.to_string());
            field(
                ui,
                "+0C u32 block_size",
                &report.header.block_size.to_string(),
            );
            field(ui, "+10 [u8;16] chip", &report.header.chip);
            ui.label(RichText::new("Package-relative field locations. Unknown/reserved bytes are preserved in the decoded header below.").small().color(MUTED));
        }
        FirmwareView::Entry(index) => {
            let entry = &report.entries[index];
            let record_start = 64 + index as u64 * 80;
            ui.label(
                RichText::new(format!("[entry record / package+{record_start:08X}]"))
                    .monospace()
                    .color(ACCENT),
            );
            field(ui, "+00 u16 type", &entry.kind.to_string());
            field(ui, "+02 u16 index", &entry.index.to_string());
            field(
                ui,
                "+04 u16 crc",
                &format!(
                    "stored {:04X} / raw {:04X}",
                    entry.checksum.stored, entry.checksum.calculated
                ),
            );
            field(ui, "+06 u16 reserved", &format!("0x{:04X}", entry.reserved));
            field(
                ui,
                "+08 u32 offset",
                &format!("package+0x{:08X}", entry.offset.0),
            );
            field(
                ui,
                "+0C u32 length",
                &format!("{} / 0x{:X}", entry.length, entry.length),
            );
            field(
                ui,
                "+10 u32 padded",
                &format!("{} / 0x{:X}", entry.padded_length, entry.padded_length),
            );
            field(ui, "+40 [u8;16] name", &entry.name);
            field(ui, "payload SHA-256", &entry.sha256);
            ui.collapsing("entry header / source spans and decoded bytes", |ui| {
                if let Ok(spans) = fwsc::source_spans(
                    PackageOffset(record_start),
                    80,
                    u64::from(report.header.length),
                ) {
                    for span in spans {
                        field(
                            ui,
                            "source file",
                            &format!("0x{:08X}+{}", span.offset.0, span.length),
                        );
                    }
                }
                for (row, chunk) in entry.decoded_header.chunks(8).enumerate() {
                    cell(
                        ui,
                        format!(
                            "+{:02X} {}",
                            row * 8,
                            chunk
                                .iter()
                                .map(|b| format!("{b:02X}"))
                                .collect::<Vec<_>>()
                                .join(" ")
                        ),
                    );
                }
            });
        }
        FirmwareView::Application => {
            let app = report.application.as_ref().unwrap();
            let flash = report.flash().unwrap();
            let package = PackageOffset(flash.offset.0 + app.flash_offset().0);
            ui.label(
                RichText::new("[checked application span]")
                    .monospace()
                    .color(ACCENT),
            );
            field(
                ui,
                "file offset",
                &format!("0x{:08X}", fwsc::file_offset(package).unwrap().0),
            );
            field(ui, "package offset", &format!("0x{:08X}", package.0));
            field(
                ui,
                "flash offset",
                &format!("0x{:08X}", app.flash_offset().0),
            );
            field(
                ui,
                "area relative",
                &format!("0x{:08X}", app.entry().offset),
            );
            field(
                ui,
                "declared address",
                &format!("0x{:08X}", app.mapped_base().0),
            );
            field(
                ui,
                "length / u32",
                &format!("{} / 0x{:X}", app.entry().length, app.entry().length),
            );
            field(
                ui,
                "app CRC / u16",
                &format!("{:04X} / match", app.checksum.stored),
            );
            field(
                ui,
                "area CRC / u16",
                &format!("{:04X} / match", app.area_checksum.stored),
            );
            field(ui, "SHA-256", &app.sha256);
            ui.collapsing("transformation and field provenance", |ui| {
                field(ui, "profile", "FM-1 V15 / MIT Package.cs");
                field(ui, "SFC", "32-byte blocks from flash+0x4000; key = 0x980F XOR (relative block offset >> 2)");
                field(ui, "base field", "area header +0x04 / u32 / declared entry address");
                field(ui, "app offset field", "app.bin directory record +0x04 / u32");
                field(ui, "app length field", "app.bin directory record +0x08 / u32");
                field(ui, "app record", &format!("flash+0x{:08X}", app.entry().record_offset.0));
            });
        }
    }
    ui.add_space(10.0);
}

fn field(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(name).monospace().size(11.0).color(MUTED));
        ui.add(
            egui::Label::new(RichText::new(value).monospace().size(12.0))
                .wrap()
                .selectable(true),
        );
    });
}

/// A view carries a coordinate space explicitly; decoded bytes are never
/// disguised as original file contents at the same numeric offset.
pub struct ByteView<'a> {
    pub bytes: &'a [u8],
    pub offset: u64,
    pub space: &'static str,
    pub source: Vec<fwsc::SourceSpan>,
    pub mapped: Option<u64>,
    pub transform: &'static str,
}

pub fn bytes(report: &FirmwareReport, view: FirmwareView) -> ByteView<'_> {
    let (bytes, offset, mapped, transform, space) = match view {
        FirmwareView::Package => (
            report.header.decoded.as_slice(),
            0,
            None,
            "ENC key 0xFFFF / decoded header",
            "package",
        ),
        FirmwareView::Entry(index) => {
            let entry = &report.entries[index];
            (
                entry.preview.as_slice(),
                entry.offset.0,
                None,
                "marker removal / raw entry payload",
                "package",
            )
        }
        FirmwareView::Application => {
            let app = report.application.as_ref().unwrap();
            let package = report.flash().unwrap().offset.0 + app.flash_offset().0;
            (
                app.preview.as_slice(),
                package,
                Some(app.mapped_base().0),
                "SFC decoded / no instructions decoded",
                "package",
            )
        }
    };
    ByteView {
        bytes,
        offset,
        space,
        source: fwsc::source_spans(
            PackageOffset(offset),
            bytes.len() as u64,
            u64::from(report.header.length),
        )
        .unwrap(),
        mapped,
        transform,
    }
}
