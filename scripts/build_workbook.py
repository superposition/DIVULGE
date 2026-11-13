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
from workshop_content import chapters, write_notes

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
        pdfmetrics.registerFont(TTFont("Hack", str(ROOT / "workshop/fonts/Hack-Regular.ttf")))
        pdfmetrics.registerFontFamily("Vera", normal="Vera", bold="VeraBold", italic="VeraItalic", boldItalic="VeraBold")
        self.c = canvas.Canvas(str(path), pagesize=(WIDTH, HEIGHT), invariant=1, pageCompression=1,
                               initialFontName="Vera", initialFontSize=10.5, lang="en-US")
        self.c.setTitle("DIVULGE - From Bytes to Evidence")
        self.c.setAuthor("DIVULGE contributors")
        self.c.setSubject("A beginner Rust workshop and native investigation workbook")
        self.c.setKeywords("DIVULGE,Rust,evidence,source:" + revision)
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

    def code(self, text, size=9, leading=11.8, after=12, language="rust"):
        lines = []
        limit = CONTENT - 24
        for original in clean(text).strip("\n").splitlines():
            line = original
            continuation = " " * (len(original) - len(original.lstrip()) + 4)
            while pdfmetrics.stringWidth(line, "Hack", size) > limit:
                cut = len(line)
                while pdfmetrics.stringWidth(line[:cut], "Hack", size) > limit:
                    cut -= 1
                split = line.rfind(" ", 0, cut)
                cut = split if split > len(continuation) else cut
                lines.append(line[:cut])
                line = continuation + line[cut:].lstrip()
            lines.append(line)
        height = len(lines) * leading + 32
        self.c.setFillColor(PALE)
        self.c.roundRect(MARGIN, self.y - height, CONTENT, height, 4, fill=1, stroke=0)
        self.c.setFillColor(TEAL)
        self.c.setFont("VeraBold", 7.5)
        self.c.drawString(MARGIN + 12, self.y - 13, language.upper())
        self.c.setFont("Hack", size)
        for n, line in enumerate(lines):
            self.c.setFillColor(MUTED if line.lstrip().startswith("//") else INK)
            self.c.drawString(MARGIN + 12, self.y - 28 - n * leading, line)
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
        self.c.drawString(MARGIN, 26, "DIVULGE 0.1.0  |  Rust and evidence")
        self.c.drawRightString(WIDTH-MARGIN, 26, str(self.page) + " / 18")

    def save(self):
        self.finish_page()
        if self.page != 18:
            raise ValueError("Expected 18 pages")
        self.c.save()
        return self.bottoms


def build(output, revision):
    book = Book(output, revision)
    for page in chapters():
        book.start(page["title"], page["topic"])
        for block in page["blocks"]:
            if block["kind"] == "prose":
                book.p(html.escape(block["text"]), size=10.5, leading=15, after=12)
            elif block["kind"] == "code":
                book.code(block["text"], language=block["language"])
            elif block["kind"] == "source":
                book.source(block["path"], block["label"])
            elif block["kind"] == "image":
                book.image(block["file"], block["crop"], block["width"], block["caption"])
    return book.save()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=pathlib.Path, default=ROOT / "output/pdf/DIVULGE-workshop.pdf")
    parser.add_argument("--source-ref", default=None)
    parser.add_argument("--write-notes", action="store_true")
    args = parser.parse_args()
    revision = args.source_ref or subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if args.write_notes:
        write_notes(chapters())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    bottoms = build(args.output, revision)
    print(json.dumps({"output": str(args.output), "pages": 18, "source_ref": revision, "content_bottoms_pt": bottoms}, indent=2))


if __name__ == "__main__":
    main()
