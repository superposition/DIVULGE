# Workshop and release rehearsal

## Measured operator walkthrough

On 2026-11-12, the planned twenty-minute route was walked through with actual live edits, Cargo runs and native Computer Use in the Windows release executable. Source: `c4cf87eeffb6e66966904cb0b699481d8081f663`; executable SHA-256: `67dc1eef7cca16feffd925d83be7853e1f7b7b8934e3286300a7d9af3ad8f32b`. One 1280x720 workbench opened the shipped synthetic replay. No model, firmware or network was involved in the lessons.

The clock ran from **20:25:40 to 20:27:29 EST: 1 minute 49 seconds**. This is measured automated operator time, including file edits, incremental compilation, observations and app navigation. It is **not** twenty minutes of spoken delivery, audience participation, manual typing speed or a beginner usability study. The twenty-minute allocation in [the lesson plan](workshop.md) remains the presenter pacing target. Audience predictions and explanations need a human rehearsal before a live event.

| Completed at (EST) | Live change and observed result | Native counterpart |
| --- | --- | --- |
| 20:25:49 | Added the big-endian conversion: 4660 versus 13330; bytes stayed 34 12. | Changed the endian selector; the same evidence range remained visible. |
| 20:26:07 | Changed the Observation's String description; its owned Vec and offset remained unchanged. | Observation 3 showed file+0, two hex bytes, and an unresolved mapped address. |
| 20:26:22 | Added Question, Observation and Explanation match arms. | The six kinds matched the graph and inspector labels. The saved replay also contains analyst review 7. |
| 20:26:39 | Truncated the borrowed slice: Ok(4660) versus Err; destination remained None. | Selected byte 12 at file+1: u8=18; u16 unavailable. No zero address was invented. |
| 20:26:49 | Borrowed then reused the Vec; triggered E0308 for MappedAddress; corrected to FileOffset(0), producing Ok(4660). | The synthetic inspector continued to say no load map. |
| 20:26:56 | Added checked evidence resolution with `?`; printed explanation, limitation and next probe. | Explanation 4 cited observation 3. |
| 20:27:29 | Finished the closing task: identified claim, evidence and limit; followed the evidence link. | The terminal showed the unknown intended meaning and the format-declaration/controlled-example probe; clicking the citation selected observation 3. |

The walkthrough found a practical Windows issue: `Copy-Item` preserved an old source timestamp, and Cargo reused the preceding example output. Editing the file caused the expected rebuild. The published commands now use `Get-Content -Raw | Set-Content` to refresh the working file, including both sides of the intentional compiler-error exercise. Completed checkpoints remain unchanged. This is why command-output checks alone did not replace the operator walkthrough.

The release Help menu also opened the updated workshop guide, including its six checkpoints, 18-page PDF and saved replay. At compact height the sidebar's guide button can sit below the visible scroll region; Help > Workshop guide remains available.

## Local firmware and failure exercises

The final local V15 rehearsal identified the exact 699956-byte reference file, parsed eight entries, decoded the 581564-byte application and mapped its first declared address to original file offset 0x4534. A fresh bounded 256-byte vendor disassembly completed with 69 rows, 34 blocks, 45 static candidate edges and zero gaps. The first direct jump's candidate destination was 0x02000124. Loop/indirect semantics and runtime behavior remained unresolved. Four raw member CRC differences remained explicit.

The fresh saved result reconstructed 20 records, one disassembly, zero model/REA receipts and zero unfinished jobs offline. The existing private talk case reconstructed 24 records, two ordered events, one disassembly, one cancelled OMP receipt and one completed REA receipt. Its SHA-256 before the final presentation check was `4a261e5cfcddffd713ffeca8d354e21ea018de6f906ce55071f00ea006933dd4`. The private case and its scoped review remain local; the public package contains the explicitly synthetic replay and review.

`scripts/rehearse_failures.py` demonstrated unavailable OMP, cancellation queued before SDK startup and a deliberately absent decoder. The unavailable SDK returned an error without a model request; queued cancellation returned a correlated cancelled event; the missing decoder produced a failed receipt and zero instruction rows. These use temporary local configuration and do not change global model settings. No cloud request used real V15 data.

The [native validation record](native-validation.md) covers actual Pause/Resume/Stop, held reading during live synthetic DeepSeek runs, compact and large layouts, source/evidence navigation, saved replay, note retention, the unsaved guard and recovery from a deliberately malformed disposable save. It distinguishes observed native keyboard/wheel panning from the inconclusive helper pointer-drag attempt. Pointer dragging has a separate direct egui input test.

## Validation and release identity

The full local host suite passed **56 library tests, 6 binary tests and 2 compile-fail doctests**, including the five normally ignored installed-tool checks. Hosted Windows CI at `c4cf87e` passed format, Clippy with warnings denied, ordinary tests, doctests, native build, all six lesson outputs, deliberate compiler error/answer, deterministic replay, the 124-type catalogue and PDF structural checks ([run 43344412852](https://github.com/superposition/DIVULGE/actions/runs/43344412852)).

The release recipe pins the workbook and package to the packaging commit, remaps private build paths, includes dependency notices and emits hashes. `release.json` and the published asset checksums are the final artifact identity; earlier validation revisions above identify the executable behavior actually observed. The [release notes](release-notes.md) describe platform and integration limits.
