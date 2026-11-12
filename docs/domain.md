# Checked domain model

These are implemented types. Structural validation makes mistaken associations harder to store; it does not prove the truth of an explanation.

## Identity rules

| Type | Identity and lifetime | What validates it |
| --- | --- | --- |
| `ArtifactId` | Lowercase SHA-256 of the original carrier bytes | Digest syntax in a DTO; an actual file read establishes a fresh match |
| `ImageId` | Lowercase SHA-256 of a complete image, independent of its address | Digest syntax and agreement with the owning map; fresh decode checks actual bytes |
| `RecordId` | Nonzero record number within one investigation; independent of graph layout | Unique IDs and valid references in `Investigation::validate` |
| `QuestionId` | A record that owns a question plan | Nonzero on deserialization; `resolve` checks the owning investigation and role |
| `ObservationId` | A byte, parsed firmware, disassembly, or completed REA evidence record | Nonzero on deserialization; `resolve` checks evidence role. A failed REA request is not evidence |
| `InsightId` | The human-facing Harness Insight attached to one record | Nonzero on deserialization; `resolve` finds that record's insight |
| `pi32::RunId` | Monotonically allocated disassembly number within one investigation | Nonzero, unique, scope-bound report; session generation rejects stale live results |
| OMP / REA request ID | Unique adapter-prefixed process/time identity, distinct from record and disassembly numbers | Transport correlation and unique persisted receipt IDs |
| REA `ev_…` ID | REA's own semantic commitment | Upstream validation and the supported offline profile; never replaced by a DIVULGE ID |

Record references are scoped to their owning investigation. They are not global IDs and cannot be moved to another session without resolving their meaning. Question and observation references serialize as the same integers used by older saves, so migration does not renumber evidence. Schema 7 adds explicit type-hypothesis records; schemas 1–6 migrate through the same checked domain validation.

## Coordinates and mappings

`FileOffset`, `PackageOffset`, `FlashOffset`, `ImageOffset` and `MappedAddress` are distinct newtypes. `FileSpan`, `PackageSpan`, `FlashSpan`, `ImageSpan` and `MappedSpan` bind a range to its carrier or image identity. Constructors and mapping methods check addition, bounds and owner identity. A deserialized span still requires validation. Empty descriptive file/package/flash spans are valid; decoded analysis requests and load resolution require bytes.

`SourceMap` implements the FWSC marker-removal mapping and can return several original-file fragments. `FlashMap` binds a flash member and package base. `LoadMap` holds declared image regions and resolves a complete image/address range without losing image identity. It reports `Resolution::Resolved`, `Ambiguous` with every candidate, or `Unresolved`; invalid arithmetic is a separate `Result::Err`. The V15 `ImageMap` uses these checked maps and additionally enforces pi32v2's 32-bit address space. Load declarations are static metadata, not observed runtime mappings.

The source mapping and cipher profile are supplied format knowledge. A length-preserving transformation preserves positions, not byte values. Source fragments therefore identify provenance, not equality between encrypted and decoded bytes.

## Verification is a runtime capability

`MatchedV15::check` borrows the exact bytes it hashes and accepts only the configured length and SHA-256. Its private state has no serde implementation. The decoder requires this proof before constructing `DecodedImage`; the image checks its decoded digest and length and owns the fresh bytes. `DecodedImage` also cannot be deserialized. Recorded artifact metadata may describe a previous match but cannot enable a tool run after reopen. Compile-fail doctests and the forged-metadata test enforce this distinction.

## Facts, views and hypotheses

Parsed fields live in `PackageHeader`, `PackageEntry`, `DirectoryEntry` and `Application`; their decoding and checksum provenance remain attached. `ScalarViews` is a chosen byte-order interpretation of available bytes in the inspector. Missing widths are `None`. `F32Bits` serializes the integer bits, retaining NaN payloads, infinities and negative zero without converting them into invalid JSON numbers.

`TypeHypothesis` is a separate record with a proposed target type, rationale, supporting `ObservationId`s, counter-evidence and a limitation. Its graph edges say **suggests** or **contradicts**. It cannot become an observation merely because it has valid citations. The native inspector can record a hypothesis from a selected evidence record and link back to its sources. A Rust host type, scalar view or plausible hypothesis does not establish the firmware's source language, packing or original type.

`Coverage::{Complete, Partial, Unavailable}` describes disassembly coverage independently of `ToolOutcome` (completed, failed, cancelled, timed out, and other execution outcomes). Resolution states describe destinations independently of both. A successful command can return incomplete coverage; partial captured output can still be an observation with limitations.

## Validation evidence

The test suite exercises cross-image and cross-artifact rejection, marker-boundary fragmentation, flash/package mapping, integer overflow, 32-bit address limits, ambiguous overlapping load regions, malformed saved DTOs, missing/incorrect-role evidence references, hypothesis round trips, and bit-exact float serialization. `cargo test --locked --doc` checks that serialized data cannot construct the two fresh-verification capabilities. `cargo run --locked --example validate_session -- <save>` validates an existing investigation offline and does not run a model or analysis tool.
