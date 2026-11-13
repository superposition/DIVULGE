# Native desktop targets

DIVULGE keeps one Rust/egui application and the same saved-case format across platforms. The prebuilt app reads synthetic or saved investigations without a backend service, compiler, analysis tool or model connection.

| Package | CPU / system | Build recipe | Runtime qualification |
| --- | --- | --- | --- |
| `windows-x86_64` | Intel/AMD 64-bit Windows | Native MSVC build, `divulge.exe` | Existing interactive Windows validation; package hashes checked for each build. |
| `macos-arm64` | Apple Silicon, including M3 | Native Apple Silicon build, `DIVULGE.app` | Rust tests, Mach-O CPU/type, code signature, package hashes, synthetic-case launch and real desktop screenshot in the packaging workflow. Inspect the accompanying smoke record/image for the particular build. |
| `macos-x86_64` | Intel Mac | Recipe supports a matching native Intel host | Not included in the current two-host workflow; unverified. |
| `linux-x86_64` / `linux-arm64` | Linux desktop | Recipe supports a matching native Linux host | Unverified; graphics/window-system dependencies must be supplied. |

These are application targets. They do not imply support for every firmware ISA or recover source code from arbitrary machine code. V15 decoding, vendor pi32v2 disassembly and REA integrations retain their existing profiles and evidence limits.

## Take the app to the M3

Download/extract the `DIVULGE-macos-arm64-release` build artifact. Its ZIP contains `DIVULGE.app`, the matching source, workshop fixtures/PDF, dependency notices, `release.json` and `SHA256SUMS.txt`. Copy the app to Applications or open it in the extracted directory. Keep the workshop folder wherever convenient and open its synthetic case through File > Open investigation.

The app is ad-hoc signed and is not Developer ID signed/notarized. macOS may require the user to approve the downloaded app through Privacy & Security. Credentials and original firmware are excluded. To move a private investigation, copy your reviewed saved case separately or use File > Export portable review. Reopening a case does not start its retained tools or model requests.

Open, Save and Find use Command on macOS and Ctrl on Windows/Linux. The Mac bundle preserves executable permission in its ZIP. `release.json` identifies the exact source commit and executable digest; `native-smoke.json` verifies package bytes and states whether the UI launch was performed. A screenshot proves the observed window, not complete interaction or live-tool correctness.

## Build on the destination Mac

Install Rust 1.92+ and Apple's Command Line Tools, then in the source directory:

```sh
cargo run --locked
```

For a distributable `.app`, use Python 3.11+ in a virtual environment with `workshop/pdf-requirements.txt`. Commit reviewed source first so the package can include an exact source archive:

```sh
python scripts/package_release.py --target aarch64-apple-darwin
python scripts/verify_native.py output/release/*macos-arm64.zip --smoke
```

The release recipe requires a matching native host, remaps private source/cache paths, builds and checks the workbook, collects exact dependency notices, constructs/signs the `.app`, and hashes its contents. It does not claim a Windows cross-build is a qualified Mac binary. The hosted workflow uses GitHub's documented [Apple Silicon `macos-15` runner](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).

## Optional live operations

The native UI, graph, evidence inspector, save/reopen and offline workshop are built into the app. OMP proposals separately require Bun plus the validated OMP SDK 18.14.2 and credentials configured on that computer; `DIVULGE_BUN` and `DIVULGE_OMP_SDK` select existing installations. REA needs a compatible Node executable and installed package, selectable in the UI or through `DIVULGE_REA_NODE` / `DIVULGE_REA_PACKAGE`. Finder may provide a smaller PATH than Terminal; explicit paths avoid relying on shell startup files.

Vendor pi32v2 disassembly additionally needs a compatible disassembler executable on the analysis host. A Linux vendor binary is not automatically usable on an M3 Mac. Missing tools remain unavailable; portable recorded evidence can still be inspected. Mac packaging and the synthetic smoke do not qualify live vendor, REA, cloud, or hardware behavior on the Mac.
