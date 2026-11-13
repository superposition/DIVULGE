# DIVULGE

**See the question. Follow the evidence. Understand the investigation.**

DIVULGE is a native Rust/egui workbench for AI-assisted reverse engineering. The human sets the objective and steers; the AI carries out bounded investigations. A graph and a detailed **Harness Insight** panel keep the process understandable at a human reading pace.

The first case study is official M-VAVE FM-1 V15 firmware and the pi32v2 architecture. A companion workshop introduces Rust through the actual types behind the application.

## Current foundation

- Native File / Edit / View / Investigation / Analysis / Model / Help menus.
- Selectable narrative and settled relationship layouts, branch/neighbor scopes, pinned positions, keyboard navigation and an accessible relationship list. See [graph navigation](docs/graph-navigation.md).
- Terminal-style Harness Insight: file offsets, selectable hex/ASCII bytes, endian-aware scalar views, artifact identity, evidence links, and the question/explanation transcript. Unmapped CPU addresses remain explicitly unresolved.
- Saved question plans explain why a question matters, how to investigate it, what its answer enables, and what evidence is sufficient. **Focus question** or a double-click opens its branch; **Fit scope** shows the overview and **Center selected** restores readable labels.
- Independent execution frontier and reading checkpoint, with new-record counts, Review next, Follow latest and Presenter hold during live work. [Durable branch memory](docs/harness-memory.md) retains relevant evidence, hypotheses, counter-evidence and analyst decisions for OMP.
- Local artifact hashing, bounded byte previews, and comparison with the configured V15 identity.
- Exact-V15 FWSC decoding: eight package entries, header and directory CRCs, raw member CRC comparisons, SFC/JLFS application decoding, and source mappings back to the original file.
- A compact **Data** view shares selection with **Graph**. Inspect field locations and Rust types, sizes, hashes, raw/decoded bytes, flash offsets, and the application's declared address. **Neighbors** brings a record's immediate relationships together.
- A **Code** view runs the installed vendor pi32v2 disassembler on selected application bytes. It shows 2/4/6-byte instructions, original source offsets, a conservative basic-block graph, and instruction questions, explanations and next probes.
- Each run retains its tool identity, version, flags, exact input, raw outputs, coverage and outcome. Unknown transfers and hardware loops remain unresolved. Cancellation, timeout and tool failure preserve partial output.
- Ordered case history, retained note revisions, analyst steering/corrections/resolutions, offline checkpoint replay, backup recovery and scoped review exports. See [history and sharing](docs/history.md).
- **Model > OMP models and next step** discovers the installed OMP catalogue and proposes a source-linked next byte span. Review its why/how/outcome in Harness Insight, then stage the span in Code. Requests are cancellable; drafts and incomplete outcomes remain distinct from completed proposals.
- Six runnable Rust workshop checkpoints, a checked type catalogue and a deterministic synthetic replay with an analyst correction.
- **AI cycle** runs checked proposals sequentially within a chosen scope and budget, with Pause, Resume, Stop and reuse of completed requests. Each follow-up explanation cites actual evidence and states its limitations. See [bounded execution](docs/execution.md).

Bounded AI cycles support the synthetic byte fixture and freshly verified V15 application disassembly. Manual proposals and probe controls remain available. REA supplies canonical entropy evidence through its separate manual operation. Package decoding and the vendor disassembler run locally. Instruction explanations are deterministic descriptions; OMP interpretations are labeled inferred model output. The app does not execute or flash firmware.

## Run

Requires Rust 1.92 or newer and the native build prerequisites for your OS. Windows is the first validation target. The manifest enables native Linux window backends, but Linux and macOS have not yet been verified.

```sh
cargo run --locked
cargo run --locked -- --compact
cargo run --locked -- --large
cargo run --locked --example workshop_types
cargo test --locked --all-targets
```

Open an artifact through **File > Open artifact**, or provide a path:

```sh
cargo run --locked -- --artifact /path/to/your/firmware.fwsc
```

Artifact reads are limited to 128 MiB. Investigation JSON files are limited to 16 MiB, 4,096 events, 200 records, 1,000 edges and 16 disassembly runs. Each run selects 2–4096 bytes and retains at most 256 KiB per output stream. Local cases and their backups retain byte previews, raw tool output, model snapshots and machine paths, including paths in original REA bundles. Credential stores are never copied. Use **File > Export portable review** to choose the material to share.

New saves use a version 2 event-ledger envelope and schema 8 records. Version 1 ledgers remain readable with their original content commitments. Plain schema 1–7 cases migrate as labeled starting snapshots; missing history remains absent. **View > Case history and review** reconstructs checkpoints and appends explicit decisions without rewriting original findings. Reopening runs no tools or models. **File > Verify source** reattaches freshly verified application bytes without replacing notes or records. Older builds cannot read the new envelope.

## OMP next-step proposals

Requires Bun and a separately installed OMP SDK **18.14.2**, the version whose isolation contract is checked here. The CLI may have another version; both are displayed. The bridge discovers the SDK under `~/.bun/install/global/node_modules/@oh-my-pi/pi-coding-agent`; `DIVULGE_OMP_SDK` overrides this directory and `DIVULGE_BUN` overrides the runtime executable. It does not install or upgrade OMP. Auth and the model catalogue come from OMP's existing configuration.

Choose **Model > OMP models and next step > Connect / refresh models**. Keep the discovered default or filter/select another tool-capable model. **Propose next step** sends a bounded snapshot (objective, durable branch memory, up to twelve evidence summaries and at most two 80-row excerpts) to that model's provider. Expand the snapshot to review it first. An offline local server must be started separately or another model selected.

