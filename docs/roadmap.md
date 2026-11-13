# DIVULGE roadmap

The first delivery is a native, reviewable foundation. The full AI-driven firmware workflow is built through the linked issues, each with acceptance criteria and dependencies.

## Milestones

1. **Native foundation**: run the desktop application, identify an artifact, understand the evidence graph, save/reopen, and review at a human pace.
2. **FM-1 investigation harness**: checked artifact transformations, pi32v2 observations, REA evidence, restricted OMP integration, bounded execution, durable memory, and reliable replay.
3. **Workshop and talk**: a 20-minute Rust introduction, a companion PDF, a guided FM-1 case study, and a native release that can be demonstrated without network dependence.

## Ticket index

| Ticket | Deliverable | Depends on |
| --- | --- | --- |
| [#1](https://github.com/superposition/DIVULGE/issues/1) | Native desktop foundation and real-window verification | - |
| [#2](https://github.com/superposition/DIVULGE/issues/2) | Checked coordinates and evidence domain invariants | #1 |
| [#3](https://github.com/superposition/DIVULGE/issues/3) | Local artifact identity and exact V15 reference | #1 |
| [#4](https://github.com/superposition/DIVULGE/issues/4) | Traceable FWSC and application decoding | #2, #3 |
| [#5](https://github.com/superposition/DIVULGE/issues/5) | pi32v2 adapter and validated instruction semantics | #2, #4 |
| [#6](https://github.com/superposition/DIVULGE/issues/6) | REA operations and canonical evidence | #2, #3 |
| [#7](https://github.com/superposition/DIVULGE/issues/7) | Restricted OMP bridge and model discovery | #1, #2 |
| [#8](https://github.com/superposition/DIVULGE/issues/8) | Bounded AI executor, pause, stop, and steering | #2, #6, #7 |
| [#9](https://github.com/superposition/DIVULGE/issues/9) | Durable Harness Insight and human-paced review | #8, #11 |
| [#10](https://github.com/superposition/DIVULGE/issues/10) | Graph navigation, scale, and accessibility | #1, #2 |
| [#11](https://github.com/superposition/DIVULGE/issues/11) | Event persistence and deterministic offline replay | #2 |
| [#12](https://github.com/superposition/DIVULGE/issues/12) | 20-minute Rust lesson and livecoding checkpoints | #1, #2 |
| [#13](https://github.com/superposition/DIVULGE/issues/13) | Reproducible, visually verified workshop PDF | #12, stable UI |
| [#14](https://github.com/superposition/DIVULGE/issues/14) | Native release and FM-1 talk rehearsal | Harness milestone, #13 |

The exact-V15 reader (#3/#4), vendor disassembly (#5), REA entropy evidence (#6), and restricted OMP next-step proposals (#7) are implemented. The checked domain model (#2) includes role-specific IDs, identity-qualified mappings and explicit type hypotheses. Event persistence (#11) retains ordered history, note revisions and analyst decisions, with offline reconstruction, backup recovery and scoped exports. The bounded executor (#8) connects checked proposals to actual probes and cited explanations, with scope, budgets, pause/stop and completed-request reuse. Durable branch memory and live reading controls (#9) now preserve cited context, counter-evidence, analyst decisions, unread arrivals and presenter hold. Graph navigation (#10), the six Rust checkpoints (#12), the 124-type catalogue and the visually verified 18-page workbook (#13) are implemented. The measured operator rehearsal and the release package are documented in docs/rehearsal.md; publication is tracked by #14. The synthetic Rust workshop remains available alongside the real V15 case study.

## Delivery rule

A UI label is not an integration. A replay is not a live run. A reference value is not independently recovered evidence. Close tickets only after their acceptance checks pass, and record the remaining limitations in the implementation and validation notes.
