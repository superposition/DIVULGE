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
