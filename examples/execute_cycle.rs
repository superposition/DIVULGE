//! Live OMP acceptance run. Only the compiled synthetic fixture can be sent.
use divulge::{
    domain::*,
    engine::{Engine, Settings, Target},
    execution::Mode,
    history::History,
    storage,
    tool_runner::ToolConfig,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) != Some("--synthetic") {
        return Err("Usage: execute_cycle --synthetic provider/model [output.divulge.json]".into());
    }
    let model = args
        .get(2)
        .ok_or("Choose a configured OMP model explicitly")?
        .clone();
    let output = args
        .get(3)
        .map_or("target/cycle-acceptance.divulge.json", String::as_str);
    let mut session = Investigation::workshop();
    let mut history = History::new(&session, "Synthetic cycle acceptance: compiled 34 12 only")?;
    let mut engine = Engine::default();
    engine.start(&mut session, &mut history, Target::Teaching, ToolConfig::default(), Settings {
        question: QuestionId::new(1)?, model, budget:2, offset:0, length:2,
        objective:"Perform a fresh read_fixture of both synthetic bytes at offset 0, length 2, before concluding. Then explain the observed bytes, their possible little-endian u16 interpretation, and why no semantic meaning has been proved. Conclude once this is done.".into(),
    }, 1)?;
    let started = Instant::now();
    let mut prior = String::new();
    loop {
        if started.elapsed() > Duration::from_secs(240) {
            engine.stop(&mut history)?;
        }
        engine.tick(&mut session, &mut history, 1)?;
        let c = engine.controller.as_ref().unwrap();
        let status = format!(
            "{:?} / probes {}/{} / model requests {} / {}",
            c.mode, c.used, c.budget, c.model_requests, c.status
        );
        if status != prior {
            println!("{status}");
            prior = status;
        }
        if !engine.busy() {
            // Terminal events are recorded on the next tick after an explicit Stop.
            engine.tick(&mut session, &mut history, 1)?;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    storage::save_history(Path::new(output), &history)?;
    let restored = storage::load_history(Path::new(output))?;
    assert_eq!(
        serde_json::to_value(restored.session()).unwrap(),
        serde_json::to_value(&session).unwrap()
    );
    let c = engine.controller.as_ref().unwrap();
    println!(
        "Saved and replayed {} records / {} history events / {} explanations",
        session.records.len(),
        history.events().len(),
        session
            .ai_runs
            .iter()
            .filter(|r| r.analysis_record.is_some())
            .count()
    );
    if c.mode != Mode::Completed
        || c.used == 0
        || !session.ai_runs.iter().any(|r| r.analysis_record.is_some())
    {
        return Err("Cycle did not complete a probe and evidence-linked explanation; inspect the saved receipt".into());
    }
    Ok(())
}
