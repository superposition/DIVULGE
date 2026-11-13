# DIVULGE type catalogue

Generated from maintained descriptions in workshop/type-catalogue.json and checked against every top-level declaration in src, including macro-generated IDs/spans. Run python scripts/check_catalogue.py --write after intentional updates.

124 implemented named types. This index includes private runtime/presentation helpers. It does not claim that a host Rust type was recovered from target firmware. See [the 20-minute route](workshop.md) for the beginner subset and [Rust forms](rust-forms.md) for primitives, collections, borrowing and library types.

## domain

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [RecordId](../src/domain.rs#L10): Stable nonzero identity within one investigation; independent of graph position | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [FileOffset](../src/domain.rs#L14): u64 coordinate in an original file; range checked when used | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [MappedAddress](../src/domain.rs#L17): u64 declared CPU address; does not imply observed runtime memory | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Observation](../src/domain.rs#L20): Owned bytes, a file offset and source description | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Basis](../src/domain.rs#L27): Observed, Derived or Inferred provenance for an explanation | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [RecordData](../src/domain.rs#L44): Question, Probe, Observation, Firmware, Disassembly, Rea, Explanation, TypeHypothesis, Review or Unknown | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [HarnessInsight](../src/domain.rs#L114): Question, purpose, action, observation, explanation, uncertainty and next step | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [QuestionPlan](../src/domain.rs#L125): Why, steps, enables, answered_when and if_not fields for a question | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Record](../src/domain.rs#L134): Stable identity, kind, concise summary, insight, note and explicit question ownership | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Relation](../src/domain.rs#L149): Investigates, Produces, Supports, LeavesOpen, Suggests, Contradicts, Revises, Resolves, Steers or Reuses | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Edge](../src/domain.rs#L180): Directed typed relationship between existing record IDs | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Origin](../src/domain.rs#L187): WorkshopFixture or LocalArtifact; keeps synthetic material distinct | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Artifact](../src/domain.rs#L193): Recorded name, byte length, hash, preview and optional parsed firmware report | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Investigation](../src/domain.rs#L212): Canonical record store with edges and retained disassembly/model/REA receipts | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |
| [Attention](../src/domain.rs#L810): Window reading cursor, available frontier, follow/hold state and reviewed arrivals; not serialized evidence | Investigation builders and checked record import | Validated as part of an Investigation; a valid record does not establish semantic truth | Graph and Harness Insight | Mixing evidence, interpretation and unknowns |

## evidence

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [QuestionId](../src/evidence.rs#L40): Nonzero reference that resolves to a record with a question plan | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [ObservationId](../src/evidence.rs#L44): Nonzero reference that resolves to completed evidence | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [InsightId](../src/evidence.rs#L48): Nonzero reference to a record's human-facing insight | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [ByteOrder](../src/evidence.rs#L85): Little or Big: the selected interpretation of observed bytes | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [F32Bits](../src/evidence.rs#L94): u32 bits preserving NaN payloads and negative zero through JSON | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [ScalarViews](../src/evidence.rs#L104): Optional u8, u16, u32 and f32 views; unavailable widths are None | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |
| [TypeHypothesis](../src/evidence.rs#L135): Proposed type, rationale, supporting/counter-evidence IDs and limitations | Checked ID constructors, scalar-view projection or analyst review | ID role resolved against the owning case; interpretations remain interpretations | Evidence links, endian views and type-hypothesis form | Treating a representation or citation as proof |

## coordinates

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [ImageOffset](../src/coordinates.rs#L13): u64 offset within a decoded image | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |
| [ArtifactId](../src/coordinates.rs#L18): Validated lowercase SHA-256 identity of original carrier bytes | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |
| [ImageId](../src/coordinates.rs#L23): Validated lowercase SHA-256 identity of a complete decoded image | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |
| [ImageSpan](../src/coordinates.rs#L69): Nonempty half-open range tied to one ImageId | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |
| [ImageMap](../src/coordinates.rs#L79): Declared relationship between original artifact, package and decoded image | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |
| [Location](../src/coordinates.rs#L89): Resolved declared address plus original-file source fragments | Identity constructors and checked ImageMap methods | Digest syntax, owner identity, arithmetic and range checks; loaded DTOs are not fresh-byte capabilities | Declared address, image offset and original source fragments | Cross-image references and accidental coordinate conversion |

## mapping

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [FileSpan](../src/mapping.rs#L49): Original-file range qualified by ArtifactId | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [PackageSpan](../src/mapping.rs#L50): Logical package range qualified by ArtifactId | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [FlashSpan](../src/mapping.rs#L51): Flash range qualified by ImageId | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [MappedSpan](../src/mapping.rs#L52): Declared memory range qualified by ImageId | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [SourceMap](../src/mapping.rs#L57): FWSC marker-removal mapping; one logical range can have several file fragments | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [FlashMap](../src/mapping.rs#L102): Flash member location within its owning package | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [Resolution](../src/mapping.rs#L137): Generic result with resolved value, all ambiguous candidates or an unresolved reason | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [LoadRegion](../src/mapping.rs#L164): One declared image-to-address region | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |
| [LoadMap](../src/mapping.rs#L172): Collection of declared regions; resolves the whole requested span | Checked span constructors and source/load mapping methods | Resolution is Resolved, Ambiguous or Unresolved; invalid arithmetic returns Err separately | Source ranges and unresolved/ambiguous address explanations | Inventing a unique mapping or dropping fragmented provenance |

## fwsc

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [PackageOffset](../src/fwsc.rs#L14): u64 coordinate after removing FWSC markers | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [FlashOffset](../src/fwsc.rs#L17): u64 coordinate within the flash member | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [SourceSpan](../src/fwsc.rs#L20): Original file offset and length retained as provenance | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [CrcCheck](../src/fwsc.rs#L26): Declared and computed checksum values; equality is a scoped check | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [PackageHeader](../src/fwsc.rs#L38): Parsed package header fields and checksum comparisons | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [PackageEntry](../src/fwsc.rs#L50): Typed directory entry, logical offsets, lengths and raw checksum result | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [DirectoryEntry](../src/fwsc.rs#L65): Decoded application-area directory metadata | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [Application](../src/fwsc.rs#L79): Checked application directory selection, identity and bounded previews | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [FirmwareView](../src/fwsc.rs#L106): Package, Entry(index) or Application projection into a graph record | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [FirmwareReport](../src/fwsc.rs#L113): Parsed package and optional application report with unresolved checks | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |
| [MatchedV15](../src/fwsc.rs#L311): Runtime-only borrowed proof of exact source identity; not deserializable | Exact-V15 reader and supplied format profile | Identity, bounds and supported CRC checks; unsupported checks remain unresolved | Data view and package/application graph records | Disassembling container bytes as if they were application bytes |

## decoded_image

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [DecodedImage](../src/decoded_image.rs#L12): Runtime capability for the actual decoded bytes | Freshly checked V15 decoding | Owns current verified image bytes and map; cannot be deserialized | Enables Disassemble only after source verification | Using saved metadata to authorize a fresh tool read |

## pi32

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [RunId](../src/pi32.rs#L22): Nonzero disassembly identity within a case | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Target](../src/pi32.rs#L25): Direct declared address or unresolved destination text | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Flow](../src/pi32.rs#L31): Instruction transfer classification, separate from mnemonic text | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [DecodeKind](../src/pi32.rs#L41): Instruction versus retained undecoded data | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Instruction](../src/pi32.rs#L47): Address, image span, exact bytes, text and candidate transfer semantics | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Gap](../src/pi32.rs#L79): Selected bytes with no accepted decoded instruction | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Coverage](../src/pi32.rs#L85): Complete, Partial or Unavailable byte coverage | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [DisassemblyReport](../src/pi32.rs#L92): Scope, map, exact input, wrapper hash, capture, accepted instructions, gaps and issues | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Parsed](../src/pi32.rs#L251): Private intermediate parser output; not a fresh source capability | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [BasicBlock](../src/pi32.rs#L500): Conservative instruction interval with an explicit starting address | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [Destination](../src/pi32.rs#L507): Internal block, external address, unresolved destination or return | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [EdgeKind](../src/pi32.rs#L515): Typed jump, branch, call, return-site or unresolved control-flow relationship | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [FlowEdge](../src/pi32.rs#L538): Candidate relationship with source block and destination state | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |
| [ControlFlow](../src/pi32.rs#L545): Basic blocks and candidate edges derived from one accepted report | Validated parser over captured vendor output and selected input bytes | Instruction spans checked against input; destinations can be internal, external or unresolved | Code listing, control-flow graph and instruction inspector | Presenting candidate static flow as observed execution |

## tool_runner

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [ToolConfig](../src/tool_runner.rs#L28): Selected distribution and installed disassembler executable | Bounded pi32 adapter process wrapper | Completed, Failed, Cancelled, TimedOut or OutputLimit; partial output stays diagnostic/limited | Tool prerequisites, outcome and full receipt | Equating a successful process with complete analysis |
| [ToolOutcome](../src/tool_runner.rs#L44): Terminal transport outcome independent of decode coverage | Bounded pi32 adapter process wrapper | Completed, Failed, Cancelled, TimedOut or OutputLimit; partial output stays diagnostic/limited | Tool prerequisites, outcome and full receipt | Equating a successful process with complete analysis |
| [ToolCapture](../src/tool_runner.rs#L53): Tool identity/version/flags, raw output, exit status, elapsed time and limitations | Bounded pi32 adapter process wrapper | Completed, Failed, Cancelled, TimedOut or OutputLimit; partial output stays diagnostic/limited | Tool prerequisites, outcome and full receipt | Equating a successful process with complete analysis |
| [RunnerReport](../src/tool_runner.rs#L67): Private helper response decoded before constructing the capture | Bounded pi32 adapter process wrapper | Completed, Failed, Cancelled, TimedOut or OutputLimit; partial output stays diagnostic/limited | Tool prerequisites, outcome and full receipt | Equating a successful process with complete analysis |
| [OwnedDirectory](../src/tool_runner.rs#L368): Private cleanup guard for this run's temporary directory | Bounded pi32 adapter process wrapper | Completed, Failed, Cancelled, TimedOut or OutputLimit; partial output stays diagnostic/limited | Tool prerequisites, outcome and full receipt | Equating a successful process with complete analysis |

## omp

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Model](../src/omp.rs#L17): Discovered provider/model metadata and capabilities | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Catalogue](../src/omp.rs#L24): Installed SDK/CLI versions, configured default and available models | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Event](../src/omp.rs#L32): Versioned request-correlated model transport event | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Proposal](../src/omp.rs#L41): Question, why, method, enables, unknowns, cited observations and requested scope | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Operation](../src/omp.rs#L57): Disassemble, ReadFixture or Conclude allowlist | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Analysis](../src/omp.rs#L77): Model explanation, significance, evidence citations and limitations | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [AiRun](../src/omp.rs#L178): Exact context snapshot, ordered events, checked proposal and published record references | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |
| [Job](../src/omp.rs#L345): Runtime channel, cancellation and request identity; never reconstructed as a live job | Installed isolated OMP bridge and strict protocol decoder | Correlated ordered events and allowlisted operations; drafts remain provisional | Model catalogue, request receipt and inferred explanations | Treating an unchecked model proposal as an executed probe |

## rea

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Config](../src/rea.rs#L19): Installed runtime and package configuration | Installed REA adapter and supported canonical entropy-bundle profile | Complete canonical evidence or explicit incomplete/unsupported outcome | REA original-file evidence and retained ev_ identities | Replacing REA provenance or interpreting entropy as proof of encryption |
| [Request](../src/rea.rs#L65): Discovery, bounded original-file entropy request or supported bundle import | Installed REA adapter and supported canonical entropy-bundle profile | Complete canonical evidence or explicit incomplete/unsupported outcome | REA original-file evidence and retained ev_ identities | Replacing REA provenance or interpreting entropy as proof of encryption |
| [Receipt](../src/rea.rs#L107): Correlated adapter response retaining original canonical bundle | Installed REA adapter and supported canonical entropy-bundle profile | Complete canonical evidence or explicit incomplete/unsupported outcome | REA original-file evidence and retained ev_ identities | Replacing REA provenance or interpreting entropy as proof of encryption |
| [ReaRun](../src/rea.rs#L126): Receipt attached to one investigation record | Installed REA adapter and supported canonical entropy-bundle profile | Complete canonical evidence or explicit incomplete/unsupported outcome | REA original-file evidence and retained ev_ identities | Replacing REA provenance or interpreting entropy as proof of encryption |
| [Job](../src/rea.rs#L371): Runtime response channel and cancellation state | Installed REA adapter and supported canonical entropy-bundle profile | Complete canonical evidence or explicit incomplete/unsupported outcome | REA original-file evidence and retained ev_ identities | Replacing REA provenance or interpreting entropy as proof of encryption |

## execution

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [ProbeKey](../src/execution.rs#L9): Target, operation, offset, length and frozen profile identity for completed-request reuse | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Scope](../src/execution.rs#L18): Allowed target identity, operation and bounded byte range | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Mode](../src/execution.rs#L86): Running, Paused, Stopped, Completed or Failed | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Phase](../src/execution.rs#L94): NeedModel, AwaitingModel, ReadyProbe, AwaitingProbe or Done | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Controller](../src/execution.rs#L115): Sequential state, budget usage, request counts and status | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Event](../src/execution.rs#L303): Cycle and question identity plus one typed execution detail | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Detail](../src/execution.rs#L309): Started, pause/resume/stop, accepted/recorded probe, explanation and terminal accounting | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |
| [Audit](../src/execution.rs#L342): Replay-time execution consistency checker | Pure checked controller and history audit | Budget/scope/generation constraints apply to every transition | AI cycle controls and ordered execution history | Out-of-scope probes, unlimited loops and stale publication |

## engine

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Target](../src/engine.rs#L24): Fresh V15 image capability or compiled synthetic teaching bytes | Application runtime around the pure controller | One current model/probe worker; generation changes invalidate stale results | Live AI cycle and evidence arriving behind a held reader | Restoring executable workers from saved history |
| [Settings](../src/engine.rs#L69): Human-selected question, model, budget, range and objective | Application runtime around the pure controller | One current model/probe worker; generation changes invalidate stale results | Live AI cycle and evidence arriving behind a held reader | Restoring executable workers from saved history |
| [Output](../src/engine.rs#L77): Private typed disassembly or synthetic-byte worker result | Application runtime around the pure controller | One current model/probe worker; generation changes invalidate stale results | Live AI cycle and evidence arriving behind a held reader | Restoring executable workers from saved history |
| [ProbeJob](../src/engine.rs#L81): Private worker ID, generation, cancellation flag and receiver | Application runtime around the pure controller | One current model/probe worker; generation changes invalidate stale results | Live AI cycle and evidence arriving behind a held reader | Restoring executable workers from saved history |
| [Engine](../src/engine.rs#L94): Runtime jobs, completed-probe cache, draft and checked controller | Application runtime around the pure controller | One current model/probe worker; generation changes invalidate stale results | Live AI cycle and evidence arriving behind a held reader | Restoring executable workers from saved history |

## history

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Document](../src/history.rs#L12): Versioned baseline, provenance and ordered events | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [Event](../src/history.rs#L21): Sequence number, previous commitment, typed payload and current commitment | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [Payload](../src/history.rs#L28): Commit, JobStarted, JobEnded or Execution event | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [Change](../src/history.rs#L35): Record/edge/receipt addition, note revision or objective steering | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [Role](../src/history.rs#L55): Question, action, observation, explanation, hypothesis, open question or analyst decision classification | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [ReviewKind](../src/history.rs#L88): Steering, Correction or Resolution | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |
| [History](../src/history.rs#L189): Validated document and reconstructed investigation projection | Append-only capture and deterministic offline replay | Versioned event order, hash chain and validated resulting projection | Case history, checkpoints and analyst corrections | Silently rewriting findings or pretending imported snapshots have full chronology |

## memory

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [BranchMemory](../src/memory.rs#L7): Selected branch, projected records/relationships and omissions | Deterministic projection from retained branch records | Bounded records/text with explicit omitted IDs and excerpt markers | Branch memory / retained context in Harness Insight | Depending on transient chat or hiding omitted evidence |
| [MemoryRecord](../src/memory.rs#L17): Retained role, rationale, plan, note, evidence/counter-evidence and original review target | Deterministic projection from retained branch records | Bounded records/text with explicit omitted IDs and excerpt markers | Branch memory / retained context in Harness Insight | Depending on transient chat or hiding omitted evidence |

## storage

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Inspection](../src/storage.rs#L16): Artifact metadata plus optional runtime DecodedImage | Local file inspection | Bounded source read with recorded metadata and optional fresh image | Open artifact and Verify source | Confusing recorded identity with currently available source bytes |

## export

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Options](../src/export.rs#L7): Byte, analyst-text and original-receipt inclusion choices | Explicit portable review selection | Independent byte/text choices; original receipts require both | Export portable review | Accidentally exporting full local receipts or treating a review export as a replay case |

## app

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [FileCommand](../src/app.rs#L20): Private requested file action pending prerequisite/unsaved checks | Native event handlers and worker lifecycle | Window state and deliberate file commands; unsaved changes checked before replacement | Menus, panels and shared selection | Losing edits or publishing into a replacement case |
| [CodeJob](../src/app.rs#L30): Private running disassembly identity, receiver and starting reading state | Native event handlers and worker lifecycle | Window state and deliberate file commands; unsaved changes checked before replacement | Menus, panels and shared selection | Losing edits or publishing into a replacement case |
| [DivulgeApp](../src/app.rs#L39): Native application state tying records, workers, graph and inspector together | Native event handlers and worker lifecycle | Window state and deliberate file commands; unsaved changes checked before replacement | Menus, panels and shared selection | Losing edits or publishing into a replacement case |

## graph

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Layout](../src/graph.rs#L15): Narrative or Relationships | Presentation controls over canonical record IDs | Bounded layout and scope; positions/pins are presentation state | Graph cards, scopes and Navigate menu | Changing evidence relationships by moving a node |
| [Scope](../src/graph.rs#L20): Private All, Branch or Neighbors view | Presentation controls over canonical record IDs | Bounded layout and scope; positions/pins are presentation state | Graph cards, scopes and Navigate menu | Changing evidence relationships by moving a node |
| [Navigation](../src/graph.rs#L26): Private named camera, selection, pin and movement actions | Presentation controls over canonical record IDs | Bounded layout and scope; positions/pins are presentation state | Graph cards, scopes and Navigate menu | Changing evidence relationships by moving a node |
| [GraphView](../src/graph.rs#L44): ID-keyed positions, pins, scope anchor and camera bounds | Presentation controls over canonical record IDs | Bounded layout and scope; positions/pins are presentation state | Graph cards, scopes and Navigate menu | Changing evidence relationships by moving a node |
| [GraphResponse](../src/graph.rs#L55): Selection or movement intent returned to the application | Presentation controls over canonical record IDs | Bounded layout and scope; positions/pins are presentation state | Graph cards, scopes and Navigate menu | Changing evidence relationships by moving a node |

## insight

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Inspector](../src/insight.rs#L16): Inspector selection, scalar interpretation and hypothesis form state | Terminal inspector widgets | Record-local reading/editing state; findings remain canonical | Harness Insight | Confusing a transient widget state with a saved observation |
| [Response](../src/insight.rs#L29): Selected/created record and note-change intent | Terminal inspector widgets | Record-local reading/editing state; findings remain canonical | Harness Insight | Confusing a transient widget state with a saved observation |

## data_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [ByteView](../src/data_view.rs#L269): Borrowed bytes with source, length and decoding provenance | Borrowed projection of existing evidence | Read-only references valid for the owner's lifetime | Data tables and byte views | Copying a presentation projection into a second evidence store |

## code_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Action](../src/code_view.rs#L15): Requested run/cancel/source verification/selection intent | Native Code view controls | Selected run/instruction, scope input and optional actions | Instruction listing and control flow | Launching a tool merely by viewing saved instructions |
| [CodeView](../src/code_view.rs#L22): Code presentation, scope fields, conservative flow and camera | Native Code view controls | Selected run/instruction, scope input and optional actions | Instruction listing and control flow | Launching a tool merely by viewing saved instructions |

## ai_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Action](../src/ai_view.rs#L21): Requested AI dialog effect | Model dialog and request controls | Discovered catalogue and selected recorded/running request | OMP models and next step | Changing the global OMP default or treating drafts as final |
| [AiView](../src/ai_view.rs#L10): Model picker, request controls and retained receipts | Model dialog and request controls | Discovered catalogue and selected recorded/running request | OMP models and next step | Changing the global OMP default or treating drafts as final |

## engine_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [Action](../src/engine_view.rs#L11): Typed cycle-control intent | AI cycle configuration and controls | Start, pause, resume, stop and model-selection intent | AI cycle dialog and execution status strip | Embedding unchecked execution inside widget rendering |
| [EngineView](../src/engine_view.rs#L20): Question, scope, model and budget form | AI cycle configuration and controls | Start, pause, resume, stop and model-selection intent | AI cycle dialog and execution status strip | Embedding unchecked execution inside widget rendering |

## rea_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [ReaView](../src/rea_view.rs#L7): REA controls, job polling and receipt display | REA operation dialog | Configured prerequisites, bounded input and retained run selection | REA original-file evidence | Hiding adapter availability or incomplete output |

## history_view

| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |
| --- | --- | --- | --- | --- |
| [HistoryView](../src/history_view.rs#L8): Checkpoint, review and portable-export form state | History and export dialogs | Read-only checkpoints plus explicit append/review/export actions | Case history and review | Editing past records through a replay preview |

## Implemented versus future work

All names above are implemented in this revision. More REA operations, verified non-Windows desktop builds, general firmware format support, recovered runtime behavior and persistent graph positions are not supplied by these types. No planned architecture is listed as a working capability.
