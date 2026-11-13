# Bounded investigation cycles

**AI cycle**, also available under **Model > Run bounded investigation**, advances one question through a checked proposal, a probe, an observation, an explanation and a next proposal. Choose an installed OMP model, the question, objective, image span and probe budget. The default is 12; the accepted range is 1–32. Existing per-case storage limits may end a cycle sooner.

Rust owns the operation allowlist and byte scope. The current V15 profile allows bounded pi32v2 disassembly of freshly verified application bytes. The synthetic workshop profile reads the compiled two-byte fixture `34 12`. Both allow a scoped conclusion without another probe. The model has only `read_evidence`; it cannot invoke the disassembler, read the filesystem or choose a shell command. REA entropy remains available as a separate manual evidence operation and its results can enter the model context.

Each model response must match `Proposal`: text fields, an operation enum, unsigned offsets and lengths, and typed observation references. Follow-up responses must also contain an `Analysis` citing the preceding observation, explaining its significance and stating limitations. The app publishes that interpretation as **Inferred**, never as a measurement. Invalid responses remain diagnostic receipts and end the cycle without a probe. Drafts stay provisional.

## Controls and sequencing

- **Pause AI** lets current work finish and retains its result. It launches neither another model request nor a probe until Resume.
- **Resume AI** continues the already checked next phase. A partial disassembly pauses for review before a follow-up request; failure without accepted evidence ends the cycle.
- **Stop AI** sets the active worker's cancellation flag and disables follow-ups. Late probe output can remain as evidence, but cannot restart execution. A cancelled model response cannot become an executable proposal.

One worker owns the cycle at a time. Request IDs and the investigation generation reject stale delivery. File replacement and other live adapters are blocked while a cycle is active; saving remains available. Closing cancels active work before the normal unsaved-case decision. Reopening a case creates no runtime controller and resumes nothing.

A completed identical request can be reused, including after reopening its ledger. Its key includes target digest, operation, offset, length and frozen tool configuration. Reuse adds a graph edge to the original observation and keeps its original tool receipt. It does not claim the installed tool was run or fingerprinted again. Reused requests consume a budget step, preventing an endless repeated proposal loop. There are at most budget + 1 model requests: the final request may explain the last result and conclude.

The objective and tool profile are frozen for each cycle. A changed objective is an explicit steering record. **View > Case history and review** appends corrections, steering or scoped resolutions; recent reviews are supplied to subsequent model requests. Reviews cannot rewrite original measurements or expand the active scope. Stop and start a new cycle to change the target span, model or budget.

## Saved evidence and replay

Ledger envelope version 2 adds typed execution events: Started, Paused, Resumed, Stopped, ProbeAccepted, ProbeRecorded, ExplanationPublished and Finished. Each event checks question ownership, scope, budget, provenance links and preceding state during replay. Version 1 ledgers remain readable; their baseline and event digests remain unchanged. The envelope version identifies accepted payloads; it is not part of the content digest or an authentication mechanism.

The graph retains questions, proposals, actual probe actions, observations and inferred explanations. Existing node positions and the reading camera stay fixed when cycle records arrive. Reading position stays separate from execution progress. The status strip exposes run state and controls even when the cycle window is closed.

## Verification

Offline tests inject clearly synthetic model transport into the real controller and fixture worker. They cover wrong JSON types, scope and arithmetic bounds, budgets, pause, stale generations, cancelled model delivery, late probe delivery, worker failure, exact-result reuse, rejected output rollback and replay of every event prefix. No test model transport is reachable from the native UI.

An explicit live check sends only the compiled teaching fixture to the selected provider:

```sh
cargo run --locked --example execute_cycle -- --synthetic provider/model
```

It requires an actual probe and evidence-linked explanation, saves the full ledger under ignored `target/`, reloads it and compares the final projection. The 2026-11-12 DeepSeek test completed one probe and two model requests. An earlier malformed response used an array for a string field; it was rejected before execution and retained as an invalid-proposal receipt. Live model quality and availability remain external dependencies, so the talk should also retain a checked offline replay.
