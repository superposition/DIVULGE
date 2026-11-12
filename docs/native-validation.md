# Native UI verification

DIVULGE is a native eframe application. Browser DOM automation cannot verify its window. Use native Computer Use to select the returned DIVULGE window, capture it, act on the observed UI, and capture again.

## Repeatable checks

- Build and launch the actual executable.
- Inspect the 1440x900 layout and `--compact` 1280x720 layout.
- Select a graph node and confirm Harness Insight changes.
- Follow an evidence link back to its observation.
- Change between Narrative and Relationships; pan/zoom and fit.
- Replay, review an older node, and verify advancing records do not steal selection.
- Inspect File/Edit/View menus and relevant keyboard shortcuts.
- Open a real local artifact, inspect its hash and bounded bytes. For the exact V15 identity, check decoded package/app data and keep instruction/runtime claims unresolved.
- Add a note, save, reopen, and verify persistence.
- Inspect keyboard focus, accessibility labels, dialogs, and error feedback.

Screenshots must show actual rendered state. Filesystem/process checks alone do not count as visual inspection. Keep screenshots local unless reviewed for public sharing. Record dates, build identity, completed checks, and remaining gaps below.

## Foundation validation

2026-11-12, Windows, initial 0.1.0 foundation:

- Built and launched the native executable; Computer Use captured its actual 1440x900 client layout and read its accessibility tree.
- Observed the File menu and compact desktop layout, selected an observation directly in the graph, and followed an explanation's evidence link back to that observation.
- Switched to the settled force-directed relationship view and exercised recorded replay. Historical selection remained independently reviewable (Reading 01 / Latest 06).
- Used the native artifact chooser to open the locally supplied FM-1 V15. The app observed 699,956 bytes and the configured SHA-256 match, and explicitly reported that package decoding is still pending.
- Entered a checkpoint note in the native editor. Opened the native Save dialog and supplied a local path, but the automated interaction did not produce a confirmed save. The storage round-trip/overwrite test passes; the native dialog path and reopen flow remain to be verified in #1.
- UI inspection prompted smaller relationship arrowheads and record-specific workshop explanations that do not expose a later interpretation in an earlier step. Those final source changes pass the checks below; a fresh visual pass remains pending.
- Six domain/storage tests pass, the workshop example runs, and format/lint checks pass.

Still pending: the final-build visual pass, 1280x720 and 1920x1080 checks, native save/reopen completion, pan/zoom and resize checks, and broader keyboard/accessibility review. These are tracked in #1, #10, and #14. No model, firmware execution, or flashing was invoked.

## Terminal evidence inspector

2026-11-12: built a separate native preview with the terminal-style right pane and the actual locally supplied V15 artifact. Visually verified the monospace transcript, artifact identity, file range, hex/ASCII rows, and scalar values. Selected the byte at file offset 0x00000001 through Computer Use and observed the little-endian u16 view change to 0x2F7B and u32 to 0x3F0C2F7B. The pane explicitly distinguishes file offsets from unresolved mapped addresses and states that instructions have not been decoded. Seven tests and Clippy pass. The earlier native save/reopen and compact-layout checks remain open.

## Question focus

2026-11-12: built and opened the question-plan preview on the local V15 artifact. Through native Computer Use, clicked Focus question from the identity explanation. Observed selection move to question 0001, the graph zoom onto the question and next probe, and the terminal pane show WHY, HOW, TO WHAT END, ANSWERED WHEN, IF NOT, and links to the available probe/observation/explanation. Fit all remains visible as the return to overview. Eight tests pass, including question ownership/plan validation and v1-to-v2 load migration without invented context. Native save/reopen verification remains pending as recorded above.

## Real V15 package data

2026-11-12: closed both superseded previews and saved the remaining five-record investigation through the native Save dialog before replacing that preview. The save was confirmed on disk (schema 2, five records, no notes); native reopen is still pending. Keep only one workbench window open during future iterations.

Built the updated native application and visually inspected its 1440x900 data view with the local V15 file. Observed eight members, four unresolved raw CRC comparisons, six JLFS directory entries, and the checked application span. The terminal pane displays original file `0x4534`, package `0x4520`, flash `0x4120`, declared address `0x02000120`, length 581,564, app CRC `5D48`, area CRC `F399`, SHA-256 and decoded bytes. Selecting USR changed the inspector to type 50, its actual offsets, lengths, hash and stored/calculated CRC `E3BC/5419`. Switching to Graph retained USR and framed its directory relationship. Returned to Data afterward.

The local `inspect_firmware` example verifies the application identity and address mapping and validates a serialized report round-trip. Synthetic tests cover CRC's standard vector, marker-boundary fragmentation, truncation, out-of-bounds/overflow ranges, invalid declared sizes, exact-profile rejection and unresolved member checksums. No firmware fixture is committed. Clippy and the native build pass. The compact layout, native reopen, complete keyboard/accessibility review and broader replay interactions remain tracked checks; the native accessibility provider truncates this dense view's root tree, so this visual pass used screenshot coordinates.

## pi32v2 instruction workspace

2026-11-12, Windows, schema-v4 implementation of #5:

