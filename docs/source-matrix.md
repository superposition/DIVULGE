# Read source-byte trends in 3D

Open an artifact, then choose **3D matrix** beside Graph, Data and Code. The view counts every byte of the same bounded read used to hash the original file. Processing happens in the artifact worker; rotating the plot does not reread the file or run a tool/model.

| Display | Measurement |
| --- | --- |
| X | Byte value, `00` through `FF`. |
| Y | Up to 64 consecutive, non-overlapping regions covering the source span. |
| Height | The byte's frequency **within its region**. The displayed maximum labels the height scale; the final shorter region uses its own length. |
| Color | Shannon entropy of that region, from 0 to 8 bits per byte. |

Drag to rotate, or use Rotate/Tilt sliders. Zoom and Height adjust presentation. Click a visible point to inspect its region and byte value; the numeric **Inspect region** and **byte** controls provide the same selection without picking a small point. **Reset camera** restores the initial view. The region inspector reports the exact start/end offsets, length, count, percentage, entropy and first 32 bytes. **Copy selected range** copies those measurements.

Offsets are original **file** offsets, with the end excluded. They are not decoded app.bin offsets, flash addresses, CPU addresses or recovered source-code locations. The matrix has its own region inspector; returning to Graph restores the existing Harness Insight selection.

## What patterns can suggest

A tall ridge at `00` or `FF` can reveal repeated padding. Changes between regions can help locate a boundary worth examining. A broad distribution with high entropy may come from compressed, encrypted or otherwise varied data; the matrix cannot decide which. Repeating byte distributions can suggest similar regions, but equal histograms do not establish equal byte order, equal contents or equal program behavior.

All byte values are counted exactly; zero-count cells are omitted from drawing. At most 64 × 256 nonzero points are rendered, so large files are summarized by larger regions rather than by silently sampling bytes. This can hide small structures inside a region. The matrix does not recover functions, instruction boundaries or types, and it does not promote a visual pattern into an evidence record.

## Fresh source and saved cases

A newly opened or successfully reverified artifact produces a whole-file matrix, up to the existing 128 MiB limit. The selected file's SHA-256 is displayed. Source and decoded-image identity checks still apply before attaching bytes to a saved case.

Reopening a case does not read the original file automatically. Only its explicitly labelled recorded preview is available in the matrix until **File > Verify source** verifies the original again. Synthetic teaching cases use their recorded observation bytes and retain their original offsets. Empty inputs show an unavailable plot rather than fabricated zero measurements.

Histogram/entropy tests cover uniform distributions, padding versus varied regions, complete uneven partitions, base offsets, empty input and overflow. Projection tests cover camera bounds. These are statistical and presentation checks, not execution or hardware verification.
