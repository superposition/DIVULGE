use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

pub const SCHEMA_VERSION: u32 = 1;
pub const V15_SHA256: &str = "db1642b2b6fa5c2cccb11ffd13878068bb28601678d3644049f99dc40e7edb8a";
pub const V15_LENGTH: u64 = 699_956;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordId(pub u64);

/// A coordinate in a file, never implicitly interchangeable with a memory address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOffset(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Explanation {
        basis: Basis,
        evidence: Vec<RecordId>,
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
            Self::Explanation { .. } => "Explanation",
            Self::Unknown { .. } => "Unknown",
        }
    }

    pub fn rust_type(&self) -> &'static str {
        match self {
            Self::Question => "RecordData::Question",
            Self::Probe { .. } => "RecordData::Probe { operation: String }",
            Self::Observation(_) => "Observation { offset: FileOffset, bytes: Vec<u8>, .. }",
            Self::Explanation { .. } => {
                "RecordData::Explanation { basis: Basis, evidence: Vec<RecordId> }"
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
pub struct Record {
    pub id: RecordId,
    pub title: String,
    pub summary: String,
    pub data: RecordData,
    pub insight: HarnessInsight,
    pub note: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Investigates,
    Produces,
    Supports,
    LeavesOpen,
    Suggests,
}

impl Relation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Investigates => "investigates",
            Self::Produces => "produces",
            Self::Supports => "supports",
            Self::LeavesOpen => "leaves open",
            Self::Suggests => "suggests",
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
}

impl Investigation {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "Unsupported investigation version: {}",
                self.schema_version
            ));
        }
        if self.records.is_empty() || self.records.len() > 200 || self.edges.len() > 1_000 {
            return Err("This foundation supports 1-200 records and at most 1,000 edges".into());
        }
        let mut ids = HashSet::new();
        for record in &self.records {
            if record.id.0 == 0 || !ids.insert(record.id) {
                return Err("Record IDs must be nonzero and unique".into());
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
            if let RecordData::Explanation { evidence, .. } = &record.data {
                if evidence.is_empty() {
                    return Err("An explanation must cite an observation".into());
                }
                for id in evidence {
                    if !self.records.iter().any(|candidate| {
                        candidate.id == *id && matches!(candidate.data, RecordData::Observation(_))
                    }) {
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
        Ok(())
    }

    pub fn record(&self, id: RecordId) -> Option<&Record> {
        self.records.iter().find(|record| record.id == id)
    }

    pub fn workshop() -> Self {
        let insight = HarnessInsight {
            question: "What do these two bytes tell us?".into(),
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
                "Start with a question we can test.",
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
                    evidence: vec![RecordId(3)],
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
                }
            })
            .collect();
        Self {
            schema_version: SCHEMA_VERSION,
            title: "From bytes to evidence".into(),
            objective: "Learn the Rust types behind an investigation, then follow their evidence in the graph.".into(),
            origin: Origin::WorkshopFixture,
            artifact: None,
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
            question: "Which exact artifact are we investigating?".into(),
            purpose: "Bind future observations to the same file content.".into(),
            action: "Read the selected file in a worker; calculate SHA-256 and retain its first 256 bytes.".into(),
            observation: format!("{} bytes. SHA-256: {}", artifact.length, artifact.sha256),
            explanation: if matched { "The length and SHA-256 match the configured official FM-1 V15 reference.".into() } else { "The bytes have an identity. They do not match the configured FM-1 V15 reference.".into() },
            uncertainty: "Hashing does not decode the package or establish runtime behavior. The firmware parser, REA adapter, pi32v2 decoder, and AI harness are not connected in this foundation build.".into(),
            next_step: if matched { "Implement the checked FWSC package reader and trace each derived range back to this artifact.".into() } else { "Keep this artifact available for generic inspection; V15-specific parsing requires an exact identity match.".into() },
        };
        let records = vec![
            Record {
                id: RecordId(1),
                title: "Which artifact is this?".into(),
                summary: "Establish identity before interpretation.".into(),
                data: RecordData::Question,
                insight: insight.clone(),
                note: String::new(),
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
                    evidence: vec![RecordId(3)],
                },
                insight: insight.clone(),
                note: String::new(),
            },
            Record {
                id: RecordId(5),
                title: "How is this file structured?".into(),
                summary: "Package decoding is the next milestone.".into(),
                data: RecordData::Unknown {
                    next_probe: "Decode and validate package records".into(),
                },
                insight,
                note: String::new(),
            },
        ];
        let mut session = Self::workshop();
        session.title = artifact.name.clone();
        session.objective =
            "Identify the artifact, then connect each interpretation to evidence.".into();
        session.origin = Origin::LocalArtifact;
        session.artifact = Some(artifact);
        session.records = records;
        session.edges.retain(|edge| edge.to.0 <= 5);
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
            evidence: vec![RecordId(1)],
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
}
