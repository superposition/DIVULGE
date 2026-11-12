# DIVULGE

**See the question. Follow the evidence. Understand the investigation.**

DIVULGE is a native Rust/egui workbench for AI-assisted reverse engineering. The human sets the objective and steers; the AI will carry out bounded investigations. A graph and a detailed **Harness Insight** panel keep the process understandable at a human reading pace.

The first case study is official M-VAVE FM-1 V15 firmware and the pi32v2 architecture. A companion workshop introduces Rust through the actual types behind the application.

## Current foundation

- Native File / Edit / View / Investigation / Analysis / Model / Help menus.
- Selectable narrative graph and a small, deterministic force-directed relationship layout.
- Terminal-style Harness Insight: file offsets, selectable hex/ASCII bytes, endian-aware scalar views, artifact identity, evidence links, and the question/explanation transcript. Unmapped CPU addresses remain explicitly unresolved.
- Saved question plans explain why a question matters, how to investigate it, what its answer enables, and what evidence is sufficient. **Focus question** or a double-click zooms to the question and its next probe; **Fit all** restores the overview.
- Independent reading position and replay frontier, with Review next and Follow latest.
- Local artifact hashing, bounded byte previews, and comparison with the configured V15 identity.
- Exact-V15 FWSC decoding: eight package entries, header and directory CRCs, raw member CRC comparisons, SFC/JLFS application decoding, and source mappings back to the original file.
- A compact **Data** view shares selection with **Graph**. Inspect field locations and Rust types, sizes, hashes, raw/decoded bytes, flash offsets, and the application's declared address. **Focus result** frames a record and its immediate evidence relationships.
- Notes, versioned investigation save/open, and replay without running tools.
- A runnable Rust workshop example using synthetic bytes.

**pi32v2 disassembly, REA integration, and OMP-driven investigation are not connected yet.** Package decoding runs locally and deterministically. The built-in workshop is labeled synthetic; it does not present simulated model output as real analysis. The app does not execute or flash firmware.

## Run

Requires Rust 1.92 or newer and the native build prerequisites for your OS. Windows is the first validation target. The manifest enables native Linux window backends, but Linux and macOS have not yet been verified.

```sh
cargo run --locked
cargo run --locked -- --compact
cargo run --locked --example workshop_types
cargo test --locked --all-targets
```

Open an artifact through **File > Open artifact**, or provide a path:

```sh
cargo run --locked -- --artifact /path/to/your/firmware.fwsc
```

Artifact reads are limited to 128 MiB in this foundation build. Investigation JSON files are limited to 8 MiB, 200 records, and 1,000 edges while larger-graph work is pending. Saved investigations include a bounded byte preview; inspect their contents before choosing to share them. Machine-specific source paths and model credentials are not stored in them.

Investigation schema v3 preserves question plans, ownership links, and structured firmware results with bounded previews. Version 1 and 2 files still open; missing plans and decoded results remain absent. Reopening a recorded report does not rehash or revalidate its original file. Older builds cannot read v3 saves.

For a local parser acceptance check without opening another window:

```sh
cargo run --locked --example inspect_firmware -- /path/to/FM-1_015.fwsc
```

The V15 case produces 699,936 logical package bytes after removing 20 markers, eight entries, and a 581,564-byte application with verified CRC. The application starts at original file offset `0x4534`, flash offset `0x4120`, and declared address `0x02000120`. A declared address is static metadata. Four members have unexplained raw CRC differences; these remain unresolved instead of being labeled corrupt. The format, area offset, and SFC key are supplied by the [MIT reference implementation](THIRD_PARTY.md).

## Workshop

The 20-minute introduction follows one example: `[0x34, 0x12]` becomes an observation, a chosen interpretation, and an unanswered question. It covers bytes, collections, structs, enums, Option, Result, borrowing, and distinct coordinate types.

The full PDF is a planned deliverable. Its outline, timings, livecoding checkpoints, exercises, and type-reference strategy are in [the workshop specification](docs/workshop.md).

## Project map

- [Architecture and product decisions](docs/architecture.md)
- [Milestones and tickets](docs/roadmap.md)
- [Native UI verification](docs/native-validation.md)
- [Contributing](CONTRIBUTING.md)

The project code is MIT licensed. Third-party firmware, vendor tools, REA, OMP, and dependencies retain their own terms. Firmware and vendor binaries are not distributed in this repository.
