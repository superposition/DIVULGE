use crate::{
    domain::*,
    fwsc::{self, FirmwareView},
};

impl Investigation {
    pub(crate) fn append_firmware_records(&mut self) {
        let Some(report) = self.artifact.as_ref().and_then(|a| a.firmware.clone()) else {
            return;
        };
        let owner = RecordId(5);
        self.records[4].data = RecordData::Question;
        self.records[4].insight.action = "Decode the identified V15 package using the reference profile; retain each transformation and source coordinate.".into();
        self.records[4].insight.observation =
            "Results are attached as decoded data records below this question.".into();
        self.records[4].insight.explanation = "Container offsets, flash offsets and declared addresses belong to different coordinate spaces.".into();
        let probe = self.add_firmware_record("Decode the V15 package".into(), "Remove markers; decode; check CRCs.".into(),
            RecordData::Probe { operation: "FWSC V15 profile -> package directory -> SFC/JLFS application".into() }, owner,
            HarnessInsight {
                question: self.records[4].insight.question.clone(),
                purpose: "Locate actual program bytes before requesting disassembly.".into(),
                action: "Remove 20 interleaved marker bytes. Decode the 64-byte header and each 80-byte record with key 0xFFFF. Check header and encrypted-directory CRCs.".into(),
                observation: "Each decoded record below retains its field values and original-file provenance.".into(),
                explanation: "This is a deterministic local parser. The format and keys come from the MIT reference implementation.".into(),
                uncertainty: "A package entry's raw CRC may differ when its type requires an unsupported transform.".into(),
                next_step: "Inspect the directory, then follow flash.bin into the application area.".into(),
            });
        self.link(owner, probe, Relation::Investigates);
        let header = self.add_firmware_record(format!("{} / {} entries", report.header.chip, report.header.count),
            format!("{} logical bytes / header + directory CRC match", report.header.length), RecordData::Firmware(FirmwareView::Package), owner,
            HarnessInsight {
                question: "What does the package declare, and does its structure check out?".into(),
                purpose: "Establish bounded directory records before following their offsets.".into(),
                action: "Decode fixed-width little-endian fields and verify CRC-16/XMODEM.".into(),
                observation: format!("Chip {}; {} entries; declared logical length {}. Header CRC {:04X}; encrypted directory CRC {:04X}; both match.", report.header.chip, report.header.count, report.header.length, report.header.checksum.stored, report.header.directory_checksum.stored),
                explanation: "The 20 FWSC marker bytes are absent from package coordinates. Early source ranges split around these markers.".into(),
                uncertainty: "The chip field is package metadata, not independent hardware identification.".into(),
                next_step: "Select a directory entry to inspect its type, sizes, checksum comparison and source bytes.".into(),
            });
        self.link(probe, header, Relation::Produces);
        let mut flash_record = None;
        for (index, entry) in report.entries.iter().enumerate() {
            let original = fwsc::file_offset(entry.offset).expect("validated source mapping");
            let record = self.add_firmware_record(entry.name.clone(),
                format!("file+0x{:X} / {} B / type {}", original.0, entry.length, entry.kind),
                RecordData::Firmware(FirmwareView::Entry(index)), owner,
                HarnessInsight {
                    question: format!("Where is {}, and what is actually validated?", entry.name),
                    purpose: "Bind a package member to its declared span and observed content.".into(),
                    action: "Decode the entry record; bounds-check its payload; calculate a raw-payload CRC and SHA-256.".into(),
                    observation: format!("Type {}; package+0x{:X}; file+0x{:X}; {} bytes ({} padded). Stored CRC {:04X}; raw calculated {:04X}.", entry.kind, entry.offset.0, original.0, entry.length, entry.padded_length, entry.checksum.stored, entry.checksum.calculated),
                    explanation: if entry.checksum.matches() { "The CRC matches the raw package payload. Its bytes and length are within the container.".into() } else { "The raw CRC differs. This entry's checksum transform is unresolved; this does not establish corruption.".into() },
                    uncertainty: "Entry type numbers and filenames do not establish the payload's runtime purpose.".into(),
                    next_step: if entry.name == "flash.bin" { "Follow the SFC transform and JLFS records to app.bin.".into() } else { "Identify this entry type's encoding before assigning stronger semantics.".into() },
                });
            self.link(header, record, Relation::Produces);
            if entry.kind == 0 && entry.name == "flash.bin" {
                flash_record = Some(record);
            }
        }
        if let Some(app) = &report.application {
            let package =
                fwsc::PackageOffset(report.flash().unwrap().offset.0 + app.flash_offset().0);
            let source = fwsc::file_offset(package).expect("validated source mapping");
            let application = self.add_firmware_record("app.bin / decoded".into(),
                format!("0x{:08X} / {} B / CRC match", app.mapped_base().0, app.entry().length),
                RecordData::Firmware(FirmwareView::Application), owner,
                HarnessInsight {
                    question: "Which decoded bytes map to the declared application address?".into(),
                    purpose: "Prepare a bounded input for the pi32v2 disassembler.".into(),
                    action: "At flash+0x4000, decode 32-byte blocks with profile key 0x980F XOR (relative block offset >> 2). Validate JLFS headers, application-area CRC and app.bin CRC.".into(),
                    observation: format!("app.bin: {} bytes; file+0x{:X}; package+0x{:X}; flash+0x{:X}; declared mapped base 0x{:08X}. SHA-256 {}.", app.entry().length, source.0, package.0, app.flash_offset().0, app.mapped_base().0, app.sha256),
                    explanation: "The byte transformation preserves length. The checked directory supplies app.bin's relative offset and its declared entry address.".into(),
                    uncertainty: "Key 0x980F and area start 0x4000 come from the reference profile. The mapping is static metadata; no instructions or runtime behavior have been established.".into(),
                    next_step: "Pass the decoded application and declared address to the pi32v2 adapter, retaining source mappings for every instruction.".into(),
                });
            self.link(
                flash_record.unwrap_or(header),
                application,
                Relation::Produces,
            );
            let next = self.add_firmware_record("What does the code do?".into(), "Next: pi32v2 instructions and references.".into(),
                RecordData::Unknown { next_probe: "Disassemble a bounded application span with the pi32v2 adapter".into() }, owner,
                HarnessInsight {
                    question: "Which instructions, functions and data references explain the FM-1 behavior?".into(),
                    purpose: "Move from a validated container to evidence about the program.".into(),
                    action: "Pending: invoke the vendor pi32v2 disassembler with an explicit address map.".into(),
                    observation: "The application bytes and checksums are available; no disassembler has run in DIVULGE.".into(),
                    explanation: "Decoding a package supplies input for code analysis. It does not recover functions by itself.".into(),
                    uncertainty: "Instruction boundaries, branch targets, symbols and runtime call paths remain unknown.".into(),
                    next_step: "Implement the bounded disassembler adapter and validate its instruction-width and branch semantics.".into(),
                });
            self.link(application, next, Relation::LeavesOpen);
        } else {
            let failure = self.add_firmware_record(
                "Application decode unresolved".into(),
                "Package results remain available.".into(),
                RecordData::Unknown {
                    next_probe: "Inspect the failed application check".into(),
                },
                owner,
                HarnessInsight {
                    question: "Why could the application span not be validated?".into(),
                    observation: report
                        .application_error
                        .unwrap_or_else(|| "No application result".into()),
                    uncertainty: "No decoded application or address map is claimed.".into(),
                    ..HarnessInsight::default()
                },
            );
            self.link(header, failure, Relation::LeavesOpen);
        }
    }

    fn add_firmware_record(
        &mut self,
        title: String,
        summary: String,
        data: RecordData,
        owner: RecordId,
        insight: HarnessInsight,
    ) -> RecordId {
        let id = RecordId(self.records.len() as u64 + 1);
        self.records.push(Record {
            id,
            title,
            summary,
            data,
            insight,
            note: String::new(),
            question_id: Some(owner),
            question_plan: None,
        });
        id
    }

    fn link(&mut self, from: RecordId, to: RecordId, relation: Relation) {
        self.edges.push(Edge { from, to, relation });
    }
}
