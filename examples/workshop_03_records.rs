//! Checkpoint 3: enum variants carry different kinds of information.
use divulge::domain::{Investigation, RecordData};

fn main() {
    let case = Investigation::workshop();
    for record in &case.records {
        let kind = match &record.data {
            RecordData::Question => "question",
            RecordData::Probe { .. } => "probe",
            RecordData::Observation(_) => "observation",
            RecordData::Explanation { .. } => "explanation",
            RecordData::Unknown { .. } => "unknown",
            _ => "another implemented record kind",
        };
        println!("#{} {kind}", record.id.0);
    }
}
