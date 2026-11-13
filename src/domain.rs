pub use crate::evidence::{InsightId, ObservationId, QuestionId};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

pub const SCHEMA_VERSION: u32 = 8;
pub const V15_SHA256: &str = "db1642b2b6fa5c2cccb11ffd13878068bb28601678d3644049f99dc40e7edb8a";
pub const V15_LENGTH: u64 = 699_956;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordId(pub u64);

/// A coordinate in a file, never implicitly interchangeable with a memory address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOffset(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedAddress(pub u64);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub offset: FileOffset,
    pub bytes: Vec<u8>,
    pub description: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Basis {
    Observed,
    Derived,
    Inferred,
}

impl fmt::Display for Basis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Observed => "Observed",
            Self::Derived => "Derived",
            Self::Inferred => "Inferred",
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordData {
    Question,
    Probe {
        operation: String,
    },
    Observation(Observation),
    Firmware(crate::fwsc::FirmwareView),
    Disassembly(crate::pi32::RunId),
    Rea(String),
    Explanation {
        basis: Basis,
        evidence: Vec<ObservationId>,
    },
    TypeHypothesis(crate::evidence::TypeHypothesis),
    Review {
        kind: crate::history::ReviewKind,
        target: RecordId,
        evidence: Vec<ObservationId>,
    },
    Unknown {
        next_probe: String,
    },
}

impl RecordData {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Question => "Question",
            Self::Probe { .. } => "Probe",
            Self::Observation(_) => "Observation",
            Self::Firmware(_) => "Decoded data",
            Self::Disassembly(_) => "Instructions",
            Self::Rea(_) => "REA evidence",
            Self::Explanation { .. } => "Explanation",
            Self::TypeHypothesis(_) => "Type hypothesis",
            Self::Review { .. } => "Analyst review",
            Self::Unknown { .. } => "Unknown",
        }
    }

    pub fn rust_type(&self) -> &'static str {
        match self {
            Self::Question => "RecordData::Question",
            Self::Probe { .. } => "RecordData::Probe { operation: String }",
            Self::Observation(_) => "Observation { offset: FileOffset, bytes: Vec<u8>, .. }",
            Self::Rea(_) => {
                "ReaRun { request: Request, receipt: Receipt }; original REA ev_ IDs remain distinct from DIVULGE RecordId"
            }
            Self::Disassembly(_) => {
                "DisassemblyReport { id: RunId, scope: ImageSpan, instructions: Vec<Instruction>, coverage: Coverage, .. }"
            }
            Self::Firmware(_) => {
                "FirmwareView -> PackageHeader / PackageEntry / Application; FileOffset != PackageOffset != FlashOffset != MappedAddress"
            }
            Self::Explanation { .. } => {
                "RecordData::Explanation { basis: Basis, evidence: Vec<ObservationId> }"
            }
            Self::TypeHypothesis(_) => {
                "TypeHypothesis { proposed_type, evidence: Vec<ObservationId>, counter_evidence, limitation } / inference"
            }
            Self::Review { .. } => {
                "Review { kind, target: RecordId, evidence: Vec<ObservationId> } / preserves the original finding"
            }
            Self::Unknown { .. } => "RecordData::Unknown { next_probe: String }",
        }
    }
}

/// Human-facing rationale and evidence summary, not hidden model reasoning.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HarnessInsight {
    pub question: String,
    pub purpose: String,
    pub action: String,
    pub observation: String,
    pub explanation: String,
    pub uncertainty: String,
    pub next_step: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuestionPlan {
    pub why: String,
    pub steps: Vec<String>,
    pub enables: String,
    pub answered_when: String,
    pub if_not: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: RecordId,
    pub title: String,
    pub summary: String,
    pub data: RecordData,
    pub insight: HarnessInsight,
    pub note: String,
    /// Explicit question ownership; an evidence edge alone does not imply ownership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_id: Option<QuestionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_plan: Option<QuestionPlan>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Investigates,
    Produces,
    Supports,
    LeavesOpen,
    Suggests,
    Contradicts,
    Revises,
    Resolves,
    Steers,
    Reuses,
}

