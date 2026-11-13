# Durable harness memory and human review

The investigation file is the memory. Each question retains its purpose and plan; probes retain actions; observations retain their source; explanations retain evidence and limitations. Hypotheses preserve supporting and counter-evidence. Analyst corrections, steering and scoped resolutions append records linked to their originals. Note revisions and ordered decisions remain in the ledger. These are human-readable explanations and recorded decisions, not hidden model reasoning.

## What OMP receives

`memory::branch` builds a deterministic projection from canonical records at the requested frontier. It follows explicit question ownership, recorded AI request ancestry and analyst review targets. It prioritizes the selected record, its question, decisions and hypotheses, then recent branch records and their evidence dependencies. A later observation on another branch cannot displace the selected branch's cited evidence merely because it is newer.

The snapshot labels observations, supported/inferred interpretations, hypotheses, steering, corrections and scoped resolutions separately. It includes question plans, purpose, action, observation, explanation, uncertainty, next probes, supporting/counter-evidence IDs and original review targets. Original findings are retained when a correction is added. Typed relationships connect the selected memory records.

Memory is bounded to 24 records and a 30,000-byte record budget. Text excerpts carry an explicit marker; omitted record IDs and shortened-record IDs are listed. OMP also receives up to 12 observation summaries, target identities and at most two instruction excerpts. Branch citations take priority; recent observations fill spare slots. Optional instruction rows are reduced when approaching the 64 KiB transport limit. The context is deliberately incomplete when these limits apply and says so. Only supplied `record_ids` may be used as observation citations.

This context is rebuilt from durable records for each request. A correction recorded while one request is running enters subsequent requests; it cannot retroactively change the earlier snapshot. Every model receipt retains the exact snapshot it received. The terminal inspector's **Branch memory / retained context** exposes the same projection for the current reading checkpoint.

## Reading while work continues

The execution frontier controls what evidence is available. The attention cursor controls what the person is reading. A separate reading checkpoint prevents newly arriving child links from inserting text above the reader's current scroll position. Each record's terminal scroll position is keyed by its stable record ID.

- **Presenter hold** keeps selection, camera and reading checkpoint fixed while AI and tools continue. This is different from **Pause AI**, which prevents the next execution step.
- **Review next** selects the earliest new record not yet reviewed and centers it. Viewing an older record again does not make already-reviewed arrivals new again.
- **Follow latest** releases presenter hold, acknowledges the available records, selects the latest and follows future arrivals. Panning, choosing another record, scrolling the inspector or beginning an edit detaches follow mode.

The new-record count tracks arrivals in the current window. Existing records are acknowledged on opening; attention state is presentation state and does not invent a persisted history of what a person has read. Saved event history remains the durable account of what happened.

Live records append without rearranging existing nodes. Explicit layout changes and focus commands may move the view. Minor OMP transport events and draft fragments stay inside receipts rather than becoming graph nodes. A probe exposes its request's events in a collapsed detail section, while its concise graph summary and verbose inspector use the same canonical action record. A proposal links to its actual probe when one exists at the reading checkpoint.

## Checks

Tests cover replay-equivalent memory, retained corrections and counter-evidence, older branch evidence amid unrelated arrivals, marked context omissions, and history-preserving excerpts. The execution integration test adds a correction while a model request is pending, advances the real fixture worker, verifies the next snapshot includes the correction, and holds the reader while the concluding explanation arrives. Attention and graph tests separately check unread counts, follow/hold transitions, frozen reading checkpoints, existing positions and camera bounds.
