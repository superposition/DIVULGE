# From Bytes to Evidence: a 20-minute Rust workshop

Developers who know variables and functions but little Rust use one synthetic example: `[0x34, 0x12]`. They build a vocabulary for the actual records in DIVULGE, then follow the evidence in the native application. A host Rust type or scalar view does not establish the original firmware's language, packing, byte order or meaning.

The opening twenty minutes teaches a small connected path. The [124-type catalogue](type-catalogue.md) and [Rust forms reference](rust-forms.md) cover the rest of the implemented program for later reading. No model, firmware, vendor decoder or network is needed for this module.

## Prepare before the clock starts

Use the matching source/release package. Windows is the verified desktop target; other platforms remain unverified. Source builds need a supported Rust MSVC toolchain, the Microsoft C++ linker/Windows SDK, and an OpenGL-capable desktop. The manifest requires Rust 1.92 or newer; the local teaching checks used rustc/cargo 1.94.1. Python 3.8+ is needed only for the workshop verifier and document tooling.

Run from the repository root before the session:

```sh
cargo build --locked --examples
cargo build --locked --bin divulge
python scripts/check_workshop.py
python scripts/check_catalogue.py
cargo run --locked -- --compact --session workshop/fixtures/synthetic-workshop.json
```

The package includes completed checkpoints, the deliberate error and answer, a deterministic saved replay, and a portable synthetic review. Installation, dependency downloads and first compilation are outside the twenty minutes. The presenter opens the editor and keeps one workbench window. If compilation is unavailable, use the prebuilt native release and the completed code/output pages.

## Timed route

| Clock | Activity | Learner-visible result |
| --- | --- | --- |
| 00:00-02:00 | Ask what the two bytes can establish. Show question, probe, observation and explanation. | Evidence and meaning are different claims |
| 02:00-05:00 | Checkpoint 1: u8/u16, array, slice and byte order | 4660 versus 13330 from the same bytes |
| 05:00-07:30 | Checkpoint 2: Observation, Vec and String | Owned evidence with a source coordinate |
| 07:30-10:00 | Checkpoint 3: RecordData and match | Questions, actions and claims have different variants |
| 10:00-12:30 | Checkpoint 4: Option versus Result | Unknown address versus insufficient bytes |
| 12:30-16:00 | Checkpoint 5: borrowing and coordinate wrappers; intentional compiler error | A CPU address is rejected by a file reader |
| 16:00-18:00 | Checkpoint 6: checked evidence IDs and ? | Explanation 4 resolves to observation 3 |
| 18:00-20:00 | In the app, follow evidence, identify a limitation and choose a next probe | Learner states an evidence-backed claim with a boundary |

Each lesson follows **Ask -> Predict -> Code -> Observe -> Explain -> Find it in the app**. The exact questions, outputs, live edits and instructor answers are maintained in [lessons.json](../workshop/lessons.json). The PDF reads the same source and code files. The rehearsal record must distinguish technical command checks from an actual timed operator or spoken run.

## Six runnable checkpoints

Run one at a time:

```sh
cargo run --locked --example workshop_01_bytes
cargo run --locked --example workshop_02_observation
cargo run --locked --example workshop_03_records
cargo run --locked --example workshop_04_absence
cargo run --locked --example workshop_05_coordinates
cargo run --locked --example workshop_06_evidence
```

1. **[Bytes](../examples/workshop_01_bytes.rs).** Predict both integer interpretations. Live-code the second conversion. The bytes stay `34 12`; little endian gives 4660 and big endian gives 13330. Find record 3 and switch the inspector's endian choice. Do not call either value the intended field meaning.
2. **[Observation](../examples/workshop_02_observation.rs).** Ask what another person needs to check the result. Fill the source offset, owned bytes and owned description. Change the description without changing the evidence. Find the offset and hex cells on record 3.
3. **[Record kinds](../examples/workshop_03_records.rs).** Predict the kinds in the graph. Write three match arms and discuss the others. Record 5 is an Unknown with a question plan; record 6 is a Question. A valid enum variant is a structural classification, not proof that a claim is true.
4. **[Absence and failure](../examples/workshop_04_absence.rs).** Truncate the borrowed slice to one byte. Observe `Err("Two bytes are required")`; keep a destination at `None`. In the app, select the last byte and observe that wider scalar views are unavailable. Explain why zero is not a substitute for unknown.
5. **[Borrowing and coordinates](../examples/workshop_05_coordinates.rs).** Borrow `&bytes`, then reuse the owner. Compare `FileOffset` with `MappedAddress`. Run the intentional error below. An explicit mapping, not a wrapper cast, relates a file location to a CPU address.
6. **[Evidence](../examples/workshop_06_evidence.rs).** Resolve the explanation's ObservationId and print its limitation/next step. The same link is clickable in Harness Insight. `?` propagates a validation failure rather than making up a record.

