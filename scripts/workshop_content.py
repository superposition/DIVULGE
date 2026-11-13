"""Audience-facing code and spoken explanations shared by the PDF and notes."""
import json
import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parents[1]


def declaration(path, name):
    source = (ROOT / path).read_text(encoding="utf-8")
    match = re.search(r"pub (?:struct|enum|fn) " + re.escape(name) + r"\b", source)
    if not match:
        raise ValueError("Missing declaration: " + name)
    start = match.start()
    brace = source.index("{", start)
    depth = 1
    end = brace + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return "\n".join(line for line in source[start:end].splitlines()
                     if not line.lstrip().startswith(("#[", "///")))


def prose(text):
    return {"kind": "prose", "text": text}


def code(text, language="rust"):
    return {"kind": "code", "text": text, "language": language}


def source(path, label=None):
    return {"kind": "source", "path": path, "label": label or path}


def picture(file, crop, width, caption):
    return {"kind": "image", "file": file, "crop": crop, "width": width, "caption": caption}


def chapter(title, topic, *blocks):
    return {"title": title, "topic": topic, "blocks": list(blocks)}


def chapters():
    pages = [chapter("From bytes to evidence", "Rust and reverse engineering",
        prose("What can we learn from two bytes? We can read them, give them a location, and choose an interpretation. Each of those steps gives us a different kind of information."),
        code("let bytes: [u8; 2] = [0x34, 0x12];\nlet value = u16::from_le_bytes(bytes);\n\nassert_eq!(value, 4660);"),
        prose("We know exactly how this calculation produced 4660. We still need evidence to say what that number represents. It could be a length, a parameter, part of an instruction, or something else."),
        code("question\n   -> probe\n   -> observation\n   -> explanation\n   -> next question", "text"),
        prose("DIVULGE gives each step a record. Rust gives those records a shape. The graph shows how they connect, and the right-hand pane lets us inspect the bytes and the explanation together."),
        prose("We will begin with the synthetic bytes 34 12, then follow the same ideas into the FM-1 V15 firmware and its pi32v2 instructions."),
        source("examples/workshop_01_bytes.rs"), source("src/domain.rs")),
    chapter("Give the question a shape", "Questions and purpose",
        prose("What makes a question useful? We need to know why it matters, how we can investigate it, and what an answer would let us do."),
        code(declaration("src/domain.rs", "QuestionPlan")),
        prose("A struct brings related values together. Here, each field gives the question a job. The steps are a Vec<String> because a question can need several actions."),
        code('QuestionPlan {\n    why: "Choose how to read this field".into(),\n    steps: vec!["Compare both byte orders".into()],\n    enables: "Choose the next observation".into(),\n    answered_when: "A format defines the field".into(),\n    if_not: "Keep both interpretations open".into(),\n}'),
        prose("The question stays attached to the investigation. When we return to a result, we can see what we were trying to establish and which part remains unanswered."),
        source("src/domain.rs"), source("docs/domain.md")),
    chapter("Keep the explanation inspectable", "Harness Insight",
        prose("How do we make an investigation understandable while it is happening? We keep the question, the action, the observation and the explanation in named fields."),
        code(declaration("src/domain.rs", "HarnessInsight")),
        prose("String gives each field owned text. The names matter as much as the container: observation describes what we saw; explanation describes what we make of it."),
        code('observation: "The bytes are 34 12"\nexplanation: "Little-endian u16 gives 4660"\nuncertainty: "The field meaning is unknown"\nnext_step:   "Find a format declaration"', "text"),
        prose("These are the sections we see in Harness Insight. A useful explanation carries its uncertainty and a next step with it, so we can question the claim without losing the evidence."),
        source("src/domain.rs"), source("src/insight.rs"))]

    lessons = json.loads((ROOT / "workshop/lessons.json").read_text(encoding="utf-8"))["lessons"]
    explanations = [
        "u8 describes one byte. [u8; 2] says the array has exactly two elements. &[u8] borrows a view of those elements. When we call from_le_bytes or from_be_bytes, we choose a rule for combining the bytes into a u16. The bytes themselves stay the same.",
        "Observation keeps the file location, the bytes and a description together. Vec<u8> owns the byte sequence; String owns the description. We can change the wording while preserving the exact bytes we observed. Rust is describing our evidence record here, not reconstructing a struct from the firmware.",
        "RecordData is an enum: a record has one of several shapes. match lets us handle those shapes explicitly. A question asks for evidence, a probe requests an action, and an explanation cites what supports it. The wildcard covers the other variants implemented by DIVULGE.",
        "Result tells us whether an operation succeeded. Option tells us whether a value is present. A failed two-byte read has an error to explain. An unknown address has no value yet. Zero would be a specific address, so we cannot use it to mean that we do not know.",
        "The function count borrows &[u8], so the Vec remains available afterwards. FileOffset and MappedAddress both contain u64 values, but Rust treats them as different types. A file position and a CPU address need an explicit mapping before we can relate them.",
        "RecordId identifies a record. The explanation holds ObservationId values, which we resolve against the investigation to check that they point to completed evidence. The ? operator returns an error if that lookup fails. Once the link resolves, we can inspect the original observation and challenge the explanation."
    ]
    connections = [
        "In Harness Insight, changing the endian selector changes 4660 to 13330. The observed bytes remain 34 12.",
        "Observation 3 shows file+0 and the same two hex cells. Its mapped address remains unresolved because this fixture has no load map.",
        "The graph uses these kinds as card labels. Unknown 5 keeps a question open; question 6 asks what evidence could establish the field's meaning.",
        "Selecting the last byte, 12, gives u8=18. The u16 view becomes unavailable because only one byte remains in the observed range.",
        "The inspector keeps file offsets, decoded-image offsets and declared addresses visible as separate coordinates. Their numeric values can differ even when they refer to the same instruction.",
        "In the graph, explanation 4 links to observation 3. Following that link brings us back to the bytes. The integer view is reproducible; the intended meaning is still open."
    ]
    for index, lesson in enumerate(lessons):
        path = "examples/" + lesson["slug"] + ".rs"
        rust = "\n".join(line for line in (ROOT / path).read_text(encoding="utf-8").splitlines()
                         if not line.startswith("//!"))
        output = lesson["stdout"] if index != 5 else "\n".join(lesson["stdout"].splitlines()[:2])
        pages.append(chapter(lesson["title"], "Rust / " + str(index + 1),
            prose(lesson["ask"]), code(rust), code(output, "output excerpt" if index == 5 else "output"),
            prose(explanations[index]), prose(connections[index]), source(path)))

    pages.extend([
    chapter("Let the compiler catch the mix-up", "Coordinate types",
        prose("What happens if we give a CPU address to a function that reads a file offset?"),
        code("use divulge::domain::{MappedAddress, read_u16_le};\n\nfn main() {\n    let bytes = [0x34, 0x12];\n    let address = MappedAddress(0x0200_0120);\n    println!(\"{:?}\", read_u16_le(&bytes, address));\n}"),
        code("error[E0308]: mismatched types\nexpected `FileOffset`, found `MappedAddress`", "output"),
        prose("The function's signature tells us which coordinate it accepts. For this buffer, we know the first byte is at file offset zero."),
        code("use divulge::domain::{FileOffset, read_u16_le};\n\nfn main() {\n    let bytes = [0x34, 0x12];\n    println!(\"{:?}\", read_u16_le(&bytes, FileOffset(0)));\n}"),
        code("Ok(4660)", "output"),
        prose("Wrapping the CPU address in FileOffset would make the compiler accept its type, but it would not establish a correct file location. The mapping is a separate fact that we have to obtain."),
        source("workshop/exercises/wrong_coordinate.rs"), source("workshop/exercises/coordinate_answer.rs")),
    chapter("Make the byte reader say what failed", "Result and the question mark operator",
        prose("The type check protects the coordinate space. We also need to check whether the requested bytes exist. This is the reader used by the examples."),
        code(declaration("src/domain.rs", "read_u16_le")),
        prose("First we convert the offset to an index for this platform. checked_add catches arithmetic overflow. get checks whether the complete range is present. Each ? passes a failure back to the caller."),
        code('read_u16_le(&[0x34, 0x12], FileOffset(0))\n// Ok(4660)\n\nread_u16_le(&[0x34], FileOffset(0))\n// Err("Two bytes are required")'),
        prose("Only after those checks do we construct the u16. The function can explain why a read failed, and the rest of the program can keep that failure separate from a successful observation."),
        source("src/domain.rs"), source("docs/rust-forms.md")),
    chapter("Connect records without merging them", "The evidence graph",
        prose("An observation and an explanation need separate identities. An edge tells us how they are related."),
        code(declaration("src/domain.rs", "Edge")),
        code("Edge {\n    from: RecordId(3),\n    to: RecordId(4),\n    relation: Relation::Supports,\n}"),
        picture("synthetic-evidence.jpg", (265, 308, 844, 700), 330,
                "Observation 3 connects the byte-reading probe to explanation 4."),
        prose("The graph draws this relationship as an arrow. Moving a card changes its position on screen; the IDs and the Supports relationship stay the same. We can also add Revises or Contradicts edges without erasing the original claim."),
        source("src/domain.rs"), source("src/graph.rs")),
    chapter("Follow one FM-1 instruction", "V15 and pi32v2",
        prose("The first decoded V15 instruction gives us a concrete coordinate problem. These values describe the same two bytes in different spaces."),
        code("file+0x00004534       original firmware file\napp.bin+0x00000000    decoded application image\n0x02000120           declared CPU address\n\n04 81                observed instruction bytes\ngoto 0x2             decoded instruction", "text"),
        prose("For this direct jump, the validated pi32v2 profile adds the signed displacement to the address after the instruction."),
        code("let pc: u64 = 0x0200_0120;\nlet width: u64 = 2;\nlet displacement: i64 = 2;\nlet target = (pc + width).checked_add_signed(displacement);\n\nassert_eq!(target, Some(0x0200_0124));"),
        picture("v15-code-large.jpg", (1505, 169, 1908, 473), 255,
                "The instruction inspector retains the bytes, coordinates and candidate destination."),
        prose("That destination tells us where to inspect next. It does not tell us whether the device takes the branch, what a register contains, or which function this belongs to."),
        source("src/pi32.rs"), source("src/coordinates.rs")),
    chapter("Represent what the decoder knows", "Instruction data",
        prose("What should a decoded instruction contain? We keep its location and original bytes alongside the text and the control-flow interpretation."),
        code(declaration("src/pi32.rs", "Instruction")),
        code(declaration("src/pi32.rs", "Target")),
        prose("Vec<SourceSpan> preserves the connection to the original file. Flow describes the kind of transfer. Target can hold a resolved address or retain the operand and the reason it remains unresolved."),
        code("Target::Unresolved {\n    operand: \"r3\".into(),\n    reason: \"The register value is unknown\".into(),\n}"),
        prose("An unresolved transfer still gives us useful data: the exact operand and the missing fact. That is enough to ask a more precise next question."),
        source("src/pi32.rs"), source("src/fwsc.rs")),
    chapter("Give AI a proposal, then check it", "Bounded investigation",
        prose("How does a model suggest the next step without becoming the authority over the bytes? Its response becomes a Proposal with fields the Rust program can inspect."),
        code(declaration("src/omp.rs", "Proposal")),
        prose("The evidence IDs connect the suggestion to existing observations. The operation, offset and length describe the requested action. Why, method, enables and unknowns explain the purpose and the remaining uncertainty."),
        code("model proposal\n   -> validate evidence IDs\n   -> check operation and byte range\n   -> check the remaining budget\n   -> run the selected probe\n   -> retain its result\n   -> ask for an explanation", "text"),
        prose("Rust owns this sequence. A suggestion outside the permitted scope is rejected before a probe runs. The next explanation can then cite the observation that the probe actually produced."),
        source("src/omp.rs"), source("src/execution.rs")),
    chapter("Preserve the answer from the tool", "REA evidence",
        prose("REA supplies evidence through its own operations. DIVULGE keeps the request and the receipt together so we can return to the operation that produced a result."),
        code(declaration("src/rea.rs", "Request")),
        code(declaration("src/rea.rs", "ReaRun")),
        prose("The Entropy request names the artifact and selects a byte range. Its receipt retains REA's evidence identity, provider, result and limitations. ImportBundle brings an existing evidence bundle into the same review process."),
        code("artifact identity + selected range\n   -> entropy observation\n   -> evidence ID + result + limitations", "text"),
        prose("Entropy describes a byte distribution. A high value alone cannot tell us that the data is encrypted. We need another observation to distinguish encryption, compression and other explanations. DIVULGE's package reader and vendor disassembler provide different kinds of evidence."),
        source("src/rea.rs"), source("docs/architecture.md")),
    chapter("Keep progress separate from reading", "Execution and history",
        prose("An investigation can advance while we are still reading an earlier result. We need one state for execution and another for the record on screen."),
        code(declaration("src/execution.rs", "Mode")),
        prose("Pause and Stop affect execution. Presenter hold keeps the current record in view. Review next and Follow latest let us decide when to move our attention."),
        code(declaration("src/history.rs", "Document")),
        prose("The saved Document contains a baseline investigation and an ordered Vec<Event>. A correction adds a review record; a note edit becomes a revision. Replaying the history reconstructs those changes without asking a model to do the investigation again."),
        source("src/execution.rs"), source("src/history.rs"), source("docs/harness-memory.md")),
    chapter("Bring the pieces together", "Using DIVULGE",
        prose("We can now follow one value all the way through the program: from its bytes, through a chosen interpretation, to the evidence link that lets someone question it."),
        code("[0x34, 0x12]\n   -> Observation { offset, bytes, description }\n   -> RecordData::Explanation { basis, evidence }\n   -> HarnessInsight { explanation, uncertainty, .. }\n   -> Edge { from, to, relation }", "text"),
        prose("In the workshop investigation, explanation 4 says that a little-endian u16 view gives 4660. Its evidence link leads to observation 3. Choosing big endian gives 13330 from those same bytes. The field's intended meaning remains the question to investigate."),
        code("cargo run --locked --example workshop_01_bytes\ncargo run --locked --example workshop_02_observation\ncargo run --locked --example workshop_03_records\ncargo run --locked --example workshop_04_absence\ncargo run --locked --example workshop_05_coordinates\ncargo run --locked --example workshop_06_evidence", "shell"),
        prose("File > Open investigation loads workshop/fixtures/synthetic-workshop.json. We can select a claim, follow its evidence, compare interpretations, and record what observation would change our mind."),
        prose("That is the question to carry into the firmware: what do we know from the bytes, what are we inferring, and what should we inspect next?"),
        source("workshop/fixtures/synthetic-workshop.json"), source("docs/type-catalogue.md"), source("docs/rust-forms.md"))
    ])
    return pages


def write_notes(pages):
    for name, chosen in [("talk", pages), ("workshop", pages[:12] + [pages[-1]])]:
        lines = ["# DIVULGE: from bytes to evidence", ""]
        for page in chosen:
            lines.extend(["## " + page["title"], ""])
            for block in page["blocks"]:
                kind = block["kind"]
                if kind == "prose":
                    lines.extend([block["text"], ""])
                elif kind == "code":
                    lang = block["language"]
                    if lang == "output excerpt":
                        lines.extend(["Output excerpt:", ""])
                    lines.extend(["```" + ("text" if lang in {"output", "output excerpt", "text"} else lang), block["text"].strip(), "```", ""])
                elif kind == "source":
                    lines.extend(["[" + block["label"] + "](../" + block["path"] + ")", ""])
                elif kind == "image":
                    lines.extend(["![" + block["caption"] + "](../workshop/assets/" + block["file"] + ")", ""])
        (ROOT / "docs" / (name + ".md")).write_text("\n".join(lines), encoding="utf-8")
