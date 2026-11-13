# DIVULGE 0.1.0 desktop package

Unzip the whole release into a directory you control. On Windows, run `divulge.exe`. On an Apple Silicon Mac (including M3), open `DIVULGE.app`; you can copy that app to Applications. It opens the same native Rust/egui investigation UI. Use File > Open investigation to open `workshop/fixtures/synthetic-workshop.json`; reopening runs no model or tool.

The companion workbook is `workshop/DIVULGE-workshop.pdf`. Follow [the Rust examples and explanations](docs/workshop.md), then continue into the firmware and AI sections of [the talk](docs/talk.md).

The release contains the matching source, six runnable examples, the intentional compiler error and answer, a saved synthetic replay, its portable review, and dependency notices. `release.json` gives the source commit, build identity and hashes. `SHA256SUMS.txt` records package-file hashes.

The Windows package requires an OpenGL-capable desktop. The Apple Silicon app requires macOS 11 or newer and uses the system window and graphics frameworks. Neither prebuilt app needs Rust, Python, Bun or a separate service to read the supplied replay. Build and native-window evidence travels alongside the package as `native-smoke.json` and, for the Mac smoke, `native-mac.png`.

The Mac app has a local ad-hoc signature; it is not Apple Developer ID signed or notarized. If macOS blocks the downloaded app, review it and use the system's Privacy & Security > Open Anyway control. The Windows executable is unsigned. Neither package installs tools or changes system settings. See [native targets](docs/native-targets.md) for building from source and optional live-tool prerequisites.

Optional live operations need separately installed tools. See [README](README.md) for exact REA, OMP/Bun and vendor pi32v2 prerequisites. Obtain the reference firmware separately. No firmware, vendor executable or model credentials are included. Keep a real local case private unless you have reviewed its bytes, paths, notes and original receipts; use the scoped portable export to share selected content.

Keep one workbench window during the talk. Center selected restores readable graph labels; Neighbors narrows the graph. Use the right-hand terminal pane to inspect the question, observed bytes, explanation, uncertainty and next probe. Presenter hold protects reading; Pause and Stop control AI execution.
