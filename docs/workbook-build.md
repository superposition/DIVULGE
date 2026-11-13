# Building the companion workbook

The 18-page workbook reads the actual six example files and `workshop/lessons.json` rather than a second copy of their code. Source links use a full Git revision, and every page states application version 0.1.0 and the short source revision. The generated PDF belongs in `output/pdf/`, outside the source tree's tracked files.

Use Python 3.11 or newer for document production. The Rust teaching verifier separately supports Python 3.8+. ReportLab 4.4.3 uses a hashing API unavailable in Python 3.8; do not run the PDF builder in that older runtime.

```powershell
python3.11 -m venv target/pdf-env
& target/pdf-env/Scripts/python.exe -m pip install -r workshop/pdf-requirements.txt
python scripts/check_workshop.py
python scripts/check_catalogue.py
& target/pdf-env/Scripts/python.exe scripts/build_workbook.py
& target/pdf-env/Scripts/python.exe scripts/check_workbook.py
New-Item -ItemType Directory -Force tmp/pdfs
pdftoppm -r 110 -png output/pdf/DIVULGE-workshop.pdf tmp/pdfs/workbook
```

Poppler provides `pdftoppm`, `pdfinfo` and `pdffonts`. A POSIX shell uses the virtual environment's `bin/python`. `--source-ref <full-commit>` pins the builder to a release/example revision, and `--output <path>` chooses an alternate PDF destination. For a release, generate from a clean checkout of that revision; the structural checker verifies that every linked source exists in it.

Inspect all 18 PNG pages, including code line wraps, screenshot crops, worksheet lines, table cells and page footers. The builder rejects content below its page margin. The checker requires 18 pages, selectable text, embedded fonts, numbered footers, all six lesson stages and source links to existing committed files. Neither check substitutes for visual inspection. Rebuilding with the same source revision, dependency versions and assets is byte-identical; PDF metadata uses ReportLab's invariant mode.

## Native screenshot provenance

`workshop/assets/synthetic-evidence.jpg` and `synthetic-explanation.jpg` were captured through Windows Computer Use from the actual 1280x720 client, with the generated seven-record synthetic replay open. The first shows the neighbor scope around observation 3; the second shows explanation 4's question, action, evidence, limitation and next probe. These are recorded fixture outputs, not a live model transcript.

`v15-code-large.jpg` was captured from the actual 1920x1080 client after opening the local saved V15 investigation offline. The PDF clips a small portion of its instruction inspector. It shows the two-byte static instruction at declared address 0x02000120 and the distinct source/image coordinates. It does not contain the firmware file or a vendor executable. No model was contacted for the real case.

The original JPEG captures are retained. The PDF clips them in page space; no UI content is synthesized, repainted or substituted. Body/code text and tables remain selectable vector content. ReportLab's bundled Bitstream Vera fonts are embedded; its font distribution includes the Bitstream license. The screenshots are the only dark areas, limiting ink use across the rest of the workbook.

## Review status

The 18 rendered pages were inspected locally with Poppler at 110 dpi. The review covered every page and rechecked changed lesson-writing areas, native screenshot crops, source-type names and the document-runtime instructions. No clipping, overlapping text or missing glyphs remained. Structural checks confirmed all fonts embedded, selectable text, 18 numbered pages and 21 source links. Two builds at example revision `c03393e8d1b2f8552f8c805c1e331e684da0cf52` produced the same PDF bytes. The release rebuild will use its final source revision; packaging remains ticket #14.
