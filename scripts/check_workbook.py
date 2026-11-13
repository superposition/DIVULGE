#!/usr/bin/env python3
"""Structural PDF checks supplement, but never replace, reviewing every rendered page."""
import argparse
import hashlib
import pathlib
import re
import subprocess

from pypdf import PdfReader

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("pdf", nargs="?", type=pathlib.Path,
                        default=ROOT / "output/pdf/DIVULGE-workshop.pdf")
    args = parser.parse_args()
    reader = PdfReader(args.pdf)
    assert len(reader.pages) == 18, "Expected 18 pages"
    sources, fonts = set(), set()
    for number, page in enumerate(reader.pages, 1):
        text = page.extract_text()
        assert len(text) > 400, "Missing selectable text on page " + str(number)
        assert "{} / 18".format(number) in text, "Missing page number"
        assert "DIVULGE 0.1.0" in text, "Missing release identity"
        for entry in page["/Resources"]["/Font"].values():
            font = entry.get_object()
            descriptor = font.get("/FontDescriptor")
            assert descriptor, "Font lacks embedding descriptor: " + str(font)
            descriptor = descriptor.get_object()
            assert any(key in descriptor for key in ["/FontFile", "/FontFile2", "/FontFile3"]), "Unembedded font"
            fonts.add(str(font["/BaseFont"]))
        for entry in page.get("/Annots", []):
            uri = entry.get_object().get("/A", {}).get("/URI")
            if not uri:
                continue
            match = re.fullmatch(r"https://github.com/superposition/DIVULGE/blob/([0-9a-f]{40})/(.+)", uri)
            assert match, "Unexpected source link: " + uri
            revision, path = match.groups()
            subprocess.run(["git", "cat-file", "-e", revision + ":" + path], cwd=ROOT, check=True,
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            sources.add(path)
    assert len(sources) >= 15, "Source links missing"
    for number in range(4, 10):
        text = reader.pages[number-1].extract_text()
        for label in ["ASK", "PREDICT", "CODE", "OBSERVE", "EXPLAIN", "FIND IT"]:
            assert label in text, "Lesson {} lacks {}".format(number-3, label)
    digest = hashlib.sha256(args.pdf.read_bytes()).hexdigest()
    print("18 pages; selectable text; embedded fonts; {} checked source links".format(len(sources)))
    print("Fonts: " + ", ".join(sorted(fonts)))
    print("SHA-256: " + digest)
    print("Structural checks passed. Review every rendered page separately.")


if __name__ == "__main__":
    main()
