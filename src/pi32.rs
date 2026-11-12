//! Bounded vendor-tool observations and conservative static flow derivation.
//! The tool profile is pinned to known-label fixtures, not mnemonic guesses.
use crate::{
    coordinates::{ImageMap, ImageOffset, ImageSpan},
    decoded_image::DecodedImage,
    domain::MappedAddress,
    fwsc::{self, SourceSpan},
    tool_runner::{self, ToolCapture, ToolConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::atomic::AtomicBool,
};

pub const MAX_SELECTION: u64 = 4096;
pub const VALIDATED_TOOL_SHA256: &str =
    "f7e449e2c8001d5ed4493a45d1f5f304c8910418e4074acdc8715079ef64e08a";

/// Monotonic within an investigation. A UI generation separately rejects stale jobs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Target {
    Resolved(MappedAddress),
    Unresolved { operand: String, reason: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    Sequential,
    Jump(Target),
    Conditional { condition: String, target: Target },
    Call(Target),
    Return,
    Unresolved(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecodeKind {
    Instruction,
    Undecoded,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Instruction {
    pub span: ImageSpan,
    pub address: MappedAddress,
    pub source: Vec<SourceSpan>,
    pub bytes: Vec<u8>,
    pub text: String,
    pub continuation: Vec<String>,
    pub kind: DecodeKind,
    pub flow: Flow,
}

impl Instruction {
    pub fn width(&self) -> usize {
        self.bytes.len()
    }
    pub fn mnemonic(&self) -> &str {
        self.text.split_whitespace().next().unwrap_or("?")
    }
    pub fn operands(&self) -> &str {
        self.text
            .split_once(char::is_whitespace)
            .map_or("", |(_, rest)| rest.trim())
    }
    pub fn target(&self) -> Option<&Target> {
        match &self.flow {
            Flow::Jump(t) | Flow::Call(t) | Flow::Conditional { target: t, .. } => Some(t),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gap {
    pub span: ImageSpan,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Coverage {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DisassemblyReport {
    pub id: RunId,
    pub map: ImageMap,
    pub scope: ImageSpan,
    pub input: Vec<u8>,
    pub wrapper_sha256: String,
    pub capture: ToolCapture,
    pub instructions: Vec<Instruction>,
    pub gaps: Vec<Gap>,
    pub coverage: Coverage,
    pub issues: Vec<String>,
}

impl DisassemblyReport {
    pub fn validated_semantics(&self) -> bool {
        validated_capture(&self.capture)
    }

    pub fn decoded_bytes(&self) -> usize {
        self.instructions
            .iter()
            .filter(|i| i.kind == DecodeKind::Instruction)
            .map(Instruction::width)
            .sum()
    }

    pub fn validate(&self) -> Result<(), String> {
        self.map.resolve(&self.scope)?;
        if self.id.0 == 0
            || self.scope.length > MAX_SELECTION
            || self.scope.length != self.input.len() as u64
            || self.capture.stdout.len() > 262_144
            || self.capture.stderr.len() > 262_144
            || self.capture.version.len() > 262_144
            || self.instructions.len() > MAX_SELECTION as usize / 2
            || self.issues.len() > 64
        {
            return Err("Invalid disassembly report size or identity".into());
        }
        let address = self.map.resolve(&self.scope)?.address;
        if fwsc::digest(&elf_wrapper(&self.input, address)?) != self.wrapper_sha256 {
            return Err("Derived ELF identity differs from its recorded input and address".into());
        }
        let parsed = parse_listing(&self.map, &self.scope, &self.input, &self.capture)?;
        if parsed.instructions != self.instructions
            || parsed.gaps != self.gaps
            || parsed.coverage != self.coverage
            || parsed.issues != self.issues
        {
            return Err(
                "Saved instruction/flow projection differs from the retained tool evidence".into(),
            );
        }
        Ok(())
    }
}

pub fn disassemble(
    image: &DecodedImage,
    id: RunId,
    offset: ImageOffset,
    length: u64,
    config: &ToolConfig,
    cancel: &AtomicBool,
) -> Result<DisassemblyReport, String> {
    if id.0 == 0
        || !(2..=MAX_SELECTION).contains(&length)
        || !offset.0.is_multiple_of(2)
        || !length.is_multiple_of(2)
    {
        return Err("Select an even image offset and an even length from 2 to 4096 bytes".into());
    }
    let (scope, bytes) = image.selection(offset, length)?;
    let address = image.map().resolve(&scope)?.address;
    let wrapper = elf_wrapper(bytes, address)?;
    let capture = tool_runner::run(&wrapper, config, cancel)?;
    report_from_capture(id, image.map().clone(), scope, bytes.to_vec(), capture)
}

pub fn report_from_capture(
    id: RunId,
    map: ImageMap,
    scope: ImageSpan,
    input: Vec<u8>,
    capture: ToolCapture,
) -> Result<DisassemblyReport, String> {
    let address = map.resolve(&scope)?.address;
    let wrapper_sha256 = fwsc::digest(&elf_wrapper(&input, address)?);
    let parsed = parse_listing(&map, &scope, &input, &capture)?;
    let report = DisassemblyReport {
        id,
        map,
        scope,
        input,
        wrapper_sha256,
        capture,
        instructions: parsed.instructions,
        gaps: parsed.gaps,
        coverage: parsed.coverage,
        issues: parsed.issues,
    };
    report.validate()?;
    Ok(report)
}

/// Minimal ELF32 with a single executable .text section; never executed.
/// Machine 241 is the installed vendor tool's pi32v2 ELF machine identifier.
pub fn elf_wrapper(bytes: &[u8], address: MappedAddress) -> Result<Vec<u8>, String> {
    if bytes.is_empty()
        || bytes.len() as u64 > MAX_SELECTION
        || address.0 > u64::from(u32::MAX)
        || address
            .0
            .checked_add(bytes.len() as u64)
            .is_none_or(|end| end > u64::from(u32::MAX) + 1)
    {
        return Err("Cannot wrap that bounded pi32v2 address span".into());
    }
    let text_offset = 52usize;
    let names = b"\0.text\0.shstrtab\0";
    let names_offset = text_offset + bytes.len();
    let section_offset = (names_offset + names.len() + 3) & !3;
    let mut elf = vec![0; section_offset + 3 * 40];
    elf[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    put16(&mut elf, 16, 2);
    put16(&mut elf, 18, 241);
    put32(&mut elf, 20, 1);
    put32(&mut elf, 24, address.0 as u32);
    put32(&mut elf, 32, section_offset as u32);
    put16(&mut elf, 40, 52);
    put16(&mut elf, 46, 40);
    put16(&mut elf, 48, 3);
    put16(&mut elf, 50, 2);
    elf[text_offset..names_offset].copy_from_slice(bytes);
    elf[names_offset..names_offset + names.len()].copy_from_slice(names);
    let text = section_offset + 40;
    put32(&mut elf, text, 1);
    put32(&mut elf, text + 4, 1);
    put32(&mut elf, text + 8, 6);
    put32(&mut elf, text + 12, address.0 as u32);
    put32(&mut elf, text + 16, text_offset as u32);
    put32(&mut elf, text + 20, bytes.len() as u32);
    put32(&mut elf, text + 32, 2);
    let strings = section_offset + 80;
    put32(&mut elf, strings, 7);
    put32(&mut elf, strings + 4, 3);
    put32(&mut elf, strings + 16, names_offset as u32);
    put32(&mut elf, strings + 20, names.len() as u32);
    put32(&mut elf, strings + 32, 1);
    Ok(elf)
}

fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
}
fn put32(bytes: &mut [u8], at: usize, value: u32) {
    bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
}

struct Parsed {
    instructions: Vec<Instruction>,
    gaps: Vec<Gap>,
    coverage: Coverage,
    issues: Vec<String>,
}

fn validated_capture(capture: &ToolCapture) -> bool {
    capture.tool_sha256 == VALIDATED_TOOL_SHA256
        && capture.flags
            == [
                "-disassemble",
                "-arch-name=pi32v2",
                "-mcpu=r3",
                "-minstrev3",
                "-print-imm-hex",
                "selection.elf",
            ]
}

fn parse_listing(
    map: &ImageMap,
    scope: &ImageSpan,
    input: &[u8],
    capture: &ToolCapture,
) -> Result<Parsed, String> {
    let start = map.resolve(scope)?.address.0;
    if scope.length != input.len() as u64 || scope.length > MAX_SELECTION {
        return Err("Disassembly input is outside the selected scope".into());
    }
    let end = start + scope.length;
    let mut instructions: Vec<Instruction> = Vec::new();
    let mut issues = Vec::new();
    let mut covered = vec![false; input.len()];
    for line in String::from_utf8_lossy(&capture.stdout).lines() {
        if line.trim() == "}" {
            if let Some(previous) = instructions.last_mut() {
                previous.continuation.push(line.to_owned());
                previous.flow = Flow::Unresolved(
                    "Hardware-loop closing marker: loop semantics are not modeled".into(),
                );
            }
            continue;
        }
        let Some((address_text, tail)) = line.trim_start().split_once(':') else {
            continue;
        };
        if address_text.is_empty() || !address_text.bytes().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(address) = u64::from_str_radix(address_text, 16) else {
            continue;
        };
        let mut rest = tail.trim_start();
        let mut bytes = Vec::new();
        while let Some(word) = rest.split_whitespace().next() {
            if word.len() != 2 || !word.bytes().all(|b| b.is_ascii_hexdigit()) {
                break;
            }
            bytes.push(u8::from_str_radix(word, 16).unwrap());
            rest = rest[word.len()..].trim_start();
        }
        let text = rest.trim().to_owned();
        let width = bytes.len() as u64;
        let valid = matches!(width, 2 | 4 | 6)
            && address % 2 == 0
            && address >= start
            && address.checked_add(width).is_some_and(|a| a <= end)
            && text.len() <= 2048;
        if !valid {
            if issues.len() < 64 {
                issues.push(format!(
                    "Unusable instruction row at 0x{address:X}; retained in raw output"
                ));
            }
            continue;
        }
        let offset = (address - start) as usize;
        if input[offset..offset + bytes.len()] != bytes
            || covered[offset..offset + bytes.len()].iter().any(|c| *c)
            || instructions
                .last()
                .is_some_and(|i| i.address.0 + i.width() as u64 > address)
        {
            if issues.len() < 64 {
                issues.push(format!(
                    "Byte mismatch, overlap or out-of-order row at 0x{address:X}"
                ));
            }
            continue;
        }
        covered[offset..offset + bytes.len()].fill(true);
        let span = map.span(ImageOffset(scope.offset.0 + offset as u64), width)?;
        let location = map.resolve(&span)?;
        let kind = if text.is_empty()
            || text.contains("unknown")
            || text.contains("invalid")
            || text.starts_with('.')
        {
            DecodeKind::Undecoded
        } else {
            DecodeKind::Instruction
        };
        let flow = if kind == DecodeKind::Undecoded {
            Flow::Unresolved("Tool did not decode these bytes as an instruction".into())
        } else {
            classify(&text, address, bytes.len(), validated_capture(capture))
        };
        instructions.push(Instruction {
            span,
            address: location.address,
            source: location.source,
            bytes,
            text,
            continuation: Vec::new(),
            kind,
            flow,
        });
    }
    let mut gaps = Vec::new();
    let mut cursor = 0;
    while cursor < covered.len() {
        if covered[cursor] {
            cursor += 1;
            continue;
        }
        let first = cursor;
        while cursor < covered.len() && !covered[cursor] {
            cursor += 1;
        }
        gaps.push(Gap {
            span: map.span(
                ImageOffset(scope.offset.0 + first as u64),
                (cursor - first) as u64,
            )?,
            bytes: input[first..cursor].to_vec(),
        });
    }
    let count: usize = instructions
        .iter()
        .filter(|i| i.kind == DecodeKind::Instruction)
        .map(Instruction::width)
        .sum();
    let coverage = if count == input.len() {
        Coverage::Complete
    } else if count == 0 {
        Coverage::Unavailable
    } else {
        Coverage::Partial
    };
    Ok(Parsed {
        instructions,
        gaps,
        coverage,
        issues,
    })
}

fn classify(text: &str, address: u64, width: usize, validated: bool) -> Flow {
    if !validated {
        return Flow::Unresolved(
            "This tool/profile has no matching branch-validation fixture".into(),
        );
    }
    if text.starts_with("rep ") || text.contains('{') {
        return Flow::Unresolved(
            "Hardware loop retained; its control flow has not been validated".into(),
        );
    }
    if text == "rts" || text == "pc = [sp++]" {
        return Flow::Return;
    }
    if let Some(operand) = text.strip_prefix("call ") {
        return Flow::Call(relative_target(
            operand,
            address,
            width,
            matches!(width, 2 | 4 | 6),
        ));
    }
    if let Some(operand) = text.strip_prefix("goto ") {
        return Flow::Jump(relative_target(
            operand,
            address,
            width,
            matches!(width, 2 | 4),
        ));
    }
    if text.starts_with("if")
        && let Some((condition, operand)) = text.split_once(" goto ")
    {
        return Flow::Conditional {
            condition: condition.into(),
            target: relative_target(operand, address, width, matches!(width, 2 | 4 | 6)),
        };
    }
    if text.contains("pc")
        || text.contains("goto")
        || text.contains("call")
        || text.starts_with("ret")
        || text.starts_with("trap")
    {
        return Flow::Unresolved("Unmodeled control transfer; no destination is invented".into());
    }
    // Register/memory assignments and nop have a candidate sequential successor.
    // This is static flow, not a statement that this instruction actually executes.
    if text.contains('=') || text == "nop" || text.starts_with("push ") || text.starts_with("pop ")
    {
        Flow::Sequential
    } else {
        Flow::Unresolved(
            "Instruction retained; sequential/control semantics are not modeled".into(),
        )
    }
}

fn relative_target(operand: &str, address: u64, width: usize, supported: bool) -> Target {
    let token = operand.split_whitespace().next().unwrap_or("");
    let unknown = |reason: &str| Target::Unresolved {
        operand: token.into(),
        reason: reason.into(),
    };
    if !supported {
        return unknown("This instruction form/width has not been validated");
    }
    let (negative, unsigned) = token
        .strip_prefix('-')
        .map_or((false, token.trim_start_matches('+')), |s| (true, s));
    let value = if let Some(hex) = unsigned.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).ok()
    } else {
        unsigned.parse::<u64>().ok()
    };
    let Some(value) = value else {
        return unknown("Register/indirect destination requires runtime or data-flow evidence");
    };
    let delta = if negative {
        -(value as i128)
    } else {
        value as i128
    };
    let target = i128::from(address) + width as i128 + delta;
    if !(0..=i128::from(u32::MAX)).contains(&target) {
        return unknown("Relative destination exceeds the 32-bit address space");
    }
    Target::Resolved(MappedAddress(target as u64))
}

#[derive(Clone, Debug)]
pub struct BasicBlock {
    pub first: usize,
    pub end: usize,
    pub address: MappedAddress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Destination {
    Block(usize),
    Outside(MappedAddress),
    Unresolved(String),
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Jump,
    Taken,
    Fallthrough,
    Call,
    ReturnSite,
    Unknown,
}

impl EdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Jump => "jump",
            Self::Taken => "taken",
            Self::Fallthrough => "next",
            Self::Call => "call",
            Self::ReturnSite => "return site",
            Self::Unknown => "unresolved",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FlowEdge {
    pub from: usize,
    pub to: Destination,
    pub kind: EdgeKind,
}

#[derive(Clone, Debug, Default)]
pub struct ControlFlow {
    pub blocks: Vec<BasicBlock>,
    pub edges: Vec<FlowEdge>,
}

pub fn control_flow(report: &DisassemblyReport) -> ControlFlow {
    let rows = &report.instructions;
    if rows.is_empty() {
        return ControlFlow::default();
    }
    let addresses: BTreeMap<u64, usize> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.address.0, index))
        .collect();
    let mut leaders = BTreeSet::from([0]);
    for (index, row) in rows.iter().enumerate() {
        if let Some(Target::Resolved(address)) = row.target()
            && let Some(target) = addresses.get(&address.0)
        {
            leaders.insert(*target);
        }
        if index + 1 < rows.len()
            && (!matches!(row.flow, Flow::Sequential)
                || row.address.0 + row.width() as u64 != rows[index + 1].address.0)
        {
            leaders.insert(index + 1);
        }
    }
    let starts: Vec<usize> = leaders.into_iter().collect();
    let blocks: Vec<BasicBlock> = starts
        .iter()
        .enumerate()
        .map(|(index, first)| BasicBlock {
            first: *first,
            end: starts.get(index + 1).copied().unwrap_or(rows.len()),
            address: rows[*first].address,
        })
        .collect();
    let block_for: BTreeMap<u64, usize> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.address.0, index))
        .collect();
    let selection_start = report.map.mapped_base.0 + report.scope.offset.0;
    let selection_end = selection_start + report.scope.length;
    let destination = |target: &Target| match target {
        Target::Resolved(address) => block_for.get(&address.0).map_or_else(
            || {
                if (selection_start..selection_end).contains(&address.0) {
                    Destination::Unresolved(format!(
                        "0x{:08X}: no decoded instruction boundary",
                        address.0
                    ))
                } else {
                    Destination::Outside(*address)
                }
            },
            |index| Destination::Block(*index),
        ),
        Target::Unresolved { operand, reason } => {
            Destination::Unresolved(format!("{operand}: {reason}"))
        }
    };
    let mut edges = Vec::new();
    for (index, block) in blocks.iter().enumerate() {
        let last = &rows[block.end - 1];
        let mut add = |to, kind| {
            edges.push(FlowEdge {
                from: index,
                to,
                kind,
            })
        };
        let next = || {
            let address = MappedAddress(last.address.0 + last.width() as u64);
            if address.0 > u64::from(u32::MAX) {
                Destination::Unresolved("Next address exceeds the 32-bit address space".into())
            } else {
                destination(&Target::Resolved(address))
            }
        };
        match &last.flow {
            Flow::Sequential => add(next(), EdgeKind::Fallthrough),
            Flow::Jump(t) => add(destination(t), EdgeKind::Jump),
            Flow::Conditional { target, .. } => {
                add(destination(target), EdgeKind::Taken);
                add(next(), EdgeKind::Fallthrough);
            }
            Flow::Call(t) => {
                add(destination(t), EdgeKind::Call);
                add(next(), EdgeKind::ReturnSite);
            }
            Flow::Return => add(Destination::Return, EdgeKind::Jump),
            Flow::Unresolved(reason) => {
                add(Destination::Unresolved(reason.clone()), EdgeKind::Unknown)
            }
        }
    }
    ControlFlow { blocks, edges }
}

pub fn explanation(instruction: &Instruction) -> (&'static str, String, &'static str) {
    match &instruction.flow {
        Flow::Sequential => ("What changes before the next instruction?", format!("The tool decodes `{}`. A sequential edge points to the next byte boundary; actual register and memory values are unknown.", instruction.text), "Inspect the operands and follow where their values are produced or used."),
        Flow::Jump(Target::Resolved(address)) => ("Where does this jump lead?", format!("A direct numeric destination uses PC + instruction length + signed displacement. Result: 0x{:08X}.", address.0), "Inspect the destination bytes and check whether they begin a valid instruction."),
        Flow::Jump(target @ Target::Unresolved { .. }) => ("What would establish this jump's destination?", target_text(target), "Inspect how the target value is produced; retain the unknown until evidence establishes an address."),
        Flow::Conditional { condition, target } => ("What decision splits this path?", format!("{condition}. The taken path is {}; the other candidate is the next instruction. No condition result has been observed.", target_text(target)), "Trace the values tested by the condition before explaining which path is taken."),
        Flow::Call(target) => ("Which routine may be called, and where could it return?", format!("Call destination: {}. The following address is a possible return site; this does not establish that the call returns or executes.", target_text(target)), "Inspect the target and calling register usage before assigning a function name or purpose."),
        Flow::Return => ("Where does this routine return?", "The tool decodes a return. This selected static span does not identify its caller or the runtime return address.".into(), "Find incoming calls or obtain an authorized runtime trace."),
        Flow::Unresolved(reason) => ("What prevents a justified control-flow edge?", reason.clone(), "Keep the raw output and validate this form with a known-label fixture or additional evidence."),
    }
}

pub fn target_text(target: &Target) -> String {
    match target {
        Target::Resolved(address) => format!("0x{:08X}", address.0),
        Target::Unresolved { operand, reason } => format!("{operand} / unresolved: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coordinates::{ArtifactId, ImageId},
        fwsc::PackageOffset,
        tool_runner::ToolOutcome,
    };

    fn capture(stdout: &str) -> ToolCapture {
        ToolCapture {
            executable: "synthetic-objdump".into(),
            tool_sha256: VALIDATED_TOOL_SHA256.into(),
            version: "test fixture".into(),
            flags: [
                "-disassemble",
                "-arch-name=pi32v2",
                "-mcpu=r3",
                "-minstrev3",
                "-print-imm-hex",
                "selection.elf",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
            outcome: ToolOutcome::Completed,
            exit_code: Some(0),
            elapsed_ms: 1,
            limitations: Vec::new(),
        }
    }

    fn fixture(input: &[u8], stdout: &str) -> DisassemblyReport {
        let map = ImageMap::new(
            ArtifactId::from_sha256(&"a".repeat(64)).unwrap(),
            ImageId::from_sha256(&fwsc::digest(input)).unwrap(),
            input.len() as u64,
            2048,
            PackageOffset(1024),
            MappedAddress(0x2000120),
        )
        .unwrap();
        let scope = map.span(ImageOffset(0), input.len() as u64).unwrap();
        report_from_capture(RunId(1), map, scope, input.to_vec(), capture(stdout)).unwrap()
    }

    #[test]
    fn relative_targets_use_end_of_variable_width_instruction() {
        assert_eq!(
            relative_target("-0x6", 0x200012e, 4, true),
            Target::Resolved(MappedAddress(0x200012c))
        );
        assert_eq!(
            relative_target("0x10345552", 0x2000120, 6, true),
            Target::Resolved(MappedAddress(0x12345678))
        );
        assert!(matches!(
            relative_target("r2", 0x2000120, 2, true),
            Target::Unresolved { .. }
        ));
        assert!(matches!(
            relative_target("-0x8", 0, 2, true),
            Target::Unresolved { .. }
        ));
        assert!(matches!(
            classify("goto 0xa", 0x2000120, 6, true),
            Flow::Jump(Target::Unresolved { .. })
        ));
        assert!(matches!(
            classify("call 0xa", 0x2000120, 2, false),
            Flow::Unresolved(_)
        ));
    }

    #[test]
    fn parsing_preserves_bytes_and_refuses_fabricated_saved_edges() {
        let bytes = [0x04, 0x81, 0x80, 0x00, 0x80, 0x00];
        let report = fixture(
            &bytes,
            "2000120: 04 81 goto 0x2\n2000122: 80 00 rts\n2000124: 80 00 rts\n",
        );
        assert_eq!(report.coverage, Coverage::Complete);
        assert_eq!(report.instructions[0].source[0].offset.0, 1044);
        let graph = control_flow(&report);
        assert_eq!(graph.blocks.len(), 3);
        assert_eq!(graph.edges[0].to, Destination::Block(2));
        let mut changed = report.clone();
        changed.instructions[0].flow = Flow::Return;
        assert!(changed.validate().is_err());
        let mut changed = report;
        changed.input[0] ^= 1;
        assert!(changed.validate().is_err());
    }

    #[test]
    fn missing_mismatched_and_loop_lines_remain_visible_unknowns() {
        let bytes = [0x10, 0x03, 0x80, 0x00, 0xff, 0xff];
        let report = fixture(
            &bytes,
            "2000120: 10 03 rep 0x2 r0 {\n2000122: 80 00 rts\n    }\n2000124: 00 00 nop\n",
        );
        assert_eq!(report.coverage, Coverage::Partial);
        assert_eq!(report.gaps[0].bytes, [0xff, 0xff]);
        assert_eq!(report.instructions[1].continuation, ["    }"]);
        assert!(matches!(report.instructions[1].flow, Flow::Unresolved(_)));
        assert!(!report.issues.is_empty());
    }

    #[test]
    fn selected_data_is_wrapped_without_executable_payload_side_effects() {
        let elf = elf_wrapper(&[0x80, 0], MappedAddress(0x2000120)).unwrap();
        assert_eq!(&elf[..7], b"\x7fELF\x01\x01\x01");
        assert_eq!(&elf[52..54], &[0x80, 0]);
        assert_eq!(u16::from_le_bytes(elf[18..20].try_into().unwrap()), 241);
        assert!(elf_wrapper(&[0; 4], MappedAddress(u32::MAX as u64)).is_err());
    }

    #[test]
    fn odd_resynchronization_conflicting_flags_and_address_end_do_not_create_edges() {
        let report = fixture(&[0xff, 0x80, 0x00, 0xff], "2000121: 80 00 rts\n");
        assert!(report.instructions.is_empty());
        let mut conflicting = capture("2000120: 80 00 rts\n");
        conflicting.flags.push("-mcpu=other".into());
        assert!(!validated_capture(&conflicting));
        let input = [0, 0];
        let map = ImageMap::new(
            ArtifactId::from_sha256(&"a".repeat(64)).unwrap(),
            ImageId::from_sha256(&fwsc::digest(&input)).unwrap(),
            2,
            2048,
            PackageOffset(1024),
            MappedAddress(0xffff_fffe),
        )
        .unwrap();
        let scope = map.span(ImageOffset(0), 2).unwrap();
        let report = report_from_capture(
            RunId(1),
            map,
            scope,
            input.to_vec(),
            capture("fffffffe: 00 00 nop\n"),
        )
        .unwrap();
        assert!(matches!(
            control_flow(&report).edges[0].to,
            Destination::Unresolved(_)
        ));
    }
}
