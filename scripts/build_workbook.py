#!/usr/bin/env python3
"""Build the versioned workbook from the checked lessons, source and native captures."""
import argparse
import html
import json
import pathlib
import subprocess

import reportlab
from reportlab.lib import colors
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.utils import ImageReader
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.pdfgen import canvas
from reportlab.platypus import Paragraph, Table, TableStyle

ROOT = pathlib.Path(__file__).resolve().parents[1]
WIDTH, HEIGHT, MARGIN = 612, 792, 42
CONTENT = WIDTH - 2 * MARGIN
INK = colors.HexColor("#182d35")
TEAL = colors.HexColor("#176b65")
MUTED = colors.HexColor("#53636b")
PALE = colors.HexColor("#f0f5f4")
RULE = colors.HexColor("#c5d1d1")
BASE = "https://github.com/superposition/DIVULGE"


def clean(value):
    return str(value).replace("\u2013", "-").replace("\u2014", " - ").replace("\u2011", "-").replace("\u2192", "->")


class Book:
    def __init__(self, path, revision):
        fonts = pathlib.Path(reportlab.__file__).parent / "fonts"
        for name, file in [("Vera", "Vera.ttf"), ("VeraBold", "VeraBd.ttf"), ("VeraItalic", "VeraIt.ttf")]:
            pdfmetrics.registerFont(TTFont(name, str(fonts / file)))
        pdfmetrics.registerFontFamily("Vera", normal="Vera", bold="VeraBold", italic="VeraItalic", boldItalic="VeraBold")
        self.c = canvas.Canvas(str(path), pagesize=(WIDTH, HEIGHT), invariant=1, pageCompression=1,
                               initialFontName="Vera", initialFontSize=10.5, lang="en-US")
        self.c.setTitle("DIVULGE - From Bytes to Evidence")
        self.c.setAuthor("DIVULGE contributors")
        self.c.setSubject("A beginner Rust workshop and native investigation workbook")
        self.revision = revision
        self.page = 0
        self.y = 0
        self.bottoms = []

    def start(self, title, label):
        if self.page:
            self.finish_page()
            self.c.showPage()
        self.page += 1
        self.c.bookmarkPage("page-" + str(self.page))
        self.c.addOutlineEntry(clean(title), "page-" + str(self.page), 0)
        self.c.setFillColor(TEAL)
        self.c.setFont("VeraBold", 9)
        self.c.drawString(MARGIN, HEIGHT - 38, "DIVULGE  /  " + clean(label).upper())
        self.y = HEIGHT - 62
        self.p(title, size=23, leading=28, bold=True, after=13)

    def check(self):
        if self.y < 53:
            raise ValueError("Page {} overflows: bottom {}".format(self.page, self.y))

    def p(self, text, size=10.5, leading=15, bold=False, after=9, width=None, x=None, color=INK):
        style = ParagraphStyle("body", fontName="VeraBold" if bold else "Vera", fontSize=size,
                               leading=leading, textColor=color, spaceAfter=0)
        paragraph = Paragraph(clean(text), style)
        _, height = paragraph.wrap(width or CONTENT, 1000)
        paragraph.drawOn(self.c, MARGIN if x is None else x, self.y - height)
        self.y -= height + after
        self.check()

    def label(self, name, text):
        self.p('<b><font color="#176b65">' + name + '</font></b>  ' + html.escape(clean(text)), after=7)

    def code(self, text, size=9.5, leading=12, after=10):
        # Proportional embedded Vera keeps the source selectable; indentation is retained.
        lines = []
        limit = CONTENT - 20
        for original in clean(text).strip("\n").splitlines():
            line = original
            continuation = " " * (len(original) - len(original.lstrip()) + 4)
            while pdfmetrics.stringWidth(line, "Vera", size) > limit:
                cut = len(line)
                while pdfmetrics.stringWidth(line[:cut], "Vera", size) > limit:
                    cut -= 1
                split = line.rfind(" ", 0, cut)
                cut = split if split > len(continuation) else cut
                lines.append(line[:cut])
                line = continuation + line[cut:].lstrip()
            lines.append(line)
        height = len(lines) * leading + 16
        self.c.setFillColor(PALE)
        self.c.roundRect(MARGIN, self.y - height, CONTENT, height, 4, fill=1, stroke=0)
        self.c.setFillColor(INK)
        self.c.setFont("Vera", size)
        for n, line in enumerate(lines):
            self.c.drawString(MARGIN + 10, self.y - 12 - n * leading, line)
        self.y -= height + after
        self.check()

    def table(self, rows, widths=None, size=9.5):
        style = ParagraphStyle("cell", fontName="Vera", fontSize=size, leading=size+4, textColor=INK)
        cells = [[Paragraph(html.escape(clean(cell)), style) for cell in row] for row in rows]
        table = Table(cells, colWidths=widths or [CONTENT / len(rows[0])] * len(rows[0]))
        table.setStyle(TableStyle([
            ("FONTNAME", (0, 0), (-1, -1), "Vera"),
            ("VALIGN", (0, 0), (-1, -1), "TOP"), ("BACKGROUND", (0, 0), (-1, 0), PALE),
            ("LINEBELOW", (0, 0), (-1, 0), 0.8, TEAL), ("LINEBELOW", (0, 1), (-1, -1), 0.3, RULE),
            ("LEFTPADDING", (0, 0), (-1, -1), 7), ("RIGHTPADDING", (0, 0), (-1, -1), 7),
            ("TOPPADDING", (0, 0), (-1, -1), 6), ("BOTTOMPADDING", (0, 0), (-1, -1), 6)]))
        _, height = table.wrap(CONTENT, 1000)
        table.drawOn(self.c, MARGIN, self.y - height)
        self.y -= height + 12
        self.check()

    def source(self, path, label=None):
        url = BASE + "/blob/" + self.revision + "/" + path
        self.p('<link href="' + url + '" color="#176b65">' + html.escape(label or path) + '</link>', size=8.5, leading=12, after=8)

    def image(self, file, crop, width, caption):
        # Clip the original native capture in PDF space; never synthesize UI pixels.
        reader = ImageReader(str(ROOT / "workshop/assets" / file))
        iw, ih = reader.getSize()
        left, top, right, bottom = crop
        scale = width / (right - left)
        height = (bottom - top) * scale
        x = MARGIN + (CONTENT - width) / 2
        y = self.y - height
        self.c.saveState()
        path = self.c.beginPath()
        path.rect(x, y, width, height)
        self.c.clipPath(path, stroke=0)
        self.c.drawImage(reader, x - left * scale, y - (ih - bottom) * scale,
                         width=iw * scale, height=ih * scale)
        self.c.restoreState()
        self.y = y - 7
        self.p(caption, size=8.5, leading=12, color=MUTED, after=12)

    def lines(self, prompt, count=2):
        self.p(prompt, bold=True, after=3)
        self.c.setStrokeColor(RULE)
        self.c.setLineWidth(0.4)
        for _ in range(count):
            self.y -= 24
            self.c.line(MARGIN, self.y, WIDTH-MARGIN, self.y)
        self.y -= 13
        self.check()

    def finish_page(self):
        self.check()
        self.bottoms.append(round(self.y, 1))
        self.c.setStrokeColor(RULE)
        self.c.line(MARGIN, 40, WIDTH-MARGIN, 40)
        self.c.setFillColor(MUTED)
        self.c.setFont("Vera", 8)
        self.c.drawString(MARGIN, 26, "DIVULGE 0.1.0  |  source " + self.revision[:7] + "  |  synthetic lessons")
        self.c.drawRightString(WIDTH-MARGIN, 26, str(self.page) + " / 18")

    def save(self):
        self.finish_page()
        if self.page != 18:
            raise ValueError("Expected 18 pages")
        self.c.save()
        return self.bottoms


