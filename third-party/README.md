# Upstream notice snapshots

Some crates omit their workspace-level license files from the published Cargo package. These files preserve notices from the exact upstream commit recorded in each crate's `.cargo_vcs_info.json`.

`notice-sources.json` records crate versions, source URLs and SHA-256 values. The release packager checks each revision and digest and links the files from `licenses/dependencies.json`. It fails if a dependency has neither packaged notices nor a reviewed fallback. This directory is already part of the committed source archive; release packaging needs no extra notice downloads.

The hexf repository at its recorded revision contains no standalone license file. Its original `parse/Cargo.toml` is retained as the author's explicit CC0-1.0 declaration, with the original authorship and repository fields. Other snapshots preserve upstream license texts unmodified, including AccessKit's Chromium notice and egui's font notice. These notices do not change DIVULGE's own MIT license.
