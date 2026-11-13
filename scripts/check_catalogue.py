#!/usr/bin/env python3
"""Fail on catalogue/source drift; render the checked catalogue without needing Rust internals."""
import argparse
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]

def declarations():
    found = {}
    for path in sorted((ROOT / "src").glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        for match in re.finditer(r"^(?:pub(?:\([^)]*\))? )?(?:struct|enum|type) (\w+)", text, re.M):
            found[path.stem + "::" + match.group(1)] = (path, text[:match.start()].count("\n")+1)
        for match in re.finditer(r"(?:record_reference|checked_span)!\(\s*(\w+)", text):
            found[path.stem + "::" + match.group(1)] = (path, text[:match.start()].count("\n")+1)
    return found

def render():
    found = declarations()
    groups = json.loads((ROOT / "workshop/type-catalogue.json").read_text(encoding="utf-8"))["groups"]
    documented = {g["module"] + "::" + name for g in groups for name in g["types"]}
    if documented != set(found):
        raise AssertionError("Catalogue drift. Missing: {}; stale: {}".format(sorted(set(found)-documented),sorted(documented-set(found))))
    lines = ["# DIVULGE type catalogue", "", "Generated from maintained descriptions in workshop/type-catalogue.json and checked against every top-level declaration in src, including macro-generated IDs/spans. Run python scripts/check_catalogue.py --write after intentional updates.", "",
             "{} implemented named types. This index includes private runtime/presentation helpers. It does not claim that a host Rust type was recovered from target firmware. See [the 20-minute route](workshop.md) for the beginner subset and [Rust forms](rust-forms.md) for primitives, collections, borrowing and library types.".format(len(found)), ""]
    for group in groups:
        lines.extend(["## " + group["module"], "", "| Type and meaning | Created by | Valid states / boundary | Find it in the app | Mistake made harder |", "| --- | --- | --- | --- | --- |"])
        for name, meaning in group["types"].items():
            path, line = found[group["module"]+"::"+name]
            source = "../" + path.relative_to(ROOT).as_posix() + "#L" + str(line)
            values = ["[{}]({}): {}".format(name,source,meaning),group["creator"],group["states"],group["ui"],group["prevents"]]
            lines.append("| " + " | ".join(v.replace("|","\\|").replace("\n"," ") for v in values) + " |")
        lines.append("")
    lines.extend(["## Implemented versus future work", "", "All names above are implemented in this revision. More REA operations, broader macOS live-tool validation, Linux desktop qualification, general firmware format support, recovered runtime behavior and persistent graph positions are not supplied by these types. No planned architecture is listed as a working capability.", ""])
    return "\n".join(lines), len(found)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--write",action="store_true")
    args = parser.parse_args()
    text, count = render()
    destination = ROOT / "docs/type-catalogue.md"
    if args.write:
        destination.write_text(text,encoding="utf-8")
    elif not destination.exists() or destination.read_text(encoding="utf-8") != text:
        raise AssertionError("Generated catalogue is stale; run with --write")
    print("Catalogue covers {} implemented named types; no missing/stale declarations".format(count))

if __name__ == "__main__":
    main()
