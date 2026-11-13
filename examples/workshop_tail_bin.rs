//! Reproduce the tail.bin panel from a locally supplied, exact FM-1 V15 file.
//! cargo run --offline --example workshop_tail_bin -- path/to/FM-1_015.fwsc
use divulge::{
    evidence::{ByteOrder, F32Bits, ScalarViews},
    fwsc::{self, PackageOffset},
};

// section: checked_read
fn read_u16(bytes: &[u8], start: usize) -> Result<u16, &'static str> {
    let end = start.checked_add(2).ok_or("Offset overflow")?;
    let pair = bytes.get(start..end).ok_or("Need two bytes")?;
    Ok(u16::from_le_bytes([pair[0], pair[1]]))
}
// endsection

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // section: load
    let path = std::env::args_os().nth(1).ok_or("Pass a V15 path")?;
    let raw: Vec<u8> = std::fs::read(path)?;
    let report = fwsc::decode_v15(&raw)?;
    let (slot, entry) = report
        .entries
        .iter()
        .enumerate()
        .find(|(_, entry)| entry.name == "tail.bin")
        .ok_or("tail.bin is missing")?;
    let payload: &[u8] = &entry.preview;
    // endsection

    // section: coordinates
    let package = entry.offset;
    let source = fwsc::file_offset(package)?;
    let end = package
        .0
        .checked_add(u64::from(entry.length))
        .ok_or("Range overflow")?;
    // endsection

    // section: integers
    let little = ScalarViews::from_bytes(payload, 0, ByteOrder::Little);
    let big = ScalarViews::from_bytes(payload, 0, ByteOrder::Big);
    let first: u8 = payload[0];
    let signed: i8 = first as i8;
    // endsection

    let bits = little.u32_value.ok_or("Need four bytes")?;
    let float = F32Bits(bits).value();
    let final_byte = ScalarViews::from_bytes(payload, 63, ByteOrder::Little);
    let header = &entry.decoded_header;
    assert_eq!(read_u16(header, 4)?, entry.checksum.stored);
    assert_eq!(read_u16(header, 79), Err("Need two bytes"));
    assert_eq!(read_u16(header, usize::MAX), Err("Offset overflow"));
    assert_eq!(entry.length as usize, payload.len());
    assert_eq!(fwsc::crc16(payload), 0x8936);
    assert_eq!(fwsc::digest(payload), entry.sha256);
    assert_eq!(source.0, 0xAADF4);
    assert_eq!(little.u16_value, Some(13344));
    assert_eq!(little.u32_value, Some(3591582752));
    assert_eq!(big.u16_value, Some(8244));
    assert_eq!(big.u32_value, Some(540283862));
    assert_eq!(float.to_bits(), 0xD6133420);
    assert!(final_byte.u16_value.is_none());
    assert_eq!(payload[7], 0x2E);

    let record_start = PackageOffset(64 + slot as u64 * 80);
    let output = serde_json::json!({
        "file_length": raw.len(), "file_sha256": fwsc::digest(&raw),
        "package_length": report.header.length, "slot": slot,
        "record_start": record_start.0,
        "entry": entry,
        "header_spans": fwsc::source_spans(record_start, 80, u64::from(report.header.length))?,
        "package_start": package.0, "package_end": end,
        "source_start": source.0,
        "source_spans": fwsc::source_spans(package, u64::from(entry.length), u64::from(report.header.length))?,
        "little": little, "big": big,
        "u8": first, "i8": signed,
        "float": float, "float_display": format!("{float:e}"),
        "numeric_cast": bits as f32,
        "final_byte": final_byte,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
