# DIVULGE: design and implementation boundaries

## Purpose

Make an AI-driven investigation legible to a person who cannot read at model speed. The human selects the target, sets the objective and bounds, reviews evidence, corrects assumptions, and steers. The AI proposes and performs bounded probes through the harness. The workbench preserves both the investigation and why each step matters.

The initial learning case is an exact official FM-1 V15 artifact. This remains a reusable investigation workbench rather than an FM-1-only editor.

## Desktop layout

Use egui's menu, panel, scrolling, scene, and widget primitives. A narrow investigation tree sits beside a dominant graph. A resizable right panel holds Harness Insight. Raw activity is available in a collapsed lower panel. Dense controls and readable evidence take precedence over dashboard decoration.

Narrative is the default graph projection. Relationships uses a force-directed layout. Positions are presentation data; moving a node never changes an evidence relationship. Layout settles before reading rather than continuously moving completed nodes. Future large-graph work needs scoping, virtualization, pinning, and coverage indicators.

Each node offers a short label and summary. Harness Insight supplies the question, purpose, action, observation, explanation, evidence, uncertainty, and next useful probe. It is human-facing rationale and a durable evidence record, not a hidden chain-of-thought display.

Questions also carry a persisted `QuestionPlan`: why the question matters, a bounded sequence of investigation steps, the decision its answer enables, sufficient-evidence criteria, and the alternative path if the check fails. Probe, observation, and explanation records explicitly identify their owning question. A support edge alone does not establish that ownership. Selecting a question puts its plan ahead of raw data; evidence records keep the question context expandable. Focus question and graph double-click explicitly zoom the camera; normal selection does not move it. Related-response links respect the replay frontier.

Schema v2 adds these plans and ownership links. Version 1 files migrate without fabricating missing plans. Validate question targets and required plan fields when loading; planning text is not proof that its criterion has been met.

## Attention and execution

Keep the execution frontier distinct from the human attention cursor. Selecting an earlier record or panning detaches follow mode. New results can accumulate without moving the selection or scrolling the text. Review next advances at the reader's pace; Follow latest explicitly catches up.

The foundation implements this with recorded replay. The live executor must preserve the same contract. Presenter mode should eventually hold execution at completed insights; exploration mode can continue within the selected budget. Replay never runs a model or analysis tool.

## Layers

1. Domain: artifact identities, typed coordinates, observations, explanations, unknowns, and relationships. No egui dependency in domain code.
2. Application: job scheduling, cancellation, session identity, effects, persistence, and validated tool messages.
3. Adapters: REA, the vendor pi32v2 tool, and a restricted OMP SDK bridge.
4. Presentation: menus, panels, graph projections, attention cursor, and readable summaries.

The foundation currently has `domain`, `storage`, `app`, and `graph` modules in one crate. Split into workspace crates when the executor and adapters warrant it; avoid packaging complexity before the boundary is useful.

## Type-driven design

`RecordId` is stable across layout and replay. `FileOffset` and `MappedAddress` are distinct coordinates. Rust validation does not establish the truth of a claim: a valid evidence link is necessary but not sufficient for a sound explanation.

The next domain milestone adds checked `Span<Space>` constructors, artifact/image identity, fragmented source maps, load maps, and qualified states such as `MatchedV15` and `MappedImage<Pi32v2>`. Loading a saved DTO must not manufacture a fresh verified artifact. Only actual checks can construct qualified states.

Represent missing information explicitly. `Option` is absence, `Result` is an operation's success or failure, and analysis coverage is a separate complete/partial/unavailable outcome. Ambiguous or unresolved destinations stay unresolved. Parsed declared fields, selected-byte scalar interpretations, and inferred types have distinct provenance. Preserve floating-point bits in serialized scalar views.

Retain REA's original records and `ev_` identities. DIVULGE observations have their own identifiers. Evidence bundle order is not a timeline: the application owns a monotonic event sequence. Unknowns must retain supporting and counter evidence, scope, revision history, and resolution rationale.

## OMP bridge

Use a small Bun process around the installed OMP SDK. Reuse OMP's model catalogue and authentication; DIVULGE must not copy credentials or maintain its own provider catalogue. Discover capabilities and report the SDK/CLI versions instead of assuming they match.

Prior local feasibility checks found that the CLI's `--no-tools` could still inherit ambient MCP tools. The bridge therefore requires an SDK session with explicit allowed tools, restricted tool names, MCP/LSP/IRC disabled, no discovered extensions, rules, skills, or context files, and an isolated session. A restricted, tool-free SDK session was feasible; model generation and the full bridge are still pending.

Rust owns execution. The cycle is question -> proposed probe -> validated request -> actual observation -> evidence-linked explanation -> follow-up. Expose only explicit workbench operations. The initial proposed default is 12 probes per question, sequential, with reuse of identical evidence requests. Pause prevents new probes; Stop cancels current work. Correlate every response with session and run identity so stale results cannot affect a different investigation.

Harness memory includes objective, target identity, established observations, supported interpretations, hypotheses, decisions, unresolved questions, and branch context. Streaming drafts must remain visibly provisional until a validated insight is published.

## FM-1 V15 and pi32v2

The configured identity reference is 699,956 bytes and SHA-256 `db1642b2b6fa5c2cccb11ffd13878068bb28601678d3644049f99dc40e7edb8a`. A matching name is insufficient. Hash comparison identifies the reference; it does not prove package integrity, runtime behavior, or an original source type.

The parser milestone covers FWSC marker removal, decoded header and entries, type-specific checksums, flash extraction, SFC transformation, and application directory records. Every derived byte needs a source map. Reference-supplied keys must remain labeled as reference inputs.

The locally inspected REA raw architecture catalogue did not include pi32v2. The installed vendor disassembler is LLVM-based; the REA GNU objdump adapter is not a drop-in interface for it. Use a separate adapter. Preserve 2-, 4-, and 6-byte instructions, undecodable regions, raw output, hardware-loop continuation lines, and tool context. Validate branch/call displacement semantics with known-label fixtures before drawing resolved edges. Static disassembly is not observed execution.

Custom FM-1 firmware source and SDK examples are reference material, not recovered facts about official V15. Keep their provenance visible. Do not publish firmware or vendor executables as part of this project.

## Foundation limits

The current graph records and storage checks establish a starting point, not the complete qualified domain model. OMP, REA, package decoding, execution cancellation, full event sourcing, and the PDF workshop remain tracked work. Menus for unimplemented operations explain their prerequisites and do not fabricate results.
