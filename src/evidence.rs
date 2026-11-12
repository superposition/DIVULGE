//! Role-specific references and interpretations. Valid links do not prove a claim.
use crate::domain::{Investigation, Record, RecordData, RecordId};
use serde::{Deserialize, Serialize};

macro_rules! record_reference {
    ($name:ident, $role:literal) => {
        #[doc = $role]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(try_from = "u64", into = "u64")]
        pub struct $name(RecordId);
        impl $name {
            /// Checks representation. Resolve against the owning investigation to check role.
            pub fn new(value: u64) -> Result<Self, String> {
                Self::try_from(value)
            }
            pub fn record(self) -> RecordId {
                self.0
            }
            pub fn value(self) -> u64 {
                self.0.0
            }
        }
        impl TryFrom<u64> for $name {
            type Error = String;
            fn try_from(value: u64) -> Result<Self, String> {
                if value == 0 {
                    Err(concat!(stringify!($name), " must be nonzero").into())
                } else {
                    Ok(Self(RecordId(value)))
                }
            }
        }
        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                value.value()
            }
        }
    };
}
record_reference!(
    QuestionId,
    "Identity of a question record within one investigation; never a graph position."
);
record_reference!(
    ObservationId,
    "Identity of a completed evidence record within one investigation; distinct from an REA ev_ ID."
);
record_reference!(
    InsightId,
    "Identity of the human-facing insight attached to a record within one investigation."
);

