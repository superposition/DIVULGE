# Investigation history and review exports

Local saves use the `divulge-event-ledger` envelope, version 2, with a schema 8 investigation baseline and ordered events. Version 1 ledgers remain readable with unchanged baseline and event digests; version 2 adds checked execution lifecycle events. Plain schema 1–7 saves migrate to a validated starting snapshot. The migration labels the missing earlier chronology; it does not invent job timestamps or edit history. Schema 8 adds explicit analyst review records.

Each event has a sequence number, a previous-content digest and a digest of its payload. Transactions append records, relationships and original adapter receipts, or retain both versions of a note/objective edit. A completed transaction must reconstruct a valid investigation. Existing findings, receipts and relationships cannot be silently rewritten or removed. Hash commitments detect accidental alteration and order changes; they are not signatures or proof that an author is trustworthy.

Job starts and endings are separate events, correlated with the live request identity. Saving during work retains its start. If the saved file has no ending, reopening treats it as unfinished historical work and never resumes it. Original model, REA and disassembly receipts remain in the local history with their existing validation and limitations. Notes are captured when editing loses focus, another durable action occurs, or the case is saved, rather than recording each keystroke. As with other document editors, unsaved work is not crash-durable.

**View > Case history and review** shows the ordered events. Selecting a checkpoint reconstructs that prefix into a read-only view, including notes at that time. Replay only reduces data: it has no adapter, process or model callback. The usual graph replay also remains an offline record-reveal view; it is distinct from selecting an event checkpoint.

The same window appends steering, corrections and scoped resolutions of the selected record. Each review states its rationale and limits; resolutions require supporting observation references. The original record and any unresolved question remain visible. A resolution is an analyst decision within its stated scope, not proof that every uncertainty disappeared. The graph and inspector link the review to the original finding and its evidence.

## Save and recovery

Loading is bounded to 16 MiB; history is bounded to 4,096 events and the existing record/run limits. Saves validate the complete replay, write and sync a unique sibling file, then replace the primary. A validated previous primary is retained as `<case filename>.bak`. An interrupted temporary write cannot become the primary merely because its file exists. File and directory durability still depend on the operating system and storage device; this is not a transactional database.

If the primary is damaged, loading reports the available backup without applying it silently. **File > Recover previous save** asks for the original case path, reads its `.bak`, and opens it for inspection with Save As required. The damaged primary remains available for diagnosis. Both primary and backup may contain private local paths, model context and byte previews.

Reopened metadata does not construct `MatchedV15` or `DecodedImage`. A fresh read/hash/decode is still required before live disassembly. History never serializes these capabilities.

## Portable exports

**File > Export portable review** creates a separate JSON projection, not a replay case or a canonical REA bundle. The exact export preview is available before saving. Defaults include IDs, coordinate maps, typed relationships and numeric evidence, while omitting byte contents, descriptions/notes, original paths and model transcripts. Options separately enable retained bytes and descriptive/analyst text. Original receipts require both options and explicitly include machine details, prompts, transcripts and bytes. No credential store is read or exported; free text and original tool output still require review before sharing.

A redacted REA projection retains the original `ev_` reference and marks itself noncanonical. Its changed contents do not claim to have the original digest. Preserve the local case when exact original evidence and replay are required. Exporting does not change the investigation or its save path.

## Verification

Tests exercise deterministic replay, retained note revisions, objective steering, immutable findings, corrections and evidence-backed resolutions, dangling references, altered/reordered events, unsupported versions, unfinished jobs, byte/text export controls, bounded input, interrupted sibling writes, backup recovery and refusal to overwrite a damaged primary. `cargo run --locked --example validate_session -- <case>` reconstructs and compares the final projection offline.
