# DIVULGE 0.1.0

The first Windows release connects a native evidence graph and terminal-style Harness Insight to exact-V15 decoding, the installed pi32v2 disassembler, REA entropy evidence and bounded OMP investigations. A human can inspect the question, follow the evidence and retain a reading position while work continues.

Included: the Windows executable, matching Rust source, six runnable beginner checkpoints, deliberate coordinate compiler error and answer, an 18-page workbook, a deterministic seven-record synthetic replay and its scoped review export, a source-checked 124-type catalogue, the longer talk route and dependency notices.

## Behavior and boundaries

- Native checks cover 1280x720 and 1920x1080. Graph scopes, pins, deterministic relationships, keyboard controls and an accessible relationship list preserve record identity. Canvas drawing is bounded by the 200-record limit and viewport culling. Pointer dragging passes direct egui input tests; the Computer Use pointer-drag attempt was inconclusive. Native wheel and keyboard panning were observed.
- OMP proposals are checked before dispatch. Budgets, pause, stop, stale-generation rejection, original receipts and evidence citations remain explicit. Presenter hold protects reading separately from execution control. The early-cancellation path handles a stop queued with the first request before SDK imports.
- Real V15 decoding and static instruction inspection are local. Live DeepSeek tests used synthetic teaching data only. Static instruction explanations do not prove runtime behavior, recovered function purpose or original source types. Four raw package CRC differences remain unexplained.
- Only the exact configured V15 identity is decoded. Other firmware versions are unsupported. The package/profile knowledge comes from the credited reference implementation; it is not claimed as independently recovered by the model.
- REA integration currently supports original-file entropy evidence and compatible canonical imports. It does not expose every REA operation. Vendor disassembly and the FWSC reader are separate integrations.
- Local saves use ledger envelope v2 / record schema 8. Older supported snapshots migrate as labeled baselines; unavailable chronology remains absent. Reopening executes nothing and grants no fresh-source capability. Explicit backup recovery requires Save As to preserve the damaged primary.
- Window layout, graph positions and pins are presentation state, not serialized evidence. Local cases and backups can contain paths, bytes, notes and model transcripts. Portable reviews expose deliberate byte/text/receipt options.
- Windows is verified. Linux and macOS are unverified. The executable is unsigned and does not install itself. Optional vendor, REA and OMP tools and the firmware must be supplied separately.

## Reproduce the package

Use a clean checkout, Rust with the Windows MSVC linker/SDK, and Python 3.11+ with `workshop/pdf-requirements.txt`. Run the standard Rust/workshop checks, then:

```powershell
& target/pdf-env/Scripts/python.exe scripts/package_release.py
```

The recipe rebuilds all release crates with project/home source paths remapped, rejects private source paths in the executable, builds and structurally checks the workbook, collects dependency notices, and packages only committed source plus the executable, workbook and hashes. `release.json` identifies the exact commit, compiler and artifact digests. Build output and local cases stay outside the tracked source.

Native/rehearsal evidence is maintained in [native validation](native-validation.md) and the workshop documents. Final artifact verification and publication are recorded with the release rather than inferred from successful compilation alone.