impl QuestionId {
    pub fn resolve(self, session: &Investigation) -> Result<&Record, String> {
        session
            .record(self.record())
            .filter(|r| {
                r.question_plan.is_some()
                    && matches!(r.data, RecordData::Question | RecordData::Unknown { .. })
            })
            .ok_or_else(|| "QuestionId references no question plan".into())
    }
}
impl ObservationId {
    pub fn resolve(self, session: &Investigation) -> Result<&Record, String> {
        session
            .record(self.record())
            .filter(|r| session.is_evidence(r))
            .ok_or_else(|| "ObservationId references no completed evidence".into())
    }
}
impl InsightId {
    pub fn resolve(
        self,
        session: &Investigation,
    ) -> Result<&crate::domain::HarnessInsight, String> {
        session
            .record(self.record())
            .map(|r| &r.insight)
            .ok_or_else(|| "InsightId references no record".into())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ByteOrder {
    #[default]
    Little,
    Big,
}

/// Integer bits preserve NaN payloads, infinities and negative zero through JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct F32Bits(pub u32);
impl F32Bits {
    pub fn value(self) -> f32 {
        f32::from_bits(self.0)
    }
}

/// Chosen scalar views of observed bytes. These are not parsed firmware fields
/// and do not establish an original source type. Missing widths remain None.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScalarViews {
    pub order: ByteOrder,
    pub u8_value: Option<u8>,
    pub u16_value: Option<u16>,
    pub u32_value: Option<u32>,
    pub f32_bits: Option<F32Bits>,
}
impl ScalarViews {
    pub fn from_bytes(bytes: &[u8], index: usize, order: ByteOrder) -> Self {
        fn window<const N: usize>(bytes: &[u8], index: usize) -> Option<[u8; N]> {
            bytes.get(index..index.checked_add(N)?)?.try_into().ok()
        }
        let u16_value = window(bytes, index).map(|b| match order {
            ByteOrder::Little => u16::from_le_bytes(b),
            ByteOrder::Big => u16::from_be_bytes(b),
        });
        let u32_value = window(bytes, index).map(|b| match order {
            ByteOrder::Little => u32::from_le_bytes(b),
            ByteOrder::Big => u32::from_be_bytes(b),
        });
        Self {
            order,
            u8_value: bytes.get(index).copied(),
            u16_value,
            u32_value,
            f32_bits: u32_value.map(F32Bits),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeHypothesis {
    pub proposed_type: String,
    pub rationale: String,
    pub evidence: Vec<ObservationId>,
    pub counter_evidence: Vec<ObservationId>,
    pub limitation: String,
}
impl TypeHypothesis {
    pub fn validate(&self, session: &Investigation) -> Result<(), String> {
        if [&self.proposed_type, &self.rationale, &self.limitation]
            .iter()
            .any(|v| v.trim().is_empty() || v.len() > 4000)
            || self.evidence.is_empty()
            || self.evidence.len() > 16
            || self.counter_evidence.len() > 16
        {
            return Err(
                "A type hypothesis needs a bounded type, rationale, evidence and limitation".into(),
            );
        }
        for id in self.evidence.iter().chain(&self.counter_evidence) {
            id.resolve(session)?;
        }
        Ok(())
    }
}

impl Investigation {
    pub fn append_type_hypothesis(&mut self, hypothesis: TypeHypothesis) -> Result<usize, String> {
        hypothesis.validate(self)?;
        if self.records.len() >= 200 {
            return Err("Investigation record limit reached".into());
        }
        let id = RecordId(
            self.records
                .iter()
                .map(|r| r.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("Record ID overflow")?,
        );
        let owner = self
            .question_for(hypothesis.evidence[0].record())
            .map(|r| QuestionId::new(r.id.0).unwrap());
        let record = Record {
            id,
            title: format!("Type hypothesis: {}", hypothesis.proposed_type),
            summary: "Analyst inference / not a recovered source type".into(),
            insight: crate::domain::HarnessInsight {
                question: "What target representation could explain these bytes?".into(),
                purpose:
                    "Keep a proposed type separate from decoded declarations and scalar views."
                        .into(),
                action: "Analyst recorded a type hypothesis; no analysis tool was run.".into(),
                observation: format!(
                    "{} cited evidence records; {} counter-evidence records",
                    hypothesis.evidence.len(),
                    hypothesis.counter_evidence.len()
                ),
                explanation: hypothesis.rationale.clone(),
                uncertainty: hypothesis.limitation.clone(),
                next_step:
                    "Choose a probe that could distinguish this type from another interpretation."
                        .into(),
            },
            data: RecordData::TypeHypothesis(hypothesis.clone()),
            note: String::new(),
            question_id: owner,
            question_plan: None,
        };
        let previous_edges = self.edges.len();
        for evidence in &hypothesis.evidence {
            self.edges.push(crate::domain::Edge {
                from: evidence.record(),
                to: id,
                relation: crate::domain::Relation::Suggests,
            });
        }
        for evidence in &hypothesis.counter_evidence {
            self.edges.push(crate::domain::Edge {
                from: evidence.record(),
                to: id,
                relation: crate::domain::Relation::Contradicts,
            });
        }
        self.records.push(record);
        if let Err(error) = self.validate() {
            self.records.pop();
            self.edges.truncate(previous_edges);
            return Err(error);
        }
        Ok(self.records.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_resolve_roles_and_reject_zero_and_missing_references() {
        let session = Investigation::workshop();
        assert!(QuestionId::new(1).unwrap().resolve(&session).is_ok());
        assert!(QuestionId::new(3).unwrap().resolve(&session).is_err());
        assert!(ObservationId::new(3).unwrap().resolve(&session).is_ok());
        assert!(ObservationId::new(1).unwrap().resolve(&session).is_err());
        assert!(InsightId::new(1).unwrap().resolve(&session).is_ok());
        assert!(InsightId::new(999).unwrap().resolve(&session).is_err());
        assert!(serde_json::from_str::<ObservationId>("0").is_err());
        assert_eq!(
            serde_json::to_string(&QuestionId::new(1).unwrap()).unwrap(),
            "1"
        );
    }
    #[test]
    fn scalar_views_preserve_every_float_bit_and_missing_widths() {
        for bits in [
            0x80000000, 0x7f800000, 0xff800000, 0x7fc01234, 0x7fa00001, 0x3f800000,
        ] {
            let values = ScalarViews::from_bytes(&u32::to_le_bytes(bits), 0, ByteOrder::Little);
            let decoded: ScalarViews =
                serde_json::from_slice(&serde_json::to_vec(&values).unwrap()).unwrap();
            assert_eq!(decoded.f32_bits.unwrap().value().to_bits(), bits);
        }
        let short = ScalarViews::from_bytes(&[0x34, 0x12], 0, ByteOrder::Little);
        assert_eq!(short.u16_value, Some(4660));
        assert_eq!(short.f32_bits, None);
        assert_eq!(
            ScalarViews::from_bytes(&[1], usize::MAX, ByteOrder::Big).u8_value,
            None
        );
    }
    #[test]
    fn hypotheses_keep_evidence_and_counter_evidence_without_becoming_observations() {
        let mut session = Investigation::workshop();
        let hypothesis = TypeHypothesis {
            proposed_type: "little-endian u16".into(),
            rationale: "Two available bytes permit this interpretation.".into(),
            evidence: vec![ObservationId::new(3).unwrap()],
            counter_evidence: vec![],
            limitation: "The original role and type are unknown.".into(),
        };
        let index = session.append_type_hypothesis(hypothesis.clone()).unwrap();
        assert!(!session.is_evidence(&session.records[index]));
        let restored: Investigation =
            serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
        restored.validate().unwrap();
        let mut invalid = hypothesis;
        invalid
            .counter_evidence
            .push(ObservationId::new(999).unwrap());
        assert!(session.append_type_hypothesis(invalid).is_err());
    }
}
