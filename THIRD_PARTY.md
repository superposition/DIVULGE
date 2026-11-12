# Format implementation reference

The read-only Rust decoder in `src/fwsc.rs` adapts the format and cipher
descriptions implemented by M-VAVE's MIT-licensed
[`sdk/Package.cs`](https://github.com/m-vave/fm1_open/blob/main/sdk/Package.cs),
which credits [jl-misctools](https://github.com/kagaimiq/jl-misctools/tree/0a5b12db0ef38f3042acffbe2452730a37fd2405).
DIVULGE adds checked readers, separate coordinate types, source mapping,
recorded results, and a native inspection UI. It does not include the writer,
firmware, device updater, or vendor disassembler.

Copyright (c) 2023 Andrey Grigoryev (kagaimiq), 2026 M-VAVE.

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