The original [workshop_types](../examples/workshop_types.rs) remains a compact combined example. These six checkpoints progressively add concepts; each is independently runnable so a learner can rejoin at any stage.

## Deliberate compiler error and instructor answer

[wrong_coordinate.rs](../workshop/exercises/wrong_coordinate.rs) passes a `MappedAddress` to `read_u16_le`, which requires `FileOffset`. The verifier compiles it against this repository's actual library and requires **E0308**, including both type names. It also compiles and runs [coordinate_answer.rs](../workshop/exercises/coordinate_answer.rs).

To show the error live in a normal Cargo example, copy the exercise to an untracked working file:

```sh
# PowerShell
Copy-Item workshop/exercises/wrong_coordinate.rs examples/live_workshop.rs
cargo run --locked --example live_workshop
Copy-Item workshop/exercises/coordinate_answer.rs examples/live_workshop.rs
cargo run --locked --example live_workshop
```

The intentional failure is expected. The corrected example prints `Ok(4660)`. A POSIX shell can use `cp` for the same copies. Remove the temporary example when finished.

**Ask:** Why not simply construct `FileOffset(address.0)`? **Answer:** That silences the type mismatch without establishing a mapping. In this fixture we know only that the owned two-byte buffer starts at offset zero. The declared address shown in checkpoint 5 is illustrative metadata, not a mapping to those bytes.

## Instructor pacing and recovery

Pre-open all six completed files. Type only the live edit named in the lesson, run it and ask for one prediction. Do not type boilerplate, tool setup or a long transcript during the slot. Reserve the last two minutes for the closing task.

If someone falls behind, give them the completed checkpoint and its expected output; continue with the concept. If a build exceeds its time box, show the verified output and move to the saved native replay. If the network or OMP is unavailable, nothing changes in this module. A live model is optional in the later talk.

The shortest useful explanation is: an observation records what was seen; an interpretation records what we chose to read from it; uncertainty names what that does not establish. Show the linked records as you say this. Do not describe model output as hidden reasoning or as automatically trustworthy.

## Guided application worksheet

Open `workshop/fixtures/synthetic-workshop.json`. Its seven records contain the original six-card fixture, a saved note and an analyst correction. It contains two ordered history events and no running workers, source file or model receipt.

1. Select explanation 4. Write its claim in one sentence.
2. Follow evidence 3. Copy the observed bytes and source range.
3. Choose the other byte order. Which part changed: evidence, interpretation or meaning?
4. State a limitation that remains true under both byte orders.
5. Inspect correction 7. Verify that explanation 4 still exists and the correction links to it.
6. Use Investigation > Replay from beginning. Select an older card while the frontier advances. Explain Reading versus Latest.
7. Use Review next, Follow latest and Presenter hold. Hold concerns reading; Pause AI concerns execution.
8. Add a note, save a copy, reopen it and find the exact note. Reopening runs no model or tool.
9. Inspect the portable synthetic review. It is for reading, not an executable session or canonical REA bundle.

**Closing task:** choose an explanation, identify its evidence, name one limitation, and propose a probe that would distinguish competing meanings. A good next probe is specific enough to have an observable outcome.

## Type reference and later FM-1 module

The [catalogue](type-catalogue.md) gives every implemented named Rust type its meaning, creator, validation boundary, UI counterpart and mistake made harder. A checker compares it with source declarations, including macro-generated IDs and spans. [Rust forms](rust-forms.md) covers primitives, collections, ownership, channels, atomics, serde and egui types. Advanced forms stay out of the first twenty minutes.

The later firmware module follows identity -> package -> mapped application -> pi32v2 observations -> explanation -> unresolved question. Keep file offsets, image offsets, flash offsets and declared addresses distinct. Static disassembly does not prove runtime behavior, source types or function purpose. The reference firmware and vendor tools are obtained separately; no firmware is in the workshop package.

## Reproduce the teaching assets

```sh
cargo run --locked --example prepare_workshop
python scripts/check_workshop.py
python scripts/check_catalogue.py --write
```

The generator is deterministic and uses only the built-in synthetic fixture. The checker compares exact example outputs, rejects the wrong coordinate with the expected compiler error, runs the answer, regenerates both JSON files and reconstructs the saved replay offline. The 18-page companion workbook is generated and visually checked; see [its build and review instructions](workbook-build.md). Timed operator rehearsal and final release packaging remain separate checks.