impl Relation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Investigates => "investigates",
            Self::Produces => "produces",
            Self::Supports => "supports",
            Self::LeavesOpen => "leaves open",
            Self::Suggests => "suggests",
            Self::Contradicts => "contradicts",
            Self::Revises => "revises",
            Self::Resolves => "resolves",
            Self::Steers => "steers",
            Self::Reuses => "reuses",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from: RecordId,
    pub to: RecordId,
    pub relation: Relation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    WorkshopFixture,
    LocalArtifact,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Artifact {
    pub name: String,
    pub length: u64,
    pub sha256: String,
    pub preview: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware: Option<crate::fwsc::FirmwareReport>,
}

impl Artifact {
    /// Identity comparison only; a saved record must be rehashed for fresh verification.
    pub fn matches_v15_reference(&self) -> bool {
        self.length == V15_LENGTH && self.sha256 == V15_SHA256
    }
}

/// Persisted records are untrusted DTOs until validate() checks their structure.
/// Validation does not establish the truth of an explanation or rehash a file.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Investigation {
    pub schema_version: u32,
    pub title: String,
    pub objective: String,
    pub origin: Origin,
    pub artifact: Option<Artifact>,
    /// Append order is the replay order, independent of graph layout.
    pub records: Vec<Record>,
    pub edges: Vec<Edge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disassemblies: Vec<crate::pi32::DisassemblyReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_runs: Vec<crate::omp::AiRun>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rea_runs: Vec<crate::rea::ReaRun>,
}

