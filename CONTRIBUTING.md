# Contributing

Start with the [roadmap](docs/roadmap.md) and [architecture](docs/architecture.md). Keep implementation changes small enough to demonstrate with an actual native interaction or a meaningful domain test.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

Use synthetic fixtures for distributable tests. Do not commit firmware, vendor tools, credentials, local model configuration, or unrelated personal files. Preserve raw evidence and unknowns; never turn missing data into a zero, false, empty result, or resolved destination without justification.

The application is Rust/egui. Use egui primitives before introducing custom controls. Domain logic stays independent of presentation. A new feature that affects a workshop type should update the corresponding lesson and runnable example.
