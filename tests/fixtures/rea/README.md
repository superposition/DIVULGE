# Synthetic REA entropy fixtures

These fixtures contain no firmware. The generator asks installed REA 6.8.0 to inspect 512 synthetic bytes: 256 zeros followed by every value from 0 to 255. A second request selects a zero-length window. It replaces temporary paths with the explicit synthetic `C:/divulge-fixtures/synthetic.bin` path and asks REA's identity implementation to compute new fixture IDs. They are teaching fixtures, not original receipts from a user's artifact.

Run `node tests/fixtures/rea/generate.mjs /path/to/rea-agents` to regenerate. The checked-in JSON supports offline Rust validation without Node or REA. It tests exact original-record preservation, the semantic commitment and manifest, empty observations, altered data, and missing coverage. Live adapter acceptance is documented in `docs/native-validation.md` and its machine-specific receipts stay outside Git.
