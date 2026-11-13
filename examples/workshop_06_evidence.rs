//! Checkpoint 6: follow the same checked evidence link as the application.
use divulge::domain::{Investigation, RecordData, RecordId};

fn main() -> Result<(), String> {
    let case = Investigation::workshop();
    case.validate()?;
    let record = case.record(RecordId(4)).ok_or("Missing record 4")?;
    if let RecordData::Explanation { basis, evidence } = &record.data {
        println!("#{} / {basis}", record.id.0);
        for id in evidence {
            let observation = id.resolve(&case)?;
            println!("Evidence #{}: {}", observation.id.0, observation.title);
        }
    }
    println!("Explanation: {}", record.insight.explanation);
    println!("Limitation: {}", record.insight.uncertainty);
    println!("Next: {}", record.insight.next_step);
    Ok(())
}
