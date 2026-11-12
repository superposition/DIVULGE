# DIVULGE: design and implementation boundaries

## Purpose

Make an AI-driven investigation legible to a person who cannot read at model speed. The human selects the target, sets the objective and bounds, reviews evidence, corrects assumptions, and steers. The AI proposes and performs bounded probes through the harness. The workbench preserves both the investigation and why each step matters.

The initial learning case is an exact official FM-1 V15 artifact. This remains a reusable investigation workbench rather than an FM-1-only editor.

## Desktop layout

Use egui's menu, panel, scrolling, scene, grid, and widget primitives. A narrow investigation tree sits beside Graph, Data and Code views with shared selection. A resizable terminal-style right panel holds Harness Insight. Data shows package and application directory tables; Graph shows the questions and evidence relationships. Code shares instruction selection between a virtualized listing, a basic-block graph and its inspector. Raw activity is available in a collapsed lower panel.

Narrative is the default graph projection. Relationships uses a force-directed layout. Positions are presentation data; moving a node never changes an evidence relationship. Layout settles before reading rather than continuously moving completed nodes. Future large-graph work needs scoping, virtualization, pinning, and coverage indicators.

Each node offers a short label and summary. Harness Insight supplies the question, purpose, action, observation, explanation, evidence, uncertainty, and next useful probe. It is human-facing rationale and a durable evidence record, not a hidden chain-of-thought display.

Questions also carry a persisted `QuestionPlan`: why the question matters, a bounded sequence of investigation steps, the decision its answer enables, sufficient-evidence criteria, and the alternative path if the check fails. Probe, observation, and explanation records explicitly identify their owning question. A support edge alone does not establish that ownership. Selecting a question puts its plan ahead of raw data; evidence records keep the question context expandable. Focus question and graph double-click explicitly zoom the camera; normal selection does not move it. Related-response links respect the replay frontier.

Schema v2 added plans and ownership links; v3 added typed firmware reports; v4 adds image-qualified scopes and disassembly runs. Version 1–3 files migrate without fabricating missing evidence. Loading checks ownership, byte ranges, mapping consistency and run identity, then reparses raw disassembler output to verify its persisted projections. A saved DTO is recorded evidence, not a freshly verified artifact.

## Attention and execution

Keep the execution frontier distinct from the human attention cursor. Selecting an earlier record or panning detaches follow mode. New results can accumulate without moving the selection or scrolling the text. Review next advances at the reader's pace; Follow latest explicitly catches up.

Recorded replay and the disassembly worker preserve this separation. A run records its starting selection; if the reader changes record, run or instruction while it works, completion retains that reading position. The worker carries a session generation and cannot publish into a replacement investigation. Presenter mode should eventually hold AI execution at completed insights; exploration mode can continue within the selected budget. Replay never runs a model or analysis tool.

## Layers

1. Domain: artifact identities, typed coordinates, observations, explanations, unknowns, and relationships. No egui dependency in domain code.
2. Application: job scheduling, cancellation, session identity, effects, persistence, and validated tool messages.
3. Adapters: REA, the vendor pi32v2 tool, and a restricted OMP SDK bridge.
4. Presentation: menus, panels, graph projections, attention cursor, and readable summaries.

The crate separates domain/storage, firmware decoding, checked coordinates, runtime decoded images, pi32 parsing/control flow, tool execution, record construction and egui views. Parsers, records and the process adapter do not depend on egui. Split into workspace crates when the executor and adapters warrant it.

## Type-driven design

`RecordId` is stable across layout and replay. `FileOffset`, `PackageOffset`, `FlashOffset`, and `MappedAddress` are distinct coordinates. `PackageHeader`, `PackageEntry`, `DirectoryEntry`, `Application`, and `CrcCheck` retain parsed values. `FirmwareView` connects a graph record to these typed results without duplicating the report. Rust validation does not establish the truth of a claim: a valid evidence link is necessary but not sufficient for a sound explanation.

The V15 reader checks ranges and constructs fragmented original-file spans across the 20 marker boundaries. `ArtifactId` and `ImageId` bind a checked `ImageMap` to SHA-256 identities; `ImageSpan` includes its image identity, offset and length. Resolution rejects cross-image, out-of-bounds and overflowing spans. The runtime-only `DecodedImage` owns freshly verified application bytes and cannot be deserialized. Reopening recorded evidence therefore cannot enable a tool run until the source is verified again. General address spaces, arbitrary load maps and the rest of the qualified domain model remain in #2.

Represent missing information explicitly. `Option` is absence, `Result` is an operation's success or failure, and analysis coverage is a separate complete/partial/unavailable outcome. Ambiguous or unresolved destinations stay unresolved. Parsed declared fields, selected-byte scalar interpretations, and inferred types have distinct provenance. Preserve floating-point bits in serialized scalar views.