impl Investigation {
    pub fn validate(&self) -> Result<(), String> {
        if self.rea_runs.len() > 16 {
            return Err("Too many REA runs".into());
        }
        let mut rea_ids = HashSet::new();
        for run in &self.rea_runs {
            run.validate(self.artifact.as_ref().ok_or("REA run has no artifact")?)?;
            if !rea_ids.insert(&run.receipt.id)
                || !self.records.iter().any(|r| {
                    r.id == run.record
                        && matches!(&r.data,RecordData::Rea(id) if id==&run.receipt.id)
                })
            {
                return Err("REA run/record identity mismatch".into());
            }
        }
        for record in &self.records {
            if let RecordData::Rea(id) = &record.data
                && !self
                    .rea_runs
                    .iter()
                    .any(|r| &r.receipt.id == id && r.record == record.id)
            {
                return Err("Missing REA run".into());
            }
        }
        if let Some(artifact) = &self.artifact
            && let Some(firmware) = &artifact.firmware
        {
            firmware.validate(artifact.length)?;
        }
        if self.ai_runs.len() > 16 {
            return Err("Too many AI runs".into());
        }
        let mut ai_ids = HashSet::new();
        for run in &self.ai_runs {
            run.validate()?;
            if !ai_ids.insert(&run.id) || !self.records.iter().any(|r| r.id == run.record) {
                return Err("Invalid AI run identity or record".into());
            }
            if let Some(proposal) = &run.proposal {
                if proposal.evidence.iter().any(|id| {
                    !self
                        .records
                        .iter()
                        .any(|r| r.id == id.record() && self.is_evidence(r))
                }) {
                    return Err("AI proposal has missing evidence".into());
                }
                let app = self
                    .artifact
                    .as_ref()
                    .and_then(|a| a.firmware.as_ref())
                    .and_then(|f| f.application.as_ref());
                let identity = if let Some(app) = app {
                    (app.sha256.clone(), u64::from(app.entry().length))
                } else if self.origin == Origin::WorkshopFixture && self.artifact.is_none() {
                    (crate::fwsc::digest(&[0x34, 0x12]), 2)
                } else {
                    return Err("AI proposal has no supported target".into());
                };
                if run.context["image_sha256"].as_str() != Some(&identity.0)
                    || run.context["image_length"].as_u64() != Some(identity.1)
                {
                    return Err("AI proposal belongs to another target".into());
                }
                if let Some(analysis) = &proposal.analysis {
                    let record = self
                        .record(
                            run.analysis_record
                                .ok_or("AI explanation has no published record")?,
                        )
                        .ok_or("Missing AI explanation")?;
                    if record.insight.explanation != analysis.explanation
                        || record.insight.uncertainty != analysis.limitations
                        || record.insight.purpose != analysis.significance
                        || !matches!(&record.data,RecordData::Explanation{basis:Basis::Inferred,evidence} if evidence==&analysis.evidence)
                    {
                        return Err("AI explanation differs from its retained model receipt".into());
                    }
                } else if run.analysis_record.is_some() {
                    return Err("AI explanation record lacks a model analysis".into());
                }
            }
        }
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "Unsupported investigation version: {}",
                self.schema_version
            ));
        }
        if self.records.is_empty() || self.records.len() > 200 || self.edges.len() > 1_000 {
            return Err("This foundation supports 1-200 records and at most 1,000 edges".into());
        }
        if self.disassemblies.len() > 16 {
            return Err("An investigation supports at most 16 bounded disassembly runs".into());
        }
        let mut runs = HashSet::new();
        for run in &self.disassemblies {
            if !runs.insert(run.id) {
                return Err("Disassembly run IDs must be unique".into());
            }
            run.validate()?;
            let artifact = self
                .artifact
                .as_ref()
                .ok_or("Disassembly has no artifact")?;
            let firmware = artifact
                .firmware
                .as_ref()
                .ok_or("Disassembly has no decoded firmware")?;
            firmware.validate(artifact.length)?;
            let app = firmware
                .application
                .as_ref()
                .ok_or("Disassembly has no application")?;
            if run.map.artifact_id.as_str() != artifact.sha256
                || run.map.image_id.as_str() != app.sha256
                || run.map.image_length != u64::from(app.entry().length)
                || run.map.mapped_base != app.mapped_base()
                || run.map.package_base.0
                    != firmware
                        .flash()
                        .ok_or("Disassembly has no flash parent")?
                        .offset
                        .0
                        + app.flash_offset().0
                || run.map.package_length != u64::from(firmware.header.length)
            {
                return Err("Disassembly map belongs to another artifact/image".into());
            }
        }
        let mut ids = HashSet::new();
        for record in &self.records {
            if record.id.0 == 0 || !ids.insert(record.id) {
                return Err("Record IDs must be nonzero and unique".into());
            }
            if let Some(plan) = &record.question_plan
                && (!matches!(
                    record.data,
                    RecordData::Question | RecordData::Unknown { .. }
                ) || record.question_id.is_some()
                    || [
                        &plan.why,
                        &plan.enables,
                        &plan.answered_when,
                        &plan.if_not,
                        &record.insight.question,
                    ]
                    .iter()
                    .any(|text| text.trim().is_empty())
                    || plan.steps.is_empty()
                    || plan.steps.len() > 8
                    || plan.steps.iter().any(|step| step.trim().is_empty()))
            {
                return Err("A question plan requires a question, rationale, bounded method, outcome, and answer criteria".into());
            }
            if let Some(question_id) = record.question_id
                && (question_id.record() == record.id
                    || !self.records.iter().any(|question| {
                        question.id == question_id.record()
                            && question.question_plan.is_some()
                            && matches!(
                                question.data,
                                RecordData::Question | RecordData::Unknown { .. }
                            )
                    }))
            {
                return Err("Record references a missing question plan".into());
            }
            if let RecordData::Observation(obs) = &record.data {
                if obs.offset.0.checked_add(obs.bytes.len() as u64).is_none() {
                    return Err("Observation byte range overflows".into());
                }
                if let Some(artifact) = &self.artifact
                    && obs.offset.0 + obs.bytes.len() as u64 > artifact.length
                {
                    return Err("Observation extends beyond the artifact".into());
                }
            }
            if let RecordData::Firmware(view) = record.data
                && !self
                    .artifact
                    .as_ref()
                    .and_then(|a| a.firmware.as_ref())
                    .is_some_and(|report| report.contains(view))
            {
                return Err("Decoded record references missing firmware data".into());
            }
            if let RecordData::Disassembly(id) = record.data
                && !runs.contains(&id)
            {
                return Err("Instruction record references a missing run".into());
            }
            if let RecordData::TypeHypothesis(hypothesis) = &record.data {
                hypothesis.validate(self)?;
            }
            if let RecordData::Review {
                kind,
                target,
                evidence,
            } = &record.data
            {
                if !self
                    .records
                    .iter()
                    .take_while(|r| r.id != record.id)
                    .any(|r| r.id == *target)
                    || record.insight.explanation.trim().is_empty()
                    || record.insight.uncertainty.trim().is_empty()
                    || evidence.len() > 16
                    || (*kind == crate::history::ReviewKind::Resolution && evidence.is_empty())
                {
                    return Err("Review requires a prior target, rationale, limits and supporting resolution evidence".into());
                }
                for id in evidence {
                    id.resolve(self)?;
                }
            }
            if let RecordData::Explanation { evidence, .. } = &record.data {
                if evidence.is_empty() {
                    return Err("An explanation must cite an observation".into());
                }
                for id in evidence {
                    if !self
                        .records
                        .iter()
                        .any(|candidate| candidate.id == id.record() && self.is_evidence(candidate))
                    {
                        return Err("Explanation references a missing observation".into());
                    }
                }
            }
        }
        for edge in &self.edges {
            if edge.from == edge.to || !ids.contains(&edge.from) || !ids.contains(&edge.to) {
                return Err("Graph edge has a missing or identical endpoint".into());
            }
        }
        if let Some(artifact) = &self.artifact
            && (artifact.sha256.len() != 64
                || !artifact
                    .sha256
                    .bytes()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
                || artifact.preview.len() > 256
                || artifact.preview.len() as u64 > artifact.length)
        {
            return Err("Invalid artifact identity or preview".into());
        }
        if let Some(artifact) = &self.artifact
            && let Some(firmware) = &artifact.firmware
        {
            if !artifact.matches_v15_reference() {
                return Err("A saved V15 report requires the V15 identity".into());
            }
            firmware.validate(artifact.length)?;
        }
        Ok(())
    }

    pub fn record(&self, id: RecordId) -> Option<&Record> {
        self.records.iter().find(|record| record.id == id)
    }

    /// The presence of a failed tool request does not make it evidence.
    pub fn is_evidence(&self, record: &Record) -> bool {
        matches!(
            record.data,
            RecordData::Observation(_) | RecordData::Firmware(_) | RecordData::Disassembly(_)
        ) || matches!(&record.data,RecordData::Rea(id) if self.rea_runs.iter().any(|run|&run.receipt.id==id&&run.receipt.kind=="completed"))
    }

    pub fn question_for(&self, id: RecordId) -> Option<&Record> {
        let record = self.record(id)?;
        if record.question_plan.is_some() {
            Some(record)
        } else {
            self.record(record.question_id?.record())
                .filter(|question| question.question_plan.is_some())
        }
    }

    pub fn workshop() -> Self {
        let insight = HarnessInsight {
            question: "What can we establish from these two bytes, and what are we only assuming about their meaning?".into(),
            purpose: "Separate a byte observation from our choice of interpretation.".into(),
            action: "Read the teaching fixture at file offset 0; interpret its two bytes as a little-endian u16.".into(),
            observation: "The fixture contains exactly two bytes: 34 12 (hexadecimal).".into(),
            explanation: "u16::from_le_bytes([0x34, 0x12]) yields 4660 (0x1234). This is a reproducible interpretation of the bytes.".into(),
            uncertainty: "We have not established that the original author intended an integer, or what the value represents. These are synthetic teaching bytes, not FM-1 evidence.".into(),
            next_step: "Find a format declaration or a controlled example that assigns meaning to this field.".into(),
        };
        let descriptions = [
            (
                "What do these bytes mean?",
                "Separate byte evidence from an assumed meaning.",
                RecordData::Question,
            ),
            (
                "Read the teaching fixture",
                "Inspect bytes before assigning meaning.",
                RecordData::Probe {
                    operation: "read bytes [0..2]".into(),
                },
            ),
            (
                "Two bytes: 34 12",
                "Observed at file offset 0x0000.",
                RecordData::Observation(Observation {
                    offset: FileOffset(0),
                    bytes: vec![0x34, 0x12],
                    description: "Synthetic workshop fixture".into(),
                }),
            ),
            (
                "One interpretation: 4660",
                "Little-endian u16; meaning still unknown.",
                RecordData::Explanation {
                    basis: Basis::Derived,
                    evidence: vec![ObservationId::new(3).unwrap()],
                },
            ),
            (
                "What does 4660 represent?",
                "A number alone does not explain its role.",
                RecordData::Unknown {
                    next_probe: "Compare with a known format or controlled input".into(),
                },
            ),
            (
                "What would establish meaning?",
                "Choose the next evidence to look for.",
                RecordData::Question,
            ),
        ];
        let records = descriptions
            .into_iter()
            .enumerate()
            .map(|(i, (title, summary, data))| {
                let mut step = insight.clone();
                match i {
                    0 => {
                        step.action = "Choose a bounded read of the teaching fixture.".into();
                        step.observation = "No observation has been recorded at this step.".into();
                        step.explanation = "Begin by inspecting the bytes before assigning a type or meaning.".into();
                        step.next_step = "Read the two-byte fixture at file offset zero.".into();
                    }
                    1 => {
                        step.action = "Read exactly two bytes starting at file offset zero.".into();
                        step.observation = "The read result is recorded in the following observation.".into();
                        step.explanation = "A bounded read makes the scope of our evidence explicit.".into();
                        step.next_step = "Inspect the resulting byte values.".into();
                    }
                    2 => {
                        step.action = "Read exactly two bytes starting at file offset zero.".into();
                        step.explanation = "These are byte values. An original data type or field meaning has not been established.".into();
                        step.next_step = "Try an explicitly chosen little-endian u16 interpretation.".into();
                    }
                    4 => {
                        step.question = "What does the interpreted number represent?".into();
                        step.action = "Review what the existing observation can and cannot support.".into();
                        step.explanation = "The number could have several roles. The current evidence does not select one.".into();
                    }
                    5 => {
                        step.question = "What evidence would establish the field's meaning?".into();
                        step.action = "Propose a discriminating follow-up probe.".into();
                        step.explanation = "A format declaration or a controlled change could distinguish competing interpretations.".into();
                    }
                    _ => {}
                }
                Record {
                    id: RecordId(i as u64 + 1), title: title.into(), summary: summary.into(),
                    data, insight: step, note: String::new(),
                    question_id: match i { 1..=3 => Some(QuestionId::new(1).unwrap()), _ => None },
                    question_plan: match i {
                        0 => Some(QuestionPlan {
                            why: "The same bytes can encode a number, text, or part of an instruction. Choosing a type too early makes an assumption look like evidence.".into(),
                            steps: vec![
                                "Read exactly two bytes at file offset 0 and retain the raw values.".into(),
                                "Interpret them as u16 in each byte order; compare the results.".into(),
                                "State which facts the read establishes and which meaning still needs evidence.".into(),
                            ],
                            enables: "Build an observation we can reproduce, then choose a useful question about what the field does.".into(),
                            answered_when: "We can reproduce the byte values and each numeric interpretation without claiming that either is the original source type.".into(),
                            if_not: "If fewer than two bytes are available, report an incomplete read. If both interpretations are possible, preserve both.".into(),
                        }),
                        4 => Some(QuestionPlan {
                            why: "A numeric conversion tells us a value, not whether it is a count, an address, a setting, or unrelated data.".into(),
                            steps: vec![
                                "List plausible roles without choosing one as fact.".into(),
                                "Look for a format declaration or a controlled example that distinguishes those roles.".into(),
                            ],
                            enables: "Name the field for a demonstrated purpose instead of attaching a convincing but unsupported label.".into(),
                            answered_when: "Independent evidence associates the exact field with one role and rules out relevant alternatives.".into(),
                            if_not: "Keep the role unknown and record the evidence needed to distinguish the remaining explanations.".into(),
                        }),
                        5 => Some(QuestionPlan {
                            why: "Repeating the same conversion cannot resolve the field's role. We need a probe whose outcomes separate competing explanations.".into(),
                            steps: vec![
                                "Choose two competing roles and write down what each predicts.".into(),
                                "Select a known example or controlled change that would produce different evidence for those roles.".into(),
                            ],
                            enables: "Turn an open-ended guess into a testable next step that someone else can repeat.".into(),
                            answered_when: "The proposed probe names its inputs, expected distinguishing outcomes, and how each outcome changes our conclusion.".into(),
                            if_not: "If the outcomes would look the same, redesign the probe before spending effort on it.".into(),
                        }),
                        _ => None,
                    },
                }
            })
            .collect();
        Self {
            schema_version: SCHEMA_VERSION,
            title: "From bytes to evidence".into(),
            objective: "Learn the Rust types behind an investigation, then follow their evidence in the graph.".into(),
            origin: Origin::WorkshopFixture,
            artifact: None,
            disassemblies: Vec::new(),
            ai_runs: Vec::new(),
            rea_runs: Vec::new(),
            records,
            edges: vec![
                Edge { from: RecordId(1), to: RecordId(2), relation: Relation::Investigates },
                Edge { from: RecordId(2), to: RecordId(3), relation: Relation::Produces },
                Edge { from: RecordId(3), to: RecordId(4), relation: Relation::Supports },
                Edge { from: RecordId(4), to: RecordId(5), relation: Relation::LeavesOpen },
                Edge { from: RecordId(5), to: RecordId(6), relation: Relation::Suggests },
            ],
        }
    }

    pub fn from_artifact(artifact: Artifact) -> Self {
        let matched = artifact.matches_v15_reference();
        let insight = HarnessInsight {
            question: "Does this file match the exact FM-1 V15 image our decoder profile expects?".into(),
            purpose: "Bind future observations to the same file content.".into(),
            action: "Read the selected file in a worker; calculate SHA-256 and retain its first 256 bytes.".into(),
            observation: format!("{} bytes. SHA-256: {}", artifact.length, artifact.sha256),
            explanation: if matched { "The length and SHA-256 match the configured official FM-1 V15 reference.".into() } else { "The bytes have an identity. They do not match the configured FM-1 V15 reference.".into() },
            uncertainty: "Identity alone does not establish program behavior. Decoding results are recorded separately; disassembly and the AI harness are not connected.".into(),
            next_step: if matched { "Inspect the FWSC package records and trace the application back to this artifact.".into() } else { "Keep this artifact available for generic inspection; V15-specific parsing requires an exact identity match.".into() },
        };
        let mut records = vec![
            Record {
                id: RecordId(1),
                title: "Is this the expected V15?".into(),
                summary: "Choose a decoder using identity evidence.".into(),
                data: RecordData::Question,
                insight: insight.clone(),
                note: String::new(),
                question_id: None,
                question_plan: Some(QuestionPlan {
                    why: "A filename can be renamed, and different firmware versions can use different layouts. The wrong profile could turn package bytes into believable but incorrect addresses.".into(),
                    steps: vec![
                        "Read the selected file; record its length and calculate SHA-256 over the complete byte stream.".into(),
                        "Compare both values with the configured official V15 reference.".into(),
                        "Record a match or mismatch before selecting a version-specific decoder.".into(),
                    ],
                    enables: "A match lets us select the V15 package profile and attach later observations to this exact artifact.".into(),
                    answered_when: "The actual byte length and SHA-256 have both been compared. A mismatch answers the identity question too; it does not qualify the file as V15.".into(),
                    if_not: "For a mismatch, continue generic inspection and identify the correct format. Do not apply V15 offsets or assume the file is corrupt.".into(),
                }),
            },
            Record {
                id: RecordId(2),
                title: "Read and hash the file".into(),
                summary: "SHA-256, length, and a bounded preview.".into(),
                data: RecordData::Probe {
                    operation: "sha256 + preview".into(),
                },
                insight: insight.clone(),
                note: String::new(),
                question_id: Some(QuestionId::new(1).unwrap()),
                question_plan: None,
            },
            Record {
                id: RecordId(3),
                title: format!("{} bytes observed", artifact.length),
                summary: artifact.name.clone(),
                data: RecordData::Observation(Observation {
                    offset: FileOffset(0),
                    bytes: artifact.preview.clone(),
                    description: "First up to 256 bytes of the selected artifact".into(),
                }),
                insight: insight.clone(),
                note: String::new(),
                question_id: Some(QuestionId::new(1).unwrap()),
                question_plan: None,
            },
            Record {
                id: RecordId(4),
                title: if matched {
                    "FM-1 V15 identity matches".into()
                } else {
                    "Artifact identity recorded".into()
                },
                summary: "An identity check, not a decoded program.".into(),
                data: RecordData::Explanation {
                    basis: Basis::Derived,
                    evidence: vec![ObservationId::new(3).unwrap()],
                },
                insight: insight.clone(),
                note: String::new(),
                question_id: Some(QuestionId::new(1).unwrap()),
                question_plan: None,
            },
            Record {
                id: RecordId(5),
                title: if matched { "Where is the application?".into() } else { "What format is this file?".into() },
                summary: "Find the bytes a decoder should inspect.".into(),
                data: RecordData::Unknown {
                    next_probe: "Decode and validate package records".into(),
                },
                insight,
                note: String::new(),
                question_id: None,
                question_plan: Some(if matched { QuestionPlan {
                    why: "The FWSC package contains structure around the program. Disassembling the container as instructions would confuse packaging bytes with application code.".into(),
                    steps: vec![
                        "Decode the package records and validate their byte ranges and supported checksums.".into(),
                        "Locate the application and preserve the mapping back to the original file bytes.".into(),
                        "Establish a load map before interpreting file offsets as pi32v2 addresses.".into(),
                    ],
                    enables: "Give the pi32v2 decoder the correct application bytes and address context, so instruction and branch claims can point back to evidence.".into(),
                    answered_when: "A checked application span, transformation provenance, and justified address mapping are available. A plausible-looking instruction listing is insufficient.".into(),
                    if_not: "Retain unsupported checks and ambiguous mappings as unknown; do not interpret unchecked bytes as instructions.".into(),
                }} else { QuestionPlan {
                    why: "The file does not match our V15 reference. Assuming its format would make every later offset and type interpretation unreliable.".into(),
                    steps: vec![
                        "Inspect signatures, declared sizes, and a bounded set of bytes.".into(),
                        "Compare those observations with an identified format specification before selecting a parser.".into(),
                    ],
                    enables: "Select a suitable parser and establish the boundaries of data it can interpret.".into(),
                    answered_when: "A format is supported by independent structural checks, with its unsupported regions recorded.".into(),
                    if_not: "Keep generic byte inspection available and preserve the format as unresolved.".into(),
                }}),
            },
        ];
        records[0].insight.action =
            "Plan a full-file identity check before choosing a decoder.".into();
        records[0].insight.observation =
            "The result of this question is recorded in its observation and explanation nodes."
                .into();
        records[0].insight.explanation =
            "Identity is the first decision: the filename alone cannot select a firmware profile."
                .into();
        records[1].insight.observation =
            "The read and hash results are retained in the following observation.".into();
        records[1].insight.explanation =
            "Use the complete byte stream, not only the preview, for the identity comparison."
                .into();
        records[2].insight.explanation = "The byte length, digest, and preview are observations. The reference comparison is recorded separately.".into();
        records[4].insight.question = if matched {
            "Where are the application bytes, and how do their file offsets become pi32v2 addresses?".into()
        } else {
            "Which format describes this file's structure?".into()
        };
        records[4].insight.purpose = records[4].question_plan.as_ref().unwrap().why.clone();
        records[4].insight.action = "Proposed investigation; no package decoder has run.".into();
        records[4].insight.observation =
            "No decoded package or application span has been recorded.".into();
        records[4].insight.explanation =
            "This question remains open. Use the plan above to identify the missing evidence."
                .into();
        let mut session = Self::workshop();
        session.title = artifact.name.clone();
        session.objective =
            "Identify the artifact, then connect each interpretation to evidence.".into();
        session.origin = Origin::LocalArtifact;
        session.artifact = Some(artifact);
        session.records = records;
        session.edges.retain(|edge| edge.to.0 <= 5);
        session.append_firmware_records();
        session
    }
}

