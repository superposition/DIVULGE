# pi32v2 disassembler evidence

Validated locally on 2026-11-12 using the installed vendor LLVM 4.0.1 tools.
The fixture sources are original, synthetic DIVULGE code. They were assembled,
linked and disassembled, never executed. No firmware or vendor executable is
included in these fixtures.

The disassembler identity is SHA-256
`f7e449e2c8001d5ed4493a45d1f5f304c8910418e4074acdc8715079ef64e08a`.
Compiler, linker and symbol-tool identities are retained in
[`expected.json`](../tests/fixtures/pi32v2/expected.json).

The checked command profile is:

```text
objdump -disassemble -arch-name=pi32v2 -mcpu=r3 -minstrev3 -print-imm-hex input.elf
```

Use the vendor adapter for this profile; its text and flags are not a generic
GNU objdump contract. WSL `--exec` invokes tools without shell startup output.

## Reproduce

From Linux or WSL, with the repository as the current directory:

```sh
python3 tests/fixtures/pi32v2/validate.py \
  --tool-dir /path/to/pi32v2/bin \
  --output target/pi32-validation/run
```

The script captures exact stdout/stderr for every assembler, linker, symbol and
disassembler command in the ignored output directory. It records arguments,
exit codes, tool identities, section hashes and the symbol oracle in
`observations.json`. Its negative cases deliberately exercise malformed input
in the disassembler process; they do not run generated code.

## Checked target convention

Known labels and linker-defined symbols provide an independent target oracle.
The same fixtures were linked at `0x02000120` and `0x02100340`, then also wrapped
as minimal ELF32 carriers without symbols or relocations. All emitted bytes
matched the original `.text` sections. Twenty-eight instruction/target pairs
passed at the two bases, in both symbol-bearing and symbol-free output.

For the tested numeric branch forms:

```text
target = instruction address + actual byte width + signed displayed operand
```

The operand is a byte displacement. It is not an absolute address or a word
count. Preserve its sign; do not apply a second scale factor.

| Fixture | Checked forms |
| --- | --- |
| `flow.s` | Forward/backward 2-byte `goto` and `call`, a 2-byte equality branch, a 4-byte signed comparison branch, 2/4/6-byte instruction widths |
| `far.s` | Forward/backward 4-byte `goto` and `call`, forward/backward 6-byte conditional branches |
| `wide_call.s` | Forward/backward 6-byte `callx` source instructions, which objdump prints as `call` |
| `loops.s` | Register/immediate `rep` instructions and unaddressed closing-brace continuation lines |

For example, the 4-byte instruction at `0x0200012E` in `flow.s` prints
`ifs (r1 > 0x0) goto -0x6`; its target is `0x0200012C`, matching the known label.
The 6-byte call at `0x02000120` in `wide_call.s` prints `call 0xfffffa`; its
target is the linker-defined symbol `0x03000120`.

The checked text convention is evidence about this tool/profile. It does not
establish that a firmware span is executable code or that a branch ran.
The sampled conditional forms do not validate every condition's meaning.

## Retained unknowns and malformed data

`call r2` and `goto r3` are indirect transfers; a register value is not supplied
by disassembly. `rts` and `pc = [sp++]` were observed return forms, but their
runtime destinations are unknown. Hardware-loop counts/control edges and other
untested transfer forms remain unresolved. A `}` line has neither an address nor
bytes and must not become a fabricated instruction.

The negative cases exposed important limitations of this old vendor tool:

- `ff ff` emits two one-byte `<unknown instruction>` rows, with exit code zero.
- `80 ff 00`, a truncated six-byte instruction, emits an unknown byte followed
  by `sti r15` at an odd offset. Successful decoding afterward does not prove
  alignment was recovered.
- A one-byte section `80` crashed with SIGSEGV after emitting an oversized byte
  line that extended outside the section. Raw crash output remains local and is
  excluded from committed fixtures.
- `00 00` is emitted as a two-byte `nop` in the valid control case.

The adapter must bound output and execution time, validate each row's address,
length and exact bytes against the selected span, retain partial/failure state,
and distinguish unknown bytes from instructions. An exit code of zero is not
proof of complete coverage. Never turn a post-gap apparent instruction or an
unvalidated target into an unqualified control-flow claim.
