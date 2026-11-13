#!/usr/bin/env python3
"""Create the PDF walkthrough of the actual tail.bin Harness Insight panel.

Input values come from examples/workshop_tail_bin.rs, run on the local V15 file.
The original user capture is placed in PDF space without altering its pixels.
"""
import argparse
import html
import json
import pathlib
import subprocess
import textwrap

from reportlab.lib.utils import ImageReader
from build_workbook import Book, ROOT, WIDTH, HEIGHT, MARGIN, CONTENT, RULE, MUTED, TEAL, INK, PALE


def block(text):
    return textwrap.dedent(text).strip("\n")


def section(name):
    source = (ROOT / "examples/workshop_tail_bin.rs").read_text(encoding="utf-8")
    return textwrap.dedent(source.split("// section: " + name + "\n", 1)[1].split("// endsection", 1)[0]).rstrip()


def declaration(path, start):
    source = (ROOT / path).read_text(encoding="utf-8")
    begin = source.index(start)
    level = 0
    opened = False
    for index in range(begin, len(source)):
        if source[index] == "{":
            opened = True
            level += 1
        elif source[index] == "}":
            level -= 1
            if opened and level == 0:
                return source[begin:index + 1]
    raise ValueError(start)


class TailBook(Book):
    total = 12

    def __init__(self, output, revision):
        super().__init__(output, revision)
        self.c.setTitle("DIVULGE - Reading tail.bin")
        self.c.setSubject("The FM-1 V15 Harness Insight panel explained through Rust, bytes and addresses")

    def start(self, title, label):
        if self.page:
            self.finish_page()
            self.c.showPage()
        self.page += 1
        self.c.bookmarkPage("page-" + str(self.page))
        self.c.addOutlineEntry(title, "page-" + str(self.page), 0)
        self.c.setFillColor(TEAL)
        self.c.setFont("VeraBold", 9)
        # Rust identifiers are case-sensitive, including in the running heading.
        self.c.drawString(MARGIN, HEIGHT - 38, "DIVULGE  /  " + label)
        self.y = HEIGHT - 62
        self.p(title, size=23, leading=28, bold=True, after=13)

    def prose(self, text, **kwargs):
        self.p(html.escape(text), **kwargs)

    def rust(self, text, **kwargs):
        self.code(block(text), **kwargs)

    def finish_page(self):
        self.check()
        self.bottoms.append(round(self.y, 1))
        self.c.setStrokeColor(RULE)
        self.c.line(MARGIN, 40, WIDTH - MARGIN, 40)
        self.c.setFillColor(MUTED)
        self.c.setFont("Vera", 8)
        self.c.drawString(MARGIN, 26, "DIVULGE  |  Reading tail.bin")
        self.c.drawRightString(WIDTH - MARGIN, 26, f"{self.page} / {self.total}")

    def save(self):
        self.finish_page()
        assert self.page == self.total
        self.c.save()
        return self.bottoms

    def graph(self):
        # Existing recorded links, arranged for reading instead of force layout.
        nodes = [
            (42, "#0005  QUESTION", "Where is the", "application?"),
            (178, "#0006  PROBE", "Decode the", "V15 package"),
            (314, "#0007  PACKAGE", "AC791N", "8 entries"),
            (450, "#0015  ENTRY", "tail.bin", "64 bytes"),
        ]
        top = self.y
        c = self.c
        for x, label, line1, line2 in nodes:
            c.setFillColor(PALE)
            c.setStrokeColor(RULE)
            c.roundRect(x, top - 68, 120, 68, 4, fill=1, stroke=1)
            c.setFillColor(TEAL)
            c.setFont("VeraBold", 7)
            c.drawString(x + 8, top - 17, label)
            c.setFillColor(INK)
            c.setFont("Vera", 9)
            c.drawString(x + 8, top - 36, line1)
            c.drawString(x + 8, top - 50, line2)
        for x, text in [(162, "investigates"), (298, "produces"), (434, "produces")]:
            c.setStrokeColor(TEAL)
            c.line(x, top - 34, x + 14, top - 34)
            c.line(x + 14, top - 34, x + 10, top - 31)
            c.line(x + 14, top - 34, x + 10, top - 37)
            c.setFillColor(MUTED)
            c.setFont("Vera", 7)
            c.drawCentredString(x + 8, top - 81, text)
        self.y = top - 105