/// The execution frontier can advance without stealing the human's reading position.
#[derive(Clone, Debug)]
pub struct Attention {
    pub visible_count: usize,
    pub selected_index: usize,
    pub following: bool,
}

impl Attention {
    pub fn all(count: usize) -> Self {
        Self {
            visible_count: count,
            selected_index: count.saturating_sub(1),
            following: true,
        }
    }
    pub fn select(&mut self, index: usize) {
        if index < self.visible_count {
            self.selected_index = index;
            self.following = false;
        }
    }
    pub fn advance(&mut self, total: usize) {
        self.visible_count = (self.visible_count + 1).min(total);
        if self.following {
            self.selected_index = self.visible_count.saturating_sub(1);
        }
    }
    pub fn follow(&mut self) {
        self.following = true;
        self.selected_index = self.visible_count.saturating_sub(1);
    }
    pub fn unread(&self) -> usize {
        self.visible_count.saturating_sub(self.selected_index + 1)
    }
}

pub fn read_u16_le(bytes: &[u8], offset: FileOffset) -> Result<u16, &'static str> {
    let start = usize::try_from(offset.0).map_err(|_| "Offset does not fit this platform")?;
    let end = start.checked_add(2).ok_or("Byte range overflows")?;
    let pair = bytes.get(start..end).ok_or("Two bytes are required")?;
    Ok(u16::from_le_bytes([pair[0], pair[1]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_read_distinguishes_value_and_missing_bytes() {
        assert_eq!(read_u16_le(&[0x34, 0x12], FileOffset(0)), Ok(4660));
        assert!(read_u16_le(&[0x34], FileOffset(0)).is_err());
        assert!(read_u16_le(&[0x34, 0x12], FileOffset(u64::MAX)).is_err());
    }

    #[test]
    fn advancing_does_not_steal_attention() {
        let mut attention = Attention::all(3);
        attention.select(0);
        attention.advance(6);
        assert_eq!(attention.selected_index, 0);
        assert_eq!(attention.unread(), 3);
        attention.follow();
        attention.advance(6);
        assert_eq!(attention.selected_index, 4);
    }

    #[test]
    fn stored_links_and_versions_are_validated() {
        let mut investigation = Investigation::workshop();
        assert!(investigation.validate().is_ok());
        investigation.edges[0].to = RecordId(999);
        assert!(investigation.validate().is_err());
        investigation = Investigation::workshop();
        investigation.records[0].id = RecordId(3);
        assert!(investigation.validate().is_err());
        investigation = Investigation::workshop();
        investigation.schema_version = 99;
        assert!(investigation.validate().is_err());
    }

    #[test]
    fn explanations_require_actual_observation_references() {
        let mut investigation = Investigation::workshop();
        investigation.records[3].data = RecordData::Explanation {
            basis: Basis::Derived,
            evidence: vec![ObservationId::new(1).unwrap()],
        };
        assert!(investigation.validate().is_err());
    }

    #[test]
    fn replay_does_not_publish_the_final_interpretation_in_early_records() {
        let investigation = Investigation::workshop();
        assert!(
            !investigation.records[0]
                .insight
                .explanation
                .contains("4660")
        );
        assert!(
            !investigation.records[2]
                .insight
                .explanation
                .contains("4660")
        );
        assert!(
            investigation.records[3]
                .insight
                .explanation
                .contains("4660")
        );
    }

    #[test]
    fn question_ownership_survives_roundtrip_and_rejects_invalid_targets() {
        let session = Investigation::workshop();
        let encoded = serde_json::to_vec(&session).unwrap();
        let mut restored: Investigation = serde_json::from_slice(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.question_for(RecordId(4)).unwrap().id, RecordId(1));
        assert_eq!(restored.question_for(RecordId(5)).unwrap().id, RecordId(5));
        assert!(
            !restored.records[0]
                .question_plan
                .as_ref()
                .unwrap()
                .enables
                .is_empty()
        );
        restored.records[3].question_id = Some(QuestionId::new(3).unwrap());
        assert!(restored.validate().is_err());
        restored.records[3].question_id = Some(QuestionId::new(1).unwrap());
        restored.records[0]
            .question_plan
            .as_mut()
            .unwrap()
            .steps
            .clear();
        assert!(restored.validate().is_err());
    }
}
