use crate::{domain::*, pi32::DisassemblyReport};

impl Investigation {
    pub fn append_disassembly(&mut self, report: DisassemblyReport) -> Result<usize, String> {
        report.validate()?;
        if self.disassemblies.len() >= 16
            || self.records.len() + 3 > 200
            || self.disassemblies.iter().any(|r| r.id == report.id)
        {
            return Err(
                "This investigation has reached its run limit or already contains this run".into(),
            );
        }
        let first = self
            .records
            .iter()
            .map(|r| r.id.0)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("Record ID overflow")?;
        if first > u64::MAX - 2 {
            return Err("Record ID overflow".into());
        }
        let q = RecordId(first);
        let p = RecordId(first + 1);
        let result = RecordId(first + 2);
        let address = report.map.resolve(&report.scope)?.address;
        let insight = HarnessInsight {
            question: format!("What instructions occupy app.bin+0x{:X} through +0x{:X}, and where can their static paths lead?", report.scope.offset.0, report.scope.offset.0 + report.scope.length),
            purpose: "Connect a selected application span to attributable instructions and justified control-flow edges.".into(),
            action: format!("Wrap {} verified image bytes in a derived ELF32 section at 0x{:08X}; invoke the installed vendor tool with pi32v2/r3 flags.", report.scope.length, address.0),
            observation: format!("Run {:?}; {} instruction rows; {} of {} bytes decoded; coverage {:?}. Raw stdout and stderr are retained.", report.capture.outcome, report.instructions.len(), report.decoded_bytes(), report.scope.length, report.coverage),
            explanation: "Each accepted row must match the selected bytes and checked image/source mapping. Numeric branch targets use validated PC-after-instruction displacement semantics only for the pinned tool and supported forms.".into(),
            uncertainty: "Static decoding does not prove execution, function boundaries or purpose. Register destinations, hardware loops, undecodable gaps and unvalidated forms remain unresolved.".into(),
            next_step: "Select an instruction or basic block; inspect its source bytes, outgoing edges and the evidence needed to explain its role.".into(),
        };
        let record = |id, title, summary, data, owner, plan, detail| Record {
            id,
            title,
            summary,
            data,
            insight: detail,
            note: String::new(),
            question_id: owner,
            question_plan: plan,
        };
        let mut question_insight = insight.clone();
        question_insight.observation =
            "The bounded tool result follows this question and probe.".into();
        let mut probe_insight = question_insight.clone();
        probe_insight.explanation = "The wrapper contains only the selected verified bytes and a declared address. No firmware is executed.".into();
        self.records.extend([
            record(q, "What do these instructions do?".into(), format!("app+0x{:X} / {} B", report.scope.offset.0, report.scope.length), RecordData::Question, None,
                Some(QuestionPlan {
                    why: "The decoded application is still just bytes. We need instruction boundaries and source-linked control transfers before naming routines or explaining behavior.".into(),
                    steps: vec!["Select a bounded image span and check its mapping to the original artifact.".into(), "Record the vendor tool, flags, wrapper identity and exact output.".into(), "Match each decoded row to the input bytes, then derive only fixture-supported static edges.".into()],
                    enables: "Inspect basic blocks and choose the next destination or data-flow question with visible evidence.".into(),
                    answered_when: "Every accepted instruction has matching source bytes, a checked address and explicit coverage; unsupported transfers and missing bytes are recorded.".into(),
                    if_not: "Retain partial output, gaps and failure diagnostics. Change the scope or validate the unsupported instruction form before claiming a destination.".into(),
                }), question_insight),
            record(p, "Disassemble selected bytes".into(), format!("pi32v2 / r3 / {} bytes", report.scope.length), RecordData::Probe { operation: "vendor pi32v2 disassembler over derived ELF32".into() }, Some(q), None, probe_insight),
            record(result, format!("{} instruction rows", report.instructions.len()), format!("0x{:08X} / {:?} coverage", address.0, report.coverage), RecordData::Disassembly(report.id), Some(q), None, insight),
        ]);
        self.edges.push(Edge {
            from: q,
            to: p,
            relation: Relation::Investigates,
        });
        self.edges.push(Edge {
            from: p,
            to: result,
            relation: Relation::Produces,
        });
        if let Some(app) = self.records.iter().find(|r| {
            matches!(
                r.data,
                RecordData::Firmware(crate::fwsc::FirmwareView::Application)
            )
        }) {
            self.edges.push(Edge {
                from: app.id,
                to: q,
                relation: Relation::Suggests,
            });
        }
        self.disassemblies.push(report);
        if let Err(error) = self.validate() {
            self.disassemblies.pop();
            self.records.truncate(self.records.len() - 3);
            self.edges
                .retain(|e| e.from != q && e.from != p && e.to != q);
            return Err(error);
        }
        Ok(self.records.len() - 1)
    }
}