The session exposes one read-only `read_evidence` tool with no filesystem access. MCP, LSP, IRC, extensions, rules, skills and context discovery are disabled. The active tool list is checked before any generation. Rust checks citations, integer offsets, alignment and bounds before recording a proposal. **Stage span in Code** fills the selection; it does not execute it. A new disassembly still requires freshly verified source bytes.

Requests have a 90-second SDK deadline and a 100-second transport deadline with three seconds for cooperative cancellation. Output is capped at 1 MiB, a line at 256 KiB, and a session at 16 AI requests within the 16 MiB save limit. Saves include the exact supplied snapshot and model event transcript, including provisional text; review them before sharing. The global OMP default is never changed.

A real model acceptance check uses synthetic teaching data (substitute a configured provider/model):

```sh
cargo run --locked --example omp_proposal -- --synthetic provider/model
```

For a local parser acceptance check without opening another window:

```sh
cargo run --locked --example inspect_firmware -- /path/to/FM-1_015.fwsc
```

The V15 case produces 699,936 logical package bytes after removing 20 markers, eight entries, and a 581,564-byte application with verified CRC. The application starts at original file offset `0x4534`, flash offset `0x4120`, and declared address `0x02000120`. A declared address is static metadata. Four members have unexplained raw CRC differences; these remain unresolved instead of being labeled corrupt. The format, area offset, and SFC key are supplied by the [MIT reference implementation](THIRD_PARTY.md).

## Read evidence with REA

**Analysis > REA original-file evidence** detects the locally installed REA 6.8.0 package and exposes its read-only `inspect_byte_entropy` operation. Select a file offset, window length (at most 1 MiB), and power-of-two block size. This measures original file bytes, not decoded `app.bin` bytes or mapped memory. The full-file hash must match the current artifact. A reopened case requires **File > Verify original source** before another live read.

The adapter uses an existing compatible Node runtime and REA package. On Windows it discovers the explicit Node/package paths in an installed `rea.cmd` launcher. Otherwise set `DIVULGE_REA_NODE` and `DIVULGE_REA_PACKAGE`, or enter those paths under **Installed REA / prerequisites**. It never installs, repairs, or executes the target. Other package versions fail with an actionable message; native desktop verification is Windows-only.

REA's original `ev_` identity, provider, authority, locations, raw/normalized results and limitations stay intact. A `null` raw result or relationship coverage remains unavailable; an empty examined result remains empty. Canonical entropy bundles for the current artifact can be imported; other operation profiles and residual-unknown bundles are explicitly unsupported. Schema 6 stores the complete original bundle and its semantic commitments. Offline replay checks identities, scope and manifests in Rust and runs neither REA nor a model. Canonical bundle order is not a timeline: each receipt is appended at its DIVULGE application position.

Requests have a 30-second deadline, two seconds for cooperative cancellation, a 2 MiB response cap and a 16-run limit. Incomplete output is diagnostic text, never completed evidence. Saved original bundles contain local paths; portable sharing is a separate export concern.

```sh
cargo run --locked --example rea_evidence -- /path/to/artifact 0 4096
```

## Disassemble a selection

On Windows, install the vendor pi32v2 tools and Python 3 in WSL. **Code > Tool / scope** selects the distribution (default `Ubuntu-22.04`) and executable. An empty executable field discovers only `~/.jieli/toolchain/pi32v2/bin/objdump` inside that distribution. The tools are supplied separately; there is no automatic download. The helper also supports a native Linux host, which has not had desktop validation.

Open V15, choose **Code**, leave the initial `app.bin+0x0 / 256 bytes`, and click **Disassemble selection**. Select an instruction to inspect its bytes and rationale, follow **Inspect target instruction**, or switch to **Control flow**. The first local V15 selection produced 69 instruction rows, 34 blocks and no uncovered bytes. Its first jump resolves from `0x02000120` to `0x02000124`.

Only the validated tool hash and exact flags enable numeric target interpretation. The [synthetic fixtures and validation notes](docs/pi32v2-validation.md) explain the checked displacement convention, gaps, hardware loops and limits. Static edges are candidates, not observed execution or recovered function names.

The same adapter can run without another window:

```sh
cargo run --locked --example disassemble -- /path/to/FM-1_015.fwsc
cargo run --locked -- --session investigation.divulge.json --artifact /path/to/FM-1_015.fwsc
```

## Workshop

The 20-minute introduction follows one example: `[0x34, 0x12]` becomes an observation, a chosen interpretation, and an unanswered question. It covers bytes, collections, structs, enums, Option, Result, borrowing, and distinct coordinate types.

Follow [the workshop route](docs/workshop.md) for six runnable checkpoints, the intentional compiler-error exercise, expected answers and recovery instructions. The [type catalogue](docs/type-catalogue.md) is checked against all 124 named application types. The companion PDF and timed rehearsal remain tracked deliverables.

```sh
python scripts/check_workshop.py
python scripts/check_catalogue.py
cargo run --locked -- --compact --session workshop/fixtures/synthetic-workshop.json
```

## Project map

- [Architecture and product decisions](docs/architecture.md)
- [Checked domain types and identity rules](docs/domain.md)
- [Milestones and tickets](docs/roadmap.md)
- [Native UI verification](docs/native-validation.md)
- [Contributing](CONTRIBUTING.md)

The project code is MIT licensed. Third-party firmware, vendor tools, REA, OMP, and dependencies retain their own terms. Firmware and vendor binaries are not distributed in this repository.
