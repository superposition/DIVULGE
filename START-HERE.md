# DIVULGE 0.1.0 for Windows

Unzip the whole release into a directory you control, then run `divulge.exe`. It opens a synthetic teaching investigation. Use File > Open investigation to open `workshop/fixtures/synthetic-workshop.json`; reopening runs no model or tool.

The companion workbook is `workshop/DIVULGE-workshop.pdf`. Start with its preparation page, then follow [the twenty-minute Rust route](docs/workshop.md). [The developer talk route](docs/talk.md) extends the session to 60, 75 or 90 minutes.

The release contains the matching source, six runnable examples, the intentional compiler error and answer, a saved synthetic replay, its portable review, and dependency notices. `release.json` gives the source commit, build identity and hashes. `SHA256SUMS.txt` records package-file hashes.

Windows with an OpenGL-capable desktop is the verified native target. Linux and macOS remain unverified. The application is unsigned; the release does not install it or change system settings. Rust and the Microsoft linker/Windows SDK are needed only to compile the source and examples, not to read the supplied replay in the prebuilt app.

Optional live operations need separately installed tools. See [README](README.md) for exact REA, OMP/Bun and vendor pi32v2 prerequisites. Obtain the reference firmware separately. No firmware, vendor executable or model credentials are included. Keep a real local case private unless you have reviewed its bytes, paths, notes and original receipts; use the scoped portable export to share selected content.

Keep one workbench window during the talk. Center selected restores readable graph labels; Neighbors narrows the graph. Use the right-hand terminal pane to inspect the question, observed bytes, explanation, uncertainty and next probe. Presenter hold protects reading; Pause and Stop control AI execution.