- Saved the current 17-record V15 investigation through the native Save dialog, confirmed the file, closed its window, and reopened that same investigation in the updated build. Supplying its original artifact reverified and attached application bytes without replacing the records.
- Through native Computer Use, opened Code and ran the first 256 application bytes. Observed `Completed`, 69 instruction rows, 256/256 decoded bytes and complete coverage. The listing shows original file offsets and 2/4/6-byte widths alongside addresses, bytes and tool text.
- The first instruction is `goto 0x2` at declared `0x02000120`, original file `0x4534`. Its inspector shows target `0x02000124`, the PC-plus-width calculation, question, explanation, significance, next probe and remaining unknowns.
- Clicked **Inspect target instruction** and observed selection move to `sp = 0x1c16fd0` at `0x02000124`, file `0x4538`, width six. The listing scrolled to it and the inspector changed together.
- Opened Control flow and inspected the 34-block projection. A distant call initially made focus too wide; Focus block now frames the selected block at reading size, with Fit all retaining the overview. Static candidates and unresolved hardware loops remain labeled.
- In the final build, verified the jump arrow from B00 to B02, clicked B02 and observed the inspector change to its first instruction, then clicked Focus block. Its three instructions remained readable despite the distant call destination.
- Saved the run with Ctrl+S; confirmed schema 4, 20 records, one run and 69 rows on disk. Reopened that save in the native application and observed the same listing and retained result. Only one DIVULGE window is kept open.

The headless adapter acceptance run also produced 34 blocks, 45 candidate edges and zero gaps, then validated a serialized/deserialized report. The synthetic vendor-tool suite checked 28 branch/call pairs at two bases in both linked and symbol-free carriers; see [the detailed evidence](pi32v2-validation.md). No firmware or vendor executable is committed.

Validation: `cargo test --locked --all-targets` passes 21 standard tests; three host-dependent tests are ignored by that command and were separately run successfully. Those tests exercise cancellation of an owned child process group, partial output after failure/SIGSEGV, output caps and the 15-second deadline. `cargo clippy --locked --all-targets -- -D warnings`, formatting and the native build pass.

Remaining UI checks: final compact and large-window layouts, broader keyboard/accessibility coverage, graph scaling with larger selections, and live attention preservation under slow jobs. Save/reopen is now visually confirmed; a dedicated note-edit round trip remains in the broader foundation checklist. OMP/REA, firmware execution and flashing were not invoked.

## OMP proposal bridge

2026-11-12, schema v5: native Computer Use verified Model > OMP models and next step, catalogue discovery, the SDK 18.14.2 / CLI 18.15.0 version-skew message, and the preserved configured default. The selected default local endpoint was offline; other configured local endpoints were also offline. No server or global model setting was changed.

Started and cancelled a request in the same V15 investigation. Cancellation recorded an incomplete AI run while the reading cursor stayed at record 20 and the frontier advanced to 21. Review receipt selected its graph node and displayed the cancelled outcome and expandable model/context transcript. Ctrl+S saved schema 5 with 21 records and one cancelled AI run. Reopened that save in the final build, selected recorded request 21 from the OMP window, and reviewed its retained graph node and cancellation receipt. The existing investigation was backed up before migration; only one workbench window was maintained.

The real synthetic acceptance test used the configured `deepseek/deepseek-flash` cloud model. It exposed exactly `read_evidence`, called it with record 3, streamed provisional text, and completed a checked proposal for offset 4, length 16. Its question, why, method, outcome and unknowns were populated; Rust validated the citation and scope. The exact protocol/context receipt remains local in ignored `target/omp-acceptance.json`. No firmware was sent in this cloud test. Successful proposal rendering and staging on a real model result still need a dedicated native visual pass; the model picker, cancellation, receipt view and save were directly observed.

Validation: 24 standard tests pass. The new host-dependent correlated-cancellation test was invoked separately and passed; three existing pi32 host tests remain ignored by the ordinary suite and were verified in the preceding milestone. Clippy and the native build pass. Unit tests reject stale IDs, out-of-order frames, ambient tools, invalid citations, unaligned/overflowing spans, altered saved projections and missing completion.

## REA entropy evidence

2026-11-12, schema 6, Windows: native Computer Use opened **Analysis > REA original-file evidence**, detected installed REA 6.8.0 and its supported adapter capabilities, and ran `inspect_byte_entropy` for file offset 0, length 4096, block size 4096. Completion appended record 22 while reading stayed at record 20; the existing 69-row instruction view remained in place. Review recorded result selected its graph node and the terminal inspector. The inspector showed REA's original `ev_` ID, provider `byte-entropy`, shipped-artifact authority, selected file range, file-wide entropy 7.9922, selected block entropy 7.6585, unavailable relationship coverage, and the original limitations.

Ctrl+S saved schema 6 with 22 records and one completed REA run. Closed that window and reopened the same file in the final build with `--compact` and no artifact argument. The 1280x720 client layout showed retained instructions, disabled live disassembly until source verification, and readable REA evidence selected through the tree filter. No REA or model request was started by reopening. A native analyst note was entered on record 22 and saved with Ctrl+S; its exact text was confirmed in the saved JSON. Only one workbench window was kept open. Graph title/summary overlap found during this pass was corrected with bounded text rows and a full-title hover display, then visually checked in the compact build.

The headless real V15 run inspected file offset 17716 (`0x4534`), length 256 and saved/reloaded its complete receipt. The selected block entropy was 7.0979; file-wide entropy was 7.9922. These are original-file observations, not decoded application measurements or proof of encryption. Original receipts remain in ignored local `target` files; public fixtures use generated synthetic bytes and clearly synthetic paths.

Validation: 28 standard tests pass, including canonical fixture preservation, identity/manifest tampering, missing fields, examined-empty versus unavailable output, partial transport, invalid scopes and stale response IDs. The host REA test separately passed discovery, cancellation, canonical synthetic bundle import and malformed-bundle rejection. Clippy with warnings denied, formatting and the native build pass. The remaining four host-dependent OMP/pi32 tests were verified in their preceding milestones. General REA operations and unknown-history bundle imports remain explicitly outside this adapter profile. The broader foundation checks still include native note reopen, pan/zoom, panel resizing and the 1920x1080 pass.
