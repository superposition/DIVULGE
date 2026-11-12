//! A portable review projection is deliberately distinct from original tool receipts.
use crate::domain::{Investigation, RecordData};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Options {
    pub include_bytes: bool,
    pub include_analyst_text: bool,
    pub include_original_receipts: bool,
}

pub fn review(session: &Investigation, options: Options) -> Result<Value, String> {
    session.validate()?;
    if options.include_original_receipts
        && (!options.include_bytes || !options.include_analyst_text)
    {
        return Err(
            "Original receipts include bytes and text; enable both before including them".into(),
        );
    }
    let records:Vec<_>=session.records.iter().map(|r| {
        let evidence=match &r.data {
            RecordData::Explanation{evidence,..}|RecordData::Review{evidence,..}=>evidence.clone(),
            RecordData::TypeHypothesis(h)=>h.evidence.clone(), _=>vec![],
        };
        let range=match &r.data {RecordData::Observation(obs)=>Some(json!({"space":"original_file","offset":obs.offset,"length":obs.bytes.len(),"bytes":options.include_bytes.then_some(&obs.bytes)})),_=>None};
        json!({"record":r.id,"kind":r.data.label(),"question":r.question_id,"evidence":evidence,"range":range,
            "title":options.include_analyst_text.then_some(&r.title),"summary":options.include_analyst_text.then_some(&r.summary),
            "insight":options.include_analyst_text.then_some(&r.insight),"note":options.include_analyst_text.then_some(&r.note),
            "interpretation":options.include_analyst_text.then(||match &r.data {RecordData::TypeHypothesis(h)=>json!(h),RecordData::Review{kind,target,evidence}=>json!({"kind":kind,"target":target,"evidence":evidence}),_=>Value::Null})})
    }).collect();
    let disassembly:Vec<_>=session.disassemblies.iter().map(|r|json!({"run":r.id,"map":r.map,"scope":r.scope,"coverage":r.coverage,
        "instructions":r.instructions.iter().map(|i|json!({"address":i.address,"span":i.span,"width":i.width(),"text":options.include_analyst_text.then_some(&i.text),
            "bytes":options.include_bytes.then_some(&i.bytes),"flow":i.flow})).collect::<Vec<_>>()})).collect();
    let rea:Vec<_>=session.rea_runs.iter().filter_map(|run|run.receipt.bundle().ok().flatten()).flat_map(|bundle|{
        bundle["records"].as_array().cloned().unwrap_or_default().into_iter().map(|e|json!({"original_evidence_id":e["evidence_id"],
            "canonical":false,"provider":e["provider"]["id"],"authority":e["authority"],"confidence":e["confidence"],
            "window":e["normalized_result"]["window"],"file_entropy":e["normalized_result"]["file_entropy"],
            "blocks":e["normalized_result"]["blocks"],"limitations":options.include_analyst_text.then_some(&e["limitations"])})).collect::<Vec<_>>()
    }).collect();
    Ok(
        json!({"format":"divulge-review-export","version":1,"options":options,
        "notice":"Portable review projection; not a replay case or a canonical REA bundle. Evidence IDs name the retained originals, not hashes of this projection. No credential store is read. Analyst text and original receipts require review before sharing.",
        "objective":options.include_analyst_text.then_some(&session.objective),
        "artifact":session.artifact.as_ref().map(|a|json!({"sha256":a.sha256,"length":a.length,"preview":options.include_bytes.then_some(&a.preview)})),
        "records":records,"relationships":session.edges,"disassembly":disassembly,"rea":rea,
        "original_receipts":options.include_original_receipts.then(||json!({"disassembly":session.disassemblies,"omp":session.ai_runs,"rea":session.rea_runs}))}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_export_requires_explicit_bytes_text_and_machine_receipts() {
        let mut session = Investigation::workshop();
        session.records[2].note = "C:/private/example sensitive-note".into();
        let value = review(&session, Options::default()).unwrap();
        let text = value.to_string();
        assert!(!text.contains("sensitive-note"));
        assert!(!text.contains("C:/private"));
        assert!(value["records"][2]["range"]["bytes"].is_null());
        assert!(value["original_receipts"].is_null());
        assert!(
            review(
                &session,
                Options {
                    include_original_receipts: true,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let value = review(
            &session,
            Options {
                include_bytes: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(value["records"][2]["range"]["bytes"], json!([52, 18]));
    }
}
