#!/usr/bin/env python3
"""Compile original synthetic fixtures; compare disassembly with linker symbols.

Uses a caller-supplied vendor toolchain. No firmware or generated program is run.
All outputs go to --output; no vendor binaries are copied into the repository.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import struct
import subprocess

FLAGS = ['-disassemble', '-arch-name=pi32v2', '-mcpu=r3', '-minstrev3', '-print-imm-hex']
PINNED_OBJDUMP = 'f7e449e2c8001d5ed4493a45d1f5f304c8910418e4074acdc8715079ef64e08a'
BASES = [0x02000120, 0x02100340]
ROW = re.compile(r'^\s*([0-9a-f]+):\s*((?:[0-9a-f]{2}\s+)+)\s*(.*?)\s*$', re.I)
BRANCH = re.compile(r'\b(?:goto|call)\s+(-?0x[0-9a-f]+)\b', re.I)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def rows(output):
    parsed = []
    for line in output.splitlines():
        match = ROW.match(line)
        if match:
            data = bytes.fromhex(match[2])
            parsed.append({'address': int(match[1], 16), 'bytes': data.hex(),
                           'width': len(data), 'text': match[3]})
    return parsed


def text_section(path):
    data = path.read_bytes()
    header = struct.unpack_from('<16sHHIIIIIHHHHHH', data)
    shoff, shentsize, shnum, shstrndx = header[6], header[11], header[12], header[13]
    sections = [struct.unpack_from('<10I', data, shoff + index * shentsize) for index in range(shnum)]
    names_header = sections[shstrndx]
    names = data[names_header[4]:names_header[4] + names_header[5]]
    for section in sections:
        if names[section[0]:].split(b'\0')[0] == b'.text':
            return data[section[4]:section[4] + section[5]]
    raise AssertionError('Fixture ELF has no .text section')


def minimal_elf(code, base):
    """Independent tiny ELF carrier with no symbols or relocations, matching app input."""
    names = b'\0.text\0.shstrtab\0'
    text_at = 0x100
    names_at = text_at + len(code)
    shoff = (names_at + len(names) + 3) & ~3
    ident = b'\x7fELF\x01\x01\x01' + b'\0' * 9
    header = struct.pack('<16sHHIIIIIHHHHHH', ident, 2, 241, 1, base, 0, shoff, 0,
                         52, 0, 0, 40, 3, 2)
    data = bytearray(shoff + 120)
    data[:52] = header
    data[text_at:text_at + len(code)] = code
    data[names_at:names_at + len(names)] = names
    struct.pack_into('<10I', data, shoff + 40, 1, 1, 6, base, text_at, len(code), 0, 0, 2, 0)
    struct.pack_into('<10I', data, shoff + 80, 7, 3, 0, 0, names_at, len(names), 0, 0, 1, 0)
    return bytes(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tool-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    tool_dir = args.tool_dir.resolve()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).resolve().parent
    commands = []

    def run(label, tool, arguments, expect_success=True):
        command = [str(tool_dir / tool), *map(str, arguments)]
        process = subprocess.run(command, cwd=output, capture_output=True, text=True, timeout=15)
        (output / f'{label}.stdout').write_text(process.stdout, encoding='utf-8')
        (output / f'{label}.stderr').write_text(process.stderr, encoding='utf-8')
        commands.append({'label': label, 'tool': tool, 'arguments': list(map(str, arguments)),
                         'exit_code': process.returncode, 'stdout': f'{label}.stdout',
                         'stderr': f'{label}.stderr'})
        if expect_success and process.returncode:
            raise AssertionError(f'{label}: exit {process.returncode}: {process.stderr}')
        return process

    identity = {name: digest((tool_dir / name).read_bytes()) for name in ['objdump', 'clang', 'ld', 'nm']}
    if identity['objdump'] != PINNED_OBJDUMP:
        raise AssertionError('Different objdump identity: review results before updating the profile')
    version = run('objdump-version', 'objdump', ['--version']).stdout
    evidence = {'tool_sha256': identity, 'version': version, 'flags': FLAGS,
                'bases': BASES, 'fixtures': [], 'negative_cases': []}
    expected = {
        'flow': [('forward_goto', 'forward_target', 2), ('forward_cond', 'call_forward', 2),
                 ('backward_cond', 'forward_target', 4), ('backward_goto', '_start', 2),
                 ('call_forward', 'call_target', 2), ('call_backward', '_start', 2)],
        'far': [('far_call', 'far_forward', 4), ('far_goto', 'far_forward', 4),
                ('far_if', 'far_forward', 6), ('far_back_if', 'far_backward', 6), ('far_back_call', 'far_backward', 4),
                ('far_back_goto', 'far_backward', 4)],
        'wide_call': [('forward_callx', 'far_forward', 6), ('backward_callx', 'far_backward', 6)],
        'loops': [],
    }
    for fixture, checks in expected.items():
        shutil.copyfile(source / f'{fixture}.s', output / f'{fixture}.s')
        run(f'{fixture}-assemble', 'clang', ['-target', 'pi32v2', '-integrated-as', '-mcpu=r3',
                                            '-c', f'{fixture}.s', '-o', f'{fixture}.o'])
        for base in BASES:
            prefix = f'{fixture}-{base:08x}'
            defines = []
            if fixture in ['far', 'wide_call']:
                distance = 0x10000 if fixture == 'far' else 0x1000000
                backward_distance = 0xff00 if fixture == 'far' else distance
                defines = [f'--defsym=far_forward={base + distance:#x}',
                           f'--defsym=far_backward={base - backward_distance:#x}']
            run(prefix + '-link', 'ld', [f'-Ttext={base:#x}', '-e', '_start', *defines,
                                         f'{fixture}.o', '-o', prefix + '.elf'])
            symbols_text = run(prefix + '-symbols', 'nm', ['-n', prefix + '.elf']).stdout
            symbols = {parts[2]: int(parts[0], 16) for line in symbols_text.splitlines()
                       if len(parts := line.split()) == 3}
            code = text_section(output / (prefix + '.elf'))
            (output / (prefix + '-raw.elf')).write_bytes(minimal_elf(code, base))
            listings = {}
            for kind, filename in [('symbols', prefix + '.elf'), ('raw', prefix + '-raw.elf')]:
                listing = run(prefix + '-' + kind, 'objdump', [*FLAGS, filename]).stdout
                decoded = rows(listing)
                listings[kind] = decoded
                assert bytes.fromhex(''.join(row['bytes'] for row in decoded)) == code
                by_address = {row['address']: row for row in decoded}
                for instruction, target, width in checks:
                    row = by_address[symbols[instruction]]
                    assert row['width'] == width, (prefix, instruction, row)
                    match = BRANCH.search(row['text'])
                    assert match, (prefix, instruction, row)
                    calculated = row['address'] + row['width'] + int(match[1], 16)
                    assert calculated == symbols[target], (prefix, instruction, calculated, symbols[target])
                if fixture == 'flow':
                    assert by_address[symbols['immediate_six']]['width'] == 6
                    assert by_address[symbols['normal_four']]['width'] == 4
                    assert by_address[symbols['indirect_call']]['text'].strip() == 'call r2'
                    assert by_address[symbols['indirect_goto']]['text'].strip() == 'goto r3'
                    assert by_address[symbols['stack_return']]['text'].strip() == 'pc = [sp++]'
                    assert by_address[symbols['plain_return']]['text'].strip() == 'rts'
                if fixture == 'loops':
                    assert sum(line.strip() == '}' for line in listing.splitlines()) == 2
                    assert any(row['text'].startswith('rep ') for row in decoded)
            evidence['fixtures'].append({'name': fixture, 'base': base, 'text_sha256': digest(code),
                'text_hex': code.hex(), 'symbols': symbols, 'rows': listings,
                'checked_targets': [{'source': symbols[a], 'target': symbols[b], 'width': w}
                                    for a, b, w in checks]})
    for name, code in [('one-byte', b'\x80'), ('truncated-six', b'\x80\xff\x00'),
                       ('unknown', b'\xff\xff'), ('zero-pair', b'\x00\x00')]:
        (output / f'{name}.elf').write_bytes(minimal_elf(code, BASES[0]))
        process = run(name, 'objdump', [*FLAGS, name + '.elf'], expect_success=False)
        evidence['negative_cases'].append({'name': name, 'bytes': code.hex(), 'exit_code': process.returncode,
                                          'stdout': process.stdout, 'stderr': process.stderr})
    evidence['commands'] = commands
    (output / 'observations.json').write_text(json.dumps(evidence, indent=2) + '\n', encoding='utf-8')
    print(f'Validated {sum(len(f["checked_targets"]) for f in evidence["fixtures"])} symbol targets '
          f'at {len(BASES)} bases, with and without symbols. All synthetic .text bytes match output.')
    print('Indirect transfers, hardware-loop control and untested forms remain unresolved.')
    for case in evidence['negative_cases']:
        print(case['name'], 'exit', case['exit_code'], 'stdout bytes', len(case['stdout']),
              'stderr bytes', len(case['stderr']))


if __name__ == '__main__':
    main()