def build(output, capture, values, revision):
    v = values
    e = v["entry"]
    payload = bytes(e["preview"])
    header = bytes(e["decoded_header"])
    assert len(payload) == 64 and payload[7] == 0x2E
    assert v["record_start"] == 0x270
    assert e["checksum"] == {"stored": 0x8936, "calculated": 0x8936}
    assert v["file_length"] == 699956 and v["package_length"] == 699936
    assert v["package_start"] == 0xAADE0 and v["source_start"] == 0xAADF4
    b = TailBook(output, revision)

    b.start("Reading tail.bin", "The bytes behind Harness Insight")
    b.prose("A single record connects a file, a decoded directory entry, a byte range and several possible interpretations. We can follow each connection in Rust.", after=16)
    top = b.y
    reader = ImageReader(str(capture))
    iw, ih = reader.getSize()
    image_width = 244
    image_height = ih * image_width / iw
    b.c.drawImage(reader, 326, top - image_height, width=image_width, height=image_height)
    b.prose("Where did this number come from?", width=260, size=17, leading=22, bold=True, after=16)
    for title, text in [
        ("FILE", "FM-1_015.fwsc is the original artifact. Its length and SHA-256 anchor everything below it."),
        ("RECORD", "The entry at package+0x270 describes tail.bin. Its fields tell us where to read and how much to read."),
        ("BYTES", "The 64-byte payload begins at package+0xAADE0. Its original file location is 0xAADF4."),
        ("VIEWS", "The selected bytes become u8, i8, u16, u32 or f32 values according to an explicit interpretation."),
    ]:
        b.p(title, width=260, size=9, leading=12, bold=True, color=TEAL, after=5)
        b.prose(text, width=260, after=16)
    b.prose("The central distinction is between a parsed field and a view we choose for arbitrary bytes. Both appear in this panel.", width=260, bold=True)
    b.y = min(b.y, top - image_height - 8)
    b.prose("Original Harness Insight capture: evidence record #0015. The following pages expand its fields and reproduce their values.", size=8.5, leading=12, color=MUTED)

    b.start("The file anchors the record", "Vec<u8> / &[u8] / Result")
    b.prose("We begin with the bytes on disk. A filename is a label; the length and digest identify the exact input accepted by this V15 parser. Reading the file produces an owned Vec<u8>.")
    b.code(section("load"))
    b.prose("The ampersand borrows those bytes. decode_v15 reads them without taking ownership. The ? operator continues with a successful value or returns the error. The entry search gives us a reference to the matching PackageEntry.")
    b.code(f"file length     {v['file_length']} bytes / 0x{v['file_length']:08X}\npackage length  {v['package_length']} bytes / 0x{v['package_length']:08X}\nremoved         20 interleaved marker bytes\n\nfile SHA-256\n{v['file_sha256'][:32]}\n{v['file_sha256'][32:]}", language="observed input")
    b.code(declaration("src/fwsc.rs", "pub fn check(raw: &'a [u8])"), size=8.7)
    b.prose("MatchedV15 borrows the freshly checked input. An unreadable file or a different digest stops this profile with Err. The file digest above is shown on two lines only to make it readable.")
    b.source("src/fwsc.rs", "Source: MatchedV15::check and decode_v15")

    b.start("An 80-byte directory record", "u16 / u32 / [u8; 16]")
    b.prose("tail.bin is directory slot 7, counting from zero. The package header occupies 64 bytes; each directory record occupies 80. The entry's index field is also 7 in this file.")
    b.rust("""
        let slot: usize = 7;
        let record_start: u64 = 64 + slot as u64 * 80;
        assert_eq!(record_start, 0x270);
    """)
    b.table([
        ["Relative offset", "Bytes after decoding", "Rust read", "Displayed value"],
        ["+0x00", "FF 00", "u16", "type 255"],
        ["+0x02", "07 00", "u16", "index 7"],
        ["+0x04", "36 89", "u16", "CRC 0x8936"],
        ["+0x06", "00 00", "u16", "reserved 0x0000"],
        ["+0x08", "E0 AD 0A 00", "u32", "offset 0xAADE0"],
        ["+0x0C", "40 00 00 00", "u32", "length 64"],
        ["+0x10", "40 00 00 00", "u32", "padded length 64"],
        ["+0x14..+0x40", "44 bytes, all 00 here", "preserved bytes", "no field shown"],
        ["+0x40..+0x50", "tail.bin + 8 zero bytes", "[u8; 16] region", "name tail.bin"],
    ], widths=[90, 157, 100, 181], size=9)
    b.prose("The 0x prefix marks hexadecimal. +0x40 means byte 64 relative to the decoded record start. u16 is a 16-bit unsigned integer occupying two bytes; u32 occupies four. The name region has 16 bytes, while the displayed Rust String contains eight ASCII characters.")
    b.code("74 61 69 6C 2E 62 69 6E  00 00 00 00 00 00 00 00\n t  a  i  l  .  b  i  n   padding", language="decoded name bytes")
    b.prose("These are byte widths in the package format. We parse them explicitly; the in-memory layout of a Rust struct is not the file format.")
    b.source("src/fwsc.rs", "Source: decode_package_bytes; PackageEntry")

    b.start("Read the field, check the range", "usize / Result / struct")
    b.prose("The entry header has already passed through the V15 header cipher before these fields are read. The payload preview follows a separate path: marker removal, then a bounded slice of the raw package payload.")
    b.code(section("checked_read"))
    b.prose("usize indexes a slice on this computer. checked_add catches arithmetic overflow, and get checks whether the bytes exist. Result<u16, &str> carries either the decoded number or a short error message.")
    b.rust("""
        let header: &[u8] = &entry.decoded_header;
        assert_eq!(read_u16(header, 0)?, 255);
        assert_eq!(read_u16(header, 2)?, 7);
        assert_eq!(read_u16(header, 4)?, 0x8936);
        assert_eq!(read_u16(header, 79), Err("Need two bytes"));
    """)
    b.code(declaration("src/fwsc.rs", "pub struct PackageEntry"), size=8.6, leading=11)
    b.prose("PackageEntry names the parsed values. decoded_header preserves the original 80 decoded bytes; preview owns up to 256 payload bytes. Here the entire payload fits. String stores display text; Vec<u8> stores bytes with a length known at runtime.")
    b.source("src/fwsc.rs", "Source: PackageEntry, checked_range, u16_at and u32_at; read_u16 above is the teaching equivalent")

    b.start("An offset needs a coordinate space", "newtypes / u64 / checked arithmetic")
    b.prose("package+0xAADE0 and file+0xAADF4 locate the same payload in different byte sequences. Removing marker bytes changes positions. Neither location establishes an address used by the processor.")
    b.rust("""
        pub struct FileOffset(pub u64);
        pub struct PackageOffset(pub u64);
        // The wrapper names keep the coordinate spaces distinct.
    """)
    b.code(declaration("src/fwsc.rs", "pub fn file_offset"), size=8.8)
    b.prose("The file has one marker after each 47 data bytes within its first 960 bytes. The conversion adds the number of markers passed, capped at 20. A universal +20 would give the wrong answer near the start.")
    b.code("package+0x000AADE0 + 0x14 = file+0x000AADF4\n\npackage range  0xAADE0..0xAAE20   64 bytes\nfile range     0xAADF4..0xAAE34   64 bytes\n\nend - start = 0x40 = 64\nlast included package byte = 0xAAE1F", language="this payload / end exclusive")
    spans = v["header_spans"]
    b.prose("The entry header starts much earlier, at package+0x270. Its 80 data bytes cross marker positions, so their original-file provenance is a Vec<SourceSpan>:")
    b.code("\n".join(f"file+0x{s['offset']:08X} / {s['length']:2} bytes" for s in spans), language="header source spans")
    b.prose("The header source spans locate the encoded input bytes. Header decoding also changes their values. The tail payload itself has one contiguous source span after all 20 markers.")
    b.source("src/fwsc.rs", "Source: file_offset and source_spans")

    b.start("The 64 bytes themselves", "slice / chunks / String")
    b.prose("Each hex pair represents one u8. The left column is a package offset; each row advances eight bytes. The ASCII column is a readable view of the same bytes.")
    lines = ["PACKAGE   HEX                       ASCII"]
    for row in range(0, len(payload), 8):
        chunk = payload[row:row + 8]
        ascii_text = "".join(chr(x) if 32 <= x <= 126 else "." for x in chunk)
        lines.append(f"{0xAADE0 + row:08X}  {chunk.hex(' ').upper()}  {ascii_text}")
    b.code("\n".join(lines), size=10, leading=14, language="tail.bin / complete payload")
    b.rust("""
        for chunk in payload.chunks(8) {
            let ascii: String = chunk.iter().map(|byte| {
                if byte.is_ascii_graphic() || *byte == b' ' {
                    *byte as char
                } else {
                    '.'
                }
            }).collect();
            println!("{ascii}");
        }
    """)
    b.prose("A dot can mean a non-printable byte or the literal byte 0x2E. The first row ends in 0x2E; its first byte is a space, 0x20. The hex column resolves that ambiguity.")
    b.rust("""
        assert_eq!(&payload[0x30..0x35], b"JLUFW");
        // package+0xAAE10: 4A 4C 55 46 57
    """)
    b.prose("JLUFW is an observed byte sequence. Calling it a signature is a hypothesis about its use; confirming that use requires code or format evidence that consumes it.")
    b.source("src/insight.rs", "Source: Inspector::bytes, hex rows and ASCII rendering")

    b.start("Byte order changes the integer", "u8 / i8 / u16 / u32 / enum")
    b.prose("Selecting package+0xAADE0 starts a view at 20 34 13 D6. Little endian assigns the first byte the lowest significance. Big endian assigns it the highest.")
    b.rust("""
        let bytes: [u8; 4] = [0x20, 0x34, 0x13, 0xD6];
        let low_pair: [u8; 2] = [bytes[0], bytes[1]];

        assert_eq!(u16::from_le_bytes(low_pair), 0x3420);
        assert_eq!(u16::from_be_bytes(low_pair), 0x2034);
        assert_eq!(u32::from_le_bytes(bytes), 0xD6133420);
        assert_eq!(u32::from_be_bytes(bytes), 0x203413D6);
    """)
    b.table([
        ["View at +0xAADE0", "Little endian", "Big endian"],
        ["u8 / i8", "32 / 32", "32 / 32"],
        ["u16", "13344 / 0x3420", "8244 / 0x2034"],
        ["u32", "3591582752 / 0xD6133420", "540283862 / 0x203413D6"],
    ], widths=[140, 194, 194], size=9)
    b.prose("For the little-endian u16, the calculation is 32 + (52 x 256) = 13344. The first byte contributes 32; the second contributes 52 groups of 256.")
    b.prose("One byte has no byte order to reverse. u8 can represent 0 through 255; i8 can represent -128 through 127. At the selected byte both views give 32. At byte 3, 0xD6 gives 214 as u8 and -42 as i8.")
    b.rust("""
        let unsigned: u8 = 0xD6;
        let signed: i8 = unsigned as i8;
        assert_eq!((unsigned, signed), (214, -42));

        enum ByteOrder { Little, Big }
    """)
    b.prose("The ByteOrder enum represents the two UI choices. This toggle changes the scalar preview; it does not change how the package header was parsed. Those header fields use the format's fixed little-endian reads.")
    b.source("src/evidence.rs", "Source: ByteOrder and ScalarViews::from_bytes")

    b.start("A float view preserves the bits", "f32 / Option / const generics")
    b.prose("The same four bytes can be read as an IEEE 754 single-precision float. from_bits keeps the bit pattern and changes its interpretation. A numeric cast asks for a float representing the integer's value.")
    b.rust("""
        let bits: u32 = 0xD6133420;
        let interpreted: f32 = f32::from_bits(bits);
        let converted: f32 = bits as f32;

        assert_eq!(interpreted.to_bits(), bits);
        println!("{interpreted:e}"); // -4.046302e13
        println!("{converted:e}");   // about +3.591583e9
    """)
    b.prose("This bit pattern has sign 1, biased exponent 172 and fraction 0x133420. Its exact float value is -40,463,021,113,344; the panel uses the shorter scientific representation above. Neither representation proves that the firmware intended a float here.")
    b.code(declaration("src/evidence.rs", "pub struct ScalarViews"), size=9)
    b.prose("At the final byte, the u8 view is Some(0). A two-byte or four-byte read has insufficient input and stays None. Zero is a real value, so it cannot also mean missing data.")
    b.rust("""
        fn window<const N: usize>(bytes: &[u8], index: usize)
            -> Option<[u8; N]>
        {
            bytes.get(index..index.checked_add(N)?)?
                .try_into().ok()
        }
        // window::<4>(payload, 0)  -> Some([0x20, 0x34, 0x13, 0xD6])
        // window::<4>(payload, 63) -> None
    """, size=8.8)
    b.prose("N is a compile-time array length. F32Bits stores the u32 pattern so that saved data can retain values such as NaN payloads and negative zero without losing their bits.")
    b.source("src/evidence.rs", "Source: F32Bits and ScalarViews::from_bytes")

    b.start("What the checksum establishes", "u16 / bool / digest")
    b.prose("The CRC row compares the entry's stored checksum with a calculation over its 64 raw payload bytes. The panel formats both values as hexadecimal: 8936 means 0x8936, which is 35126 in decimal.")
    b.rust("""
        let stored: u16 = read_u16(&entry.decoded_header, 4)?;
        let calculated: u16 = fwsc::crc16(payload);
        let matches: bool = stored == calculated;

        assert_eq!(stored, 0x8936); // header bytes: 36 89
        assert!(matches);
    """)
    b.code(declaration("src/fwsc.rs", "pub fn crc16"), size=8.7, leading=11.5)
    b.prose("This is CRC-16/XMODEM: polynomial 0x1021, initial value 0, no reflection and no final XOR. The comparison checks consistency with this payload. It does not establish its meaning or authenticate its author.")
    b.code(f"payload SHA-256\n{e['sha256'][:32]}\n{e['sha256'][32:]}\n\nCRC scope       these 64 payload bytes\nSHA-256 scope   these same 64 payload bytes", language="tail.bin identity")
    b.prose("The payload digest is different from the whole-file digest because the inputs differ. SHA-256 identifies this byte sequence with a much larger digest; a matching hash by itself still does not supply a trusted origin.")
    b.source("src/fwsc.rs", "Source: crc16, CrcCheck::matches and digest")

    b.start("From Rust data to egui", "borrowed view / Option / selection")
    b.prose("The parser owns the data. The panel borrows a view of it, together with the coordinates and transformation that explain where those bytes came from.")
    b.code(declaration("src/data_view.rs", "pub struct ByteView"), size=9.2)
    b.prose("The lifetime 'a ties the byte slice to the data it borrows. source can contain several file spans. mapped is None for this entry, which becomes the visible text 'unresolved / no load map'.")
    b.rust("""
        let mapped_label = view.mapped.map_or_else(
            || "unresolved / no load map".to_owned(),
            |address| format!("0x{address:08X}"),
        );
        ui.label(mapped_label);
    """, language="rust / reduced display example")
    b.rust("""
        ui.selectable_value(
            &mut self.byte_order, ByteOrder::Little, "little endian",
        );
        ui.selectable_value(
            &mut self.byte_order, ByteOrder::Big, "big endian",
        );
        let values = ScalarViews::from_bytes(
            view.bytes, self.byte_index, self.byte_order,
        );
    """, language="rust / selection flow")
    b.prose("A click changes the selected index or byte order. The next UI draw computes the displayed numbers from those choices. The raw evidence remains available beneath each view, and selecting another record resets the byte index.")
    b.source("src/data_view.rs", "Source: ByteView and bytes")
    b.source("src/insight.rs", "Source: byte selection and endian controls")

    b.start("The question behind the record", "RecordId / enum / evidence links")
    b.prose("Question #0005 asks where the application is. The package decode produces a directory, and the directory produces the tail.bin record. These links explain how we arrived at this screen.")
    b.graph()
    b.rust("""
        let record = RecordId(15);
        let data = RecordData::Firmware(FirmwareView::Entry(7));
        let question = QuestionId::new(5)?;
    """, language="rust / this record's identities")
    b.prose("15 identifies an investigation record. 7 selects a package entry. 5 identifies the question it belongs to. The entry's on-disk index also happens to be 7. These numbers have different roles; none is a byte address.")
    b.code(declaration("src/domain.rs", "pub struct HarnessInsight"), size=9, leading=11.5)
    b.prose("The record asks: 'Where is tail.bin, and what is actually validated?' Its observation is the bounded 64-byte span and matching raw CRC. Its explanation connects those checks to the entry. Its uncertainty is the payload's runtime purpose.")
    b.prose("That gives the graph a useful next question: which code reads this entry or its JLUFW bytes, and what does it check? A stronger explanation needs evidence for that use.", bold=True)
    b.source("src/domain.rs", "Source: Record, RecordData and HarnessInsight")
    b.source("src/firmware_records.rs", "Source: package records and their question ownership")

    b.start("Carry the evidence into pi32v2", "Reproduce / explain / investigate")
    b.prose("We can reproduce this panel from a local copy of the exact V15 file. The example uses the same parser and scalar-view code as DIVULGE, and emits the fields, source spans and interpretations as JSON.")
    b.code("cargo run --offline --example workshop_tail_bin -- path/to/FM-1_015.fwsc", language="terminal", size=8.8)
    b.code("type             255\nentry index      7\nrecord start     package+0x270\npayload span     package+0xAADE0..0xAAE20\noriginal source  file+0xAADF4 / 64 B\nCRC              stored 0x8936 / calculated 0x8936\nu16 / LE         13344 / 0x3420\nu32 / LE         3591582752 / 0xD6133420\nf32 / same bits  -4.046302e13\nmapped address   unresolved", language="values to recognize in the output", size=9.3, leading=12.5)
    b.prose("The tail entry does not supply a load map. To make pi32v2 instruction claims, follow the package's flash.bin member into the decoded application, establish its checked byte span and address mapping, then inspect the instructions that consume relevant data.")
    b.table([
        ["Question", "What would answer it?"],
        ["Where is tail.bin?", "The parsed entry, checked range and original-file source span."],
        ["Are these bytes a float?", "Evidence that code consumes them as a float; the f32 view alone is insufficient."],
        ["What does JLUFW mean?", "A format definition or code that compares, writes or interprets those bytes."],
        ["What belongs in the graph?", "The question, action, observed result, explanation and remaining uncertainty, linked by record IDs."],
    ], widths=[163, 365], size=9)
    b.prose("Types make each step explicit: bytes in a Vec, a borrowed slice, fixed-width reads, named coordinate spaces, optional mappings and evidence-linked records. Every displayed claim has a place to trace it back to the input.")
    b.source("src/fwsc.rs", "Source: decode_application and Application address fields")
    b.source("src/coordinates.rs", "Source: ImageMap, ImageSpan and Location")
    return b.save()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--values", type=pathlib.Path, required=True)
    parser.add_argument("--capture", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "output/pdf/DIVULGE-tail-bin.pdf")
    args = parser.parse_args()
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    values = json.loads(args.values.read_text(encoding="utf-8-sig"))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bottoms = build(args.output, args.capture, values, revision)
    print(json.dumps({"pdf": str(args.output), "pages": 12, "content_bottoms_pt": bottoms}, indent=2))


if __name__ == "__main__":
    main()