def build(output, revision):
    data = json.loads((ROOT / "workshop/lessons.json").read_text(encoding="utf-8"))
    b = Book(output, revision)
    b.start("From bytes to evidence", "Preparation / before the clock")
    b.p("A 20-minute Rust introduction, a guided native investigation, and a reference for the longer FM-1 talk.", size=14, leading=20, after=15)
    b.p("You need variables and functions, curiosity about bytes, and the matching DIVULGE 0.1.0 source or release package. The six lessons use only <b>34 12</b>, generated teaching data. They need no firmware, model or network.")
    b.label("PREPARE", "Windows is the verified desktop platform. Source builds need Rust 1.92+ with the MSVC linker/Windows SDK and an OpenGL-capable desktop. Install and compile before the twenty minutes.")
    b.code("cargo build --locked --examples\ncargo build --locked --bin divulge\npython scripts/check_workshop.py\npython scripts/check_catalogue.py\ncargo run --locked -- --compact --session\n    workshop/fixtures/synthetic-workshop.json")
    b.p("The final command is shown wrapped: enter it as one line. Python 3.8+ runs the checks; the app itself needs no Python. Prebuilt releases can open the included replay through File &gt; Open investigation.")
    b.table([["Clock", "Focus", "Pages"], ["00:00-02:00", "Orient: question, action, evidence, explanation", "2-3"], ["02:00-18:00", "Six small Rust checkpoints", "4-9"], ["18:00-20:00", "Evidence, limitation, next useful probe", "11"], ["After the introduction", "Review, firmware, type reference and facilitation", "10-18"]], [90, 380, 58])
    b.label("KEEP UP", "Each completed example runs independently. If a build stalls, use the printed output and saved replay, then rejoin at the next checkpoint.")
    b.source("docs/workshop.md", "Preparation, commands and teaching route")

    b.start("A question becomes a record", "Visual investigation map")
    b.p("The graph is a projection of stored records. Its position is presentation; its IDs, kinds, citations and recorded text carry the meaning.")
    b.table([["Record", "What it contributes", "Relationship"], ["1 Question", "What can these bytes establish?", "motivates a probe"], ["2 Probe", "Read the two-byte fixture", "produces observation 3"], ["3 Observation", "file+0x0 contains 34 12", "supports explanation 4"], ["4 Explanation", "Chosen little-endian view: 4660", "leaves unknown 5"], ["5 Unknown / 6 Question", "What would establish intended meaning?", "plans the next investigation"], ["7 Analyst review", "Keep representation separate from meaning", "revises record 4; preserves it"]], [119, 263, 146])
    b.image("synthetic-evidence.jpg", (265, 308, 844, 700), 430,
            "Actual native Graph: Neighbors around observation 3. The scope count states which links are outside this view. Labels identify kinds without relying on color.")
    b.source("src/graph.rs", "Graph projection, scopes and accessibility")

    b.start("Read the call and response", "Harness Insight / actual native app")
    b.p("Select explanation 4 and scroll the terminal pane. This is retained, inspectable work: the question, purpose, action, observation, explanation, evidence, uncertainty and next probe.")
    b.image("synthetic-explanation.jpg", (870, 199, 1268, 679), 398,
            "Actual native Harness Insight, synthetic replay. The evidence link returns to record 3. These sections explain the recorded result; they are not a model's hidden reasoning.")
    b.label("READING HABIT", "Read the observation first, check the citation, then assess the explanation and its limit. Ask what new observation would change the claim.")
    b.source("src/insight.rs", "Inspector rendering and evidence navigation")

    for index, lesson in enumerate(data["lessons"], 1):
        b.start(lesson["title"], "Lesson {} / {}".format(index, clean(lesson["minutes"])))
        b.label("ASK", lesson["ask"])
        b.label("PREDICT", lesson["predict"])
        b.label("CODE", lesson["live_edit"])
        file = "examples/" + lesson["slug"] + ".rs"
        source = (ROOT / file).read_text(encoding="utf-8")
        source = "\n".join(line for line in source.splitlines() if not line.startswith("//!"))
        b.code(source, size=9.2, leading=11.3, after=7)
        b.label("OBSERVE", "cargo run --locked --example " + lesson["slug"])
        b.code(lesson["stdout"], size=8.9, leading=11.4, after=7)
        b.label("EXPLAIN", lesson["explain"])
        b.label("FIND IT", lesson["find"])
        b.source(file, "Runnable checkpoint " + str(index) + " / full source")
        b.lines("In my own words: what did this type help us preserve?", 2)

    b.start("Make the wrong call on purpose", "Compiler exercise / lesson 5 companion")
    b.p("Predict the result before compiling. Both wrappers contain numbers, but their names describe different coordinate spaces.")
    b.code((ROOT / "workshop/exercises/wrong_coordinate.rs").read_text(encoding="utf-8"))
    b.label("EXPECTED", "E0308: the file reader requires FileOffset; this call supplies MappedAddress. The exercise verifier checks this error and both type names.")
    b.code("Copy-Item workshop/exercises/wrong_coordinate.rs examples/live_workshop.rs\ncargo run --locked --example live_workshop")
    b.p("This PowerShell copy creates an ignored livecoding file. On a POSIX shell, use cp. The instructor answer uses the actual start of our two-byte buffer:")
    b.code((ROOT / "workshop/exercises/coordinate_answer.rs").read_text(encoding="utf-8"))
    b.code("Copy-Item workshop/exercises/coordinate_answer.rs examples/live_workshop.rs\ncargo run --locked --example live_workshop\n# Output: Ok(4660)")
    b.label("DISCUSS", "Constructing FileOffset(address.0) would silence the mismatch without proving any mapping. A valid wrapper is not a guarantee that a numeric value is correct.")
    b.source("workshop/exercises/wrong_coordinate.rs")
    b.source("workshop/exercises/coordinate_answer.rs")

    b.start("Make one defensible claim", "Closing task / 18:00-20:00")
    b.p("Open the included synthetic-workshop.json replay. Work in pairs, or answer aloud. Select explanation 4 and follow its evidence link to observation 3.")
    b.lines("1. Which exact bytes and range were observed?", 2)
    b.lines("2. What interpretation produced 4660?", 2)
    b.lines("3. Change byte order. What changes, and what stays fixed?", 2)
    b.lines("4. Name one limitation shared by both integer views.", 2)
    b.lines("5. Choose a next probe with an observable outcome.", 2)
    b.label("SUCCESS", "A useful answer identifies evidence, states a bounded claim and proposes a way to distinguish competing meanings. Confidence alone does not add evidence.")
    b.source("workshop/fixtures/synthetic-workshop.json", "Included offline replay: seven records, two history events")

    b.start("Review without losing your place", "Guided application worksheet")
    b.table([["Do this", "Look for"], ["Select correction 7", "Original explanation 4 still exists; the review records its own scope."], ["Investigation > Replay from beginning", "The frontier advances independently from your reading position."], ["Select an older record", "Reading and Latest can show different IDs."], ["Review next / Follow latest", "Review an arrival, or release the reader to follow progress."], ["Presenter hold", "Protect the reader's position; this is separate from pausing AI execution."], ["File > Save investigation as...", "Save your exercise copy, preserving the shipped checkpoint."], ["Close and reopen the copy", "Find the exact saved note; reopening starts no worker or model."]], [225, 303])
    b.lines("My saved note: what would make this explanation stronger?", 3)
    b.label("HISTORY", "An analyst correction adds a record; it does not silently rewrite the old finding. Note revisions and recorded jobs belong to an ordered history. A replay is a reconstruction of recorded work.")
    b.label("PORTABLE REVIEW", "The included synthetic-review.json is a reading export. It is not an executable session or a canonical REA evidence bundle. In your own exports, choose bytes, text and original receipts deliberately.")
    b.source("docs/history.md", "Session envelope, history and export format")
    b.source("workshop/fixtures/synthetic-review.json", "Included scoped synthetic review")

    b.start("Later: the FM-1 coordinate problem", "Firmware module / outside the opening twenty minutes")
    b.p("The local V15 case follows identity -> package -> mapped application -> pi32v2 observations -> bounded explanation -> next question. Obtain the exact reference artifact and installed tools separately; none is bundled here.")
    b.image("v15-code-large.jpg", (1505, 169, 1908, 473), 403,
            "Actual native instruction inspector from the local V15 case. This small recorded excerpt shows coordinates and a static direct-jump calculation; it does not establish runtime behavior.")
    b.table([["Coordinate", "Meaning for this first decoded row"], ["file+0x00004534", "Location in the original .fwsc artifact"], ["app.bin+0x00000000", "Offset in the decoded application image"], ["declared 0x02000120", "Address declared by the decoded-image metadata"], ["target 0x02000124", "Candidate from current PC + 2-byte width + signed displacement"]], [176, 352])
    b.label("BOUNDARY", "A disassembler reports an interpretation of bytes under an instruction-set model. Static rows do not prove executed paths, function names, runtime register values or original source types.")
    b.source("docs/pi32v2-validation.md", "Decoder integration, coordinates and limitations")

    b.start("Types that carry evidence", "Reference / actual application declarations")
    b.p("The catalogue inventories every named struct, enum, alias and macro-generated ID/span in the implemented source. The source checker currently covers 124 types. This page is a route through them, not a second beginner lecture.")
    b.table([["Type family", "What it carries", "Mistake made harder"], ["RecordId / ObservationId", "Identity and a checked evidence role", "Citing a missing record or a question as an observation"], ["RecordData", "Question, probe, observation, explanation, unknown and other implemented variants", "Treating every record as the same kind of claim"], ["Observation / HarnessInsight", "Observed bytes and the explanation/uncertainty shown in the inspector", "Losing the distinction between observation and interpretation"], ["FileOffset / ImageOffset / FlashOffset / MappedAddress", "Distinct source-coordinate spaces", "Accidentally passing a CPU address to a file reader"], ["Checked spans and mappings", "A checked start/length and explicit conversion between spaces", "Silent overflow or unbounded reads"], ["TypeHypothesis", "An analyst's candidate layout or field interpretation", "Promoting a guessed target type to proven fact"], ["Tool receipts / provenance", "The actual request, output, status and artifact identity", "Presenting a cancelled or failed run as fresh evidence"], ["History / event records", "Ordered changes that reconstruct a saved case", "Replaying by executing tools again"]], [156, 186, 186], size=9.3)
    b.label("TYPE SAFETY HAS LIMITS", "A compiler can reject incompatible coordinates. Runtime validation can reject an out-of-range span. Neither proves a reverse-engineering hypothesis's semantic truth.")
    b.source("docs/type-catalogue.md", "Complete 124-type catalogue with source links and validation boundaries")
    b.source("scripts/check_catalogue.py", "Check the catalogue against the actual declarations")

    b.start("Rust forms behind the workbench", "Reference / after the beginner route")
    b.table([["Form", "Use in this application"], ["u8; u16/u32/u64; usize", "Bytes; fixed-width integers; collection indices. Width alone says nothing about byte order."], ["[T; N] and &[T]", "A fixed-size array and a borrowed slice. A slice does not own the bytes."], ["Vec<T> and String", "Owned collections and UTF-8 text retained in records."], ["Option<T> and Result<T, E>", "Absence versus an operation's success or failure."], ["&T, &mut T and lifetimes", "Read/shared or exclusive mutable access, bounded by the owner."], ["HashMap/HashSet; BTreeMap/BTreeSet", "Lookup and uniqueness; ordered keys when deterministic order matters."], ["Box<T> and Arc<T>", "Owned indirection and shared ownership across workers."], ["Sender<T>/Receiver<T>; atomics", "Typed worker messages, cancellation flags and generation checks."], ["Serialize/Deserialize; JSON", "Wire representation. Decoding a saved record grants no fresh-source capability."], ["Ui/Context/Response; Pos2/Rect", "egui layout, interaction and camera geometry. Presentation values are not target addresses."], ["match, if let, ? and impl", "Syntax for alternatives, propagation and methods - not data types."]], [203, 325], size=9.7)
    b.label("FOLLOW A VALUE", "Choose one byte from observation 3. Trace its owner, the borrowed view, the scalar conversion, the explanation and the citation back to that owner.")
    b.source("docs/rust-forms.md", "Expanded primitives, collections, traits, concurrency and egui reference")

    b.start("Run the twenty-minute room", "Instructor notes / pacing")
    b.table([["Time", "Say / do", "Stop point"], ["00:00-02:00", "Ask what two bytes can establish. Show records 1-4.", "One learner distinguishes bytes from meaning."], ["02:00-05:00", "Add the big-endian conversion. Predict before running.", "4660 and 13330; bytes unchanged."], ["05:00-07:30", "Fill an Observation and change its description.", "Owned evidence survives the wording change."], ["07:30-10:00", "Write three match arms; inspect the remaining variants.", "Record kind is explicit."], ["10:00-12:30", "Truncate a read and compare it with None.", "Failure and absence are different."], ["12:30-16:00", "Borrow, reuse the owner, show E0308, then fix the coordinate.", "A map is needed, not a cast."], ["16:00-18:00", "Resolve the evidence ID and print the limitation.", "Record 4 cites observation 3."], ["18:00-20:00", "Use the app and the page 11 closing task.", "Evidence + limitation + useful next probe."]], [86, 277, 165], size=9.3)
    b.label("LIVE-CODE ONLY THE CHANGE", "Keep imports, completed checkpoints and expected outputs ready. Ask for one prediction per lesson. The full source is here for recovery; typing it all is not the teaching objective.")
    b.label("WHEN TIME SLIPS", "Show the verified output and move on. Preserve the final two-minute evidence exercise. Keep tool setup, firmware acquisition and model configuration outside this slot.")
    b.label("MEASURE HONESTLY", "Record command/UI timing separately from spoken delivery. A fast automated verifier is not a rehearsal, and an operator walkthrough is not audience-tested speech.")
    b.source("docs/workshop.md", "Presenter route and recovery instructions")

    b.start("Instructor answers and prompts", "Discussion / leave room for uncertainty")
    for index, lesson in enumerate(data["lessons"], 1):
        b.label(str(index).zfill(2) + " / ANSWER", lesson["answer"])
    b.p("Closing exercise: a model answer", bold=True, after=8)
    b.p("Observation 3 retains two bytes, 34 12, at file offset 0. Reading them as a little-endian u16 yields 4660. A big-endian view yields 13330. Neither observation establishes an intended field meaning. A format declaration or controlled example could distinguish those interpretations.")
    b.lines("Follow-up question I will ask the room", 2)
    b.label("AVOID", "Do not say the model discovered the original Rust type. Rust is the host application's language; the target's source types and intent require separate evidence.")
    b.source("workshop/lessons.json", "Maintained questions, live edits, exact outputs and instructor answers")

    b.start("Keep the investigation reproducible", "Version, sources and next session")
    b.p("This workbook targets <b>DIVULGE 0.1.0</b>. The footer identifies the source revision used for its code links. The six examples and the two-byte fixture are checked against the application library. Screenshots were captured from the actual Windows native build, not a design mockup.")
    b.code("python scripts/check_workshop.py\npython scripts/check_catalogue.py\npython scripts/build_workbook.py\npdftoppm -r 120 -png output/pdf/DIVULGE-workshop.pdf\n    tmp/pdfs/workbook")
    b.p("The rendering command is wrapped for the page; enter it on one line. PDF creation needs Python 3.11+ with workshop/pdf-requirements.txt installed and Poppler for rendering. The builder reads the maintained lesson JSON, actual example files and original screenshots. Every rendered page needs visual review after meaningful edits.")
    b.table([["Included", "Obtained separately"], ["Native application, runnable lessons, saved synthetic replay, scoped review and workbook", "Exact reference firmware, vendor decoder, installed REA/OMP tools and any model access"], ["Local source identity and static evidence can be inspected in the later talk", "A saved case does not reattach source bytes or prove a live device's behavior"], ["Windows native behavior is verified", "Other operating systems remain unverified"]], [264, 264])
    b.label("NEXT SESSION", "Follow the V15 package map, choose a small pi32v2 range and preserve its receipt. Explain the static result with all coordinate spaces visible. Finish with one unresolved question and the observation needed to answer it.")
    b.source("README.md", "Repository, build instructions and current capabilities")
    b.source("docs/native-validation.md", "Recorded native checks and their limits")
    b.source("docs/type-catalogue.md", "All implemented named types")
    b.p("Document design uses ink-friendly vector text, embedded Bitstream Vera fonts from ReportLab, and small native excerpts. Application/source: MIT. Font notices accompany ReportLab. The original firmware remains a separately obtained reference artifact.", size=8.5, leading=12)
    return b.save()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "output/pdf/DIVULGE-workshop.pdf")
    parser.add_argument("--source-ref", default=None)
    args = parser.parse_args()
    revision = args.source_ref or subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bottoms = build(args.output, revision)
    print(json.dumps({"output": str(args.output), "pages": 18, "source_ref": revision, "content_bottoms_pt": bottoms}, indent=2))


if __name__ == "__main__":
    main()
