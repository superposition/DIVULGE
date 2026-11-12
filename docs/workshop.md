# From Bytes to Evidence: Learning Rust with DIVULGE

## Audience and outcome

Software developers who understand variables and functions but know little Rust. After the introduction, a learner can describe the data behind a graph node, follow the investigation, and distinguish an observation from an explanation or an unknown.

The 20 minutes includes instructor livecoding. Installation happens before the session. The PDF is a companion workbook, not a compressed tour of every Rust feature. A reference section covers all types actually used by the application as the implementation evolves.

## Twenty-minute route

| Time | Concept and livecoding | Visible application counterpart |
| --- | --- | --- |
| 0-2 | Question -> probe -> observation -> explanation -> next question | Narrative graph and Harness Insight |
| 2-5 | u8, u16, arrays, slices, and byte order | Selected bytes and a scalar interpretation |
| 5-8 | struct, String, and Vec<T> | Observation record and evidence card |
| 8-11 | enum and states | Question, probe, observation, explanation, unknown |
| 11-14 | Option<T> versus Result<T, E> | Unknown destination versus a failed read |
| 14-17 | References and coordinate wrapper types | Borrowed bytes, FileOffset, MappedAddress |
| 17-20 | Run the example and inspect the same records in DIVULGE | Evidence link, uncertainty, and next question |

Use `cargo run --locked --example workshop_types`. The initial example is already runnable. The full set of staged checkpoints is a separate ticket.

## Repeated page structure

Ask -> Predict -> Code -> Observe -> Explain -> Find it in the app.

Each lesson page has one primary concept, a small code block, its output, a visual link to the application, and room for a prediction. Include concise instructor prompts with expected answers in a separate appendix. Never require an audience to read a wall of model output while listening.

## Planned PDF: approximately 16-18 pages

1. Preparation and run instructions: one page, tested versions, fallback saved investigation.
2. The investigation in three forms: one illustrated spread linking bytes, Rust records, and the app.
3. Livecoding: six pages, one concept per page, exact runnable checkpoints and an intentional compiler error.
4. Guided application workshop: three pages with prediction and evidence worksheets.
5. Type reference and instructor notes: four to six pages, code links tied to a release.

## One example, progressively enriched

Start with synthetic bytes `[0x34, 0x12]`. Interpreting them with `u16::from_le_bytes` yields 4660. The value is reproducible; its intended role is unknown. The fixture is not FM-1 firmware evidence.

Add `Observation`, a `FileOffset`, and owned byte storage. Introduce `RecordData` as an enum. Deliberately pass a `MappedAddress` where a `FileOffset` is expected and discuss the compiler error. Truncate the fixture and distinguish the resulting read error from an unknown branch destination. Show how evidence IDs connect an explanation back to the observation.

The workbook must distinguish host Rust types from target firmware representations. A Rust struct does not prove that the firmware used the same struct, packing, byte order, or source language.

## Application exercise

1. Open the workshop investigation and inspect the two observed bytes.
2. Follow the explanation's evidence link.
3. State one thing the bytes support and one thing they do not establish.
4. Replay records and select an earlier insight while replay advances.
5. Use Review next and Follow latest; explain why reading position differs from execution progress.
6. Add a note, save, reopen, and replay without executing tools.
7. In the later firmware module, open a locally supplied V15 artifact and repeat the evidence discipline.

Closing task: choose an explanation, show its evidence, identify a limitation, and propose the next probe.

## Type reference contract

For each type: meaning, creator, valid states, visible UI counterpart, and mistake prevented. Group the catalogue into primitives; text and collections; structs and enums; absence and failure; IDs and coordinates; checked ranges and mappings; analysis coverage; sharing and jobs; serialization and events; presentation state.

Advanced topics such as Arc, generic coordinate spaces, restricted constructors, channels, and serialized DTO validation belong in the reference and later exercises. Do not overload the opening twenty minutes.

## Production and acceptance

Generate the PDF reproducibly from maintained content. Compile every code sample. Use real, current native screenshots with matching terms and visual semantics; avoid mock screenshots presented as implemented behavior. Use selectable text, readable code, embedded fonts, page numbers, source links, and an ink-friendly treatment. Render and inspect every page. Tie the PDF and saved investigation to a release so screenshots and types do not drift.