Retain REA's original records and `ev_` identities. DIVULGE observations have their own identifiers. Evidence bundle order is not a timeline: the application owns a monotonic event sequence. Unknowns must retain supporting and counter evidence, scope, revision history, and resolution rationale.

## OMP bridge

Use a small Bun process around the installed OMP SDK. Reuse OMP's model catalogue and authentication; DIVULGE must not copy credentials or maintain its own provider catalogue. Discover capabilities and report the SDK/CLI versions instead of assuming they match.

The Bun bridge now creates an isolated SDK session with `read_evidence` as its sole active tool. The explicit allowlist, restricted custom tools, MCP/LSP/IRC disable flags and empty extensions/rules/skills/context are checked against SDK 18.14.2; other SDK versions fail clearly. A named in-memory session avoids automatic title generation. OMP owns credentials and model selection; DIVULGE stores only model metadata and its own evidence snapshot. No global default is changed.

Protocol v1 uses request IDs and monotonic event sequence numbers for ready, draft, evidence-read, proposal, completion, error and cancellation. A ready acknowledgement cannot publish a proposal. Rust validates proposal text limits, cited supplied records and the even 2–4096-byte image span. Schema v5 saves the context hash, exact snapshot and transcript; altered projections are rejected on load. Completion appends a graph record without moving the reader's selection or camera. Staging is a separate user action; the bounded autonomous executor remains #8.

Rust owns execution. The cycle is question -> proposed probe -> validated request -> actual observation -> evidence-linked explanation -> follow-up. Expose only explicit workbench operations. The initial proposed default is 12 probes per question, sequential, with reuse of identical evidence requests. Pause prevents new probes; Stop cancels current work. Correlate every response with session and run identity so stale results cannot affect a different investigation.

Harness memory includes objective, target identity, established observations, supported interpretations, hypotheses, decisions, unresolved questions, and branch context. Streaming drafts must remain visibly provisional until a validated insight is published.

## FM-1 V15 and pi32v2

The configured identity reference is 699,956 bytes and SHA-256 `db1642b2b6fa5c2cccb11ffd13878068bb28601678d3644049f99dc40e7edb8a`. A matching name is insufficient. Hash comparison identifies the reference; it does not prove package integrity, runtime behavior, or an original source type.

The read-only parser now removes FWSC markers, decodes header/entry records, checks header and encrypted-directory CRCs, compares raw member CRCs, extracts flash in memory, applies the reference SFC transform, and validates JLFS headers and the application payload. Four member checksum transforms remain unresolved. Every displayed byte maps back to original-file source spans. The area offset `0x4000` and key `0x980F` are reference inputs, not independently recovered facts. The application header declares `0x02000120`; it is not a runtime observation. Full decoded payloads are hashed during the import worker, with bounded previews retained in the session.

The locally inspected REA raw architecture catalogue did not include pi32v2. The vendor LLVM tool uses a separate adapter. Rust wraps a selected 2–4096-byte image span in a minimal ELF32 carrier with the declared base; the carrier is never executed. A Python helper invokes the installed disassembler without a shell, fingerprints it, captures its version and bounds output/time. Cancel, timeout, output limit and nonzero exit are separate outcomes, and partial outputs survive. The helper stops its owned process group and disables core dumps.

Accepted instruction rows must have aligned addresses, 2/4/6-byte widths, non-overlapping ranges and bytes identical to the selected input. Rejected or absent rows become explicit gaps. Hardware-loop continuation lines remain attached to their row. Only the fixture-validated tool hash and exact argument profile enable numeric branch/call targets: address + width + signed byte displacement. Other profiles still retain their raw output and instruction candidates with unresolved flow. See [the fixture evidence](pi32v2-validation.md).

Basic-block edges distinguish direct jumps, conditional alternatives, calls, possible return sites, external destinations and unresolved transfers. An address inside the selection without a decoded instruction boundary stays unresolved. Return values, hardware-loop edges and indirect destinations are unknown. Static disassembly does not establish that a span is code or that any edge executes.

Custom FM-1 firmware source and SDK examples are reference material, not recovered facts about official V15. Keep their provenance visible. Do not publish firmware or vendor executables as part of this project.

## Foundation limits

The current graph records and storage checks establish a starting point, not the complete qualified domain model. REA, the broader AI executor, full event sourcing and the PDF workshop remain tracked work. OMP currently proposes one step from a bounded snapshot; it cannot launch an analysis tool. Both model requests and vendor disassembly support cancellation. Menus for unimplemented operations explain their prerequisites and do not fabricate results.
