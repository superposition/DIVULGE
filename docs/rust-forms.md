# Rust forms used in DIVULGE

The [named type catalogue](type-catalogue.md) covers application declarations. This page explains the primitive, standard-library and dependency forms those declarations use. These are host implementation choices, not recovered FM-1 source types. Lessons 1-6 introduce only the first few; the rest support later exploration.

| Form | Meaning and use | Where to look / mistake avoided |
| --- | --- | --- |
| `u8` | Unsigned eight-bit byte, 0-255 | Hex evidence; a byte alone has no semantic field type |
| `u16`, `u32`, `u64` | Unsigned fixed-width integers | Checksums, declared fields, lengths and wrapped coordinates; width does not imply byte order |
| `usize` | Platform-sized collection index | Selected row/byte indices; checked conversion from file coordinates avoids truncation |
| `i8`, `i32`, `i128` | Signed integers used for scalar views, exit codes and widened displacement arithmetic | Negative displacements remain distinct from unsigned addresses |
| `u128` | Wide unsigned timestamp arithmetic | Request identity suffixes; not a target firmware field |
| `f32` | Floating-point UI geometry or chosen scalar interpretation | Graph positions and `F32Bits::value`; JSON retains bits when float payloads matter |
| `bool` | True/false choice | Hold, following, cancellation and validation checks; uncertainty needs richer states than a boolean |
| `char` | Unicode scalar value | Text clipping/parsing; a character is not necessarily one UTF-8 byte |
| `[T; N]` | Fixed-size array; N is part of its type | Two teaching bytes, fixed-size chunks and tool flags |
| `&[T]`, `&str` | Borrowed slices of elements or UTF-8 text | Read bytes without copying or taking ownership |
| `Vec<T>`, `String` | Owned variable-length elements/text | Retained bytes, records, explanations and notes |
| `Option<T>` | `Some(value)` or `None` | Missing address, optional model analysis or unavailable scalar width; absence is not zero |
| `Result<T, E>` | `Ok(value)` or `Err(reason)` | Checked reads, mappings, parsing and storage; failure is not an empty successful observation |
| `()` and `Result<(), E>` | Unit value; success without an additional result | Validation and file writes |
| `(A, B)` | Tuple grouping different values | Scope pairs, indices and worker starting selections |
| `&T`, `&mut T` | Shared read borrow or exclusive mutable borrow | Domain reads versus editing a note; owners remain responsible for lifetime |
| `'a` | Named relationship between borrowed lifetimes | `ByteView<'a>` and `MatchedV15<'a>` cannot outlive their borrowed bytes |
| `HashMap<K, V>`, `HashSet<T>` | Keyed lookup and uniqueness | Stable graph positions, pins and reviewed records; hash iteration is not chronology |
| `BTreeMap<K, V>`, `BTreeSet<T>` | Ordered keys and unique sorted values | Job tracking and deterministic address/block construction |
| `Box<T>` | Own a heap-allocated value through a fixed-size pointer | Large record/report variants in event history |
| `Arc<T>` | Share ownership across threads with atomic reference counting | Fresh immutable decoded image shared with workers; not permission to mutate it freely |
| `Sender<T>`, `Receiver<T>` | Typed messages between worker and UI threads | Results arrive without blocking rendering; generation checks reject stale work |
| `AtomicBool`, `AtomicU64`, `Ordering` | Atomic flags/counters and memory-order choices | Cancellation and unique temporary-directory sequences; not persistent evidence |
| `Duration`, `Instant`, `SystemTime` | Time span, monotonic elapsed clock and wall clock | Deadlines, replay pacing and request IDs; wall-clock order is not the event ledger |
| `Path`, `PathBuf` | Borrowed and owned filesystem paths | Local source selection; portable exports are a separate privacy-aware projection |
| `Command`, `Child`, `ExitStatus`, `Stdio` | Owned subprocess configuration and outcome | Bounded installed-tool adapters; target firmware is never executed |
| `File`, `Read`, `Write`, buffered IO traits | Files and capabilities for consuming/producing bytes | Bounded reads and atomic replacement; a filename is not identity |
| `serde_json::Value` | Dynamically shaped JSON value | Adapter protocol and bounded context; validate it before building trusted domain records |
| `Serialize`, `Deserialize` | Traits for converting data to/from a wire format | Saved DTOs; successful decoding does not grant fresh-source capability |
| `Clone`, `Copy`, `Debug`, `Default`, `Eq`, `Hash` | Derived behavior selected per type | Cheap ID copies, explicit owned copies, comparisons and keyed collections |
| `impl`, generics and trait bounds | Methods and reusable behavior constrained by capabilities | `Resolution<T>` and const-size byte windows; no class inheritance is required |
| Closures and `FnMut` | Anonymous functions capturing context | egui callbacks and thread work; captures obey ownership rules |
| `?`, `match`, `if let` | Control-flow syntax rather than data types | Propagate an error or handle alternatives explicitly |
| `egui::Ui`, `Context`, `Response` | Immediate-mode layout, shared UI context and interaction result | Panels return actions; widget rendering is not evidence generation |
| `Pos2`, `Vec2`, `Rect` | Presentation point, displacement and rectangle | Camera/node geometry; never substitute these for source addresses |
| `Color32`, `FontId`, `RichText`, `Stroke` | Presentation styling | Card kinds also have text labels, so color is not the only meaning |
| `Scene`, `Panel`, `ScrollArea`, widgets | Reused egui layout/interaction primitives | Pan/zoom, resizable panes and readable inspectors |

For APIs, use the installed Rust and egui documentation matching `Cargo.lock`. The workbook links to the actual repository declarations instead of promising that every possible dependency type is taught in twenty minutes.
