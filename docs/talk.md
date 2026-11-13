# DIVULGE: make reverse engineering inspectable

The 75-minute developer route combines the opening Rust workshop with the FM-1 V15 case. The audience should leave able to identify a claim's evidence, distinguish coordinate spaces and state a useful next question. Rust experience is optional for the first module.

## Prepare before the audience arrives

Use the release package, its matching source and the 18-page workbook. Prebuild the examples and run `scripts/check_workshop.py`. Open one workbench window with the supplied synthetic replay. Pre-open the six source checkpoints; install nothing during the twenty-minute Rust slot.

For the later firmware module, supply the exact V15 file locally. Check its identity with `inspect_firmware` and rehearse one 256-byte application disassembly. Keep a local saved case with the observed rows and a separately generated portable review. Neither real firmware nor the private local case belongs in the public teaching package.

The installed-tool prerequisites are independent: the FWSC reader is part of DIVULGE; the pi32v2 decoder is a separately supplied vendor tool; REA 6.8.0 supplies the supported entropy operation; Bun and the validated OMP SDK 18.14.2 supply model access. Show the actual discovery state. A configured model name does not prove a server is running.

## The 75-minute route

| Clock | Question and demonstration | Evidence to show |
| --- | --- | --- |
| 00-05 | What makes a plausible explanation checkable? Select a question and show its why/how/outcome. | Graph IDs, kinds and Harness Insight |
| 05-25 | Run the twenty-minute Rust module. Ask, predict, make the small edit, observe, explain and locate it in the app. | Six checkpoints, deliberate E0308, worksheet closing task |
| 25-35 | How do we know which bytes we are examining? Identify V15, unpack the container and map its application. | Full-file hash, package fields, eight entries, application hash and source fragments |
| 35-50 | What does pi32v2 tell us, and what does it leave unknown? Decode the first 256 application bytes and inspect the first jump. | 69 recorded rows in the verified reference run; distinct file/image/declared coordinates; coverage and tool receipt |
| 50-60 | What does REA contribute? Inspect the retained original-file entropy result and its canonical evidence identity. | Operation, exact range, authority/provider, raw/normalized results, limits and unavailable fields |
| 60-70 | How can AI move faster than a human without losing the human? Use only synthetic data for the optional live cycle. | Proposal versus actual probe, budget, Pause/Stop, Presenter hold, Review next and branch memory |
| 70-75 | Can another person challenge this tomorrow? Add a scoped review, reopen a saved case and finish with a next useful probe. | Original finding preserved, cited observation, limitation, ordered history and offline replay |

For a 60-minute slot, keep the full Rust module, reduce the pi32v2 segment to ten minutes, combine the REA and AI explanation into ten minutes using saved results, and reserve five minutes for review. For 90 minutes, add fifteen minutes after the pi32v2 segment for audience hypotheses, target-instruction navigation and source-map questions. Do not fill that extension by claiming unobserved runtime behavior.

## Name the steps as they happen

1. **Identify.** Read and hash the exact artifact. A name is not identity, and a matching hash is not a trust verdict.
2. **Unpack.** Parse the bounded container and record each transformation. The V15 format profile is supplied by a reference implementation; it was not rediscovered by the model on stage.
3. **Map.** Relate original-file, package, flash, decoded-image and declared-address spaces through checked mappings. Preserve ambiguity and missing maps.
4. **Observe.** Run the selected read-only operation. Retain its request, tool identity, output, coverage and outcome. A failure or cancellation is a recorded outcome, not new evidence.
5. **Explain.** State what the observation establishes, why it matters and the assumptions used. Label deterministic interpretation separately from inferred model output.
6. **Challenge.** Follow the citation. Add a correction, counter-evidence or a scoped resolution while preserving the original finding.
7. **Probe.** Choose the smallest next observation that could distinguish competing meanings. Enforce scope and budget before dispatch.
8. **Replay.** Save the ordered history. Reopening reconstructs recorded work without executing tools or contacting a model.

REA is the evidence-producing system behind the supported entropy operation. DIVULGE is the native harness and teaching interface around selected operations. Its custom FWSC reader and vendor pi32v2 adapter are separate integrations. The force layout changes presentation, not the canonical evidence. The right pane is a retained explanatory record, not a transcript of hidden model reasoning.

## The first pi32v2 question

Ask: "Where does this direct jump lead?" The observed first row has two bytes, `04 81`, at `app.bin+0x0`, original `file+0x4534`, declared `0x02000120`. The validated tool profile interprets it as `goto 0x2`. Its signed displacement is added to the next PC, yielding candidate destination `0x02000124`. Follow that target, retain its bytes and ask whether its instruction boundary is valid.

To what end: choose the next bounded inspection while keeping every coordinate visible. The row does not establish that the device takes this branch at runtime, the values of registers, a recovered function name or a source-language type. Four raw package-member CRC differences remain unexplained; do not silently call those members corrupt.

## Failure and fallback route

| Condition | Honest demonstration / fallback |
| --- | --- |
| OMP or its server is unavailable | Show the actual error. Use a saved synthetic receipt, or continue the model-free lessons. Never substitute a fixture and call it a live answer. |
| Vendor decoder is missing | Show the failed receipt and zero decoded rows. Open the previously saved reference run and label it recorded. |
| Malformed or unsupported artifact | Show the identity/parser boundary; do not force it through the V15 profile. |
| Stop during work | Preserve the incomplete receipt; verify that no later result becomes a completed probe. Hold reading is a separate control. |
| Damaged save | Demonstrate explicit recovery on a disposable synthetic copy. Preserve the damaged primary and Save As to a new file. |
| No network | Open the supplied synthetic replay and workbook. The opening Rust module and offline review remain fully usable. |

Use the release validation record for what was actually run. Static observations, deterministic interpretations, inferred model explanations and presenter hypotheses must remain visibly distinct throughout the talk.
