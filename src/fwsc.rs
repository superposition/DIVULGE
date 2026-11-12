//! Read-only FM-1 V15 profile. Format/cipher reference: M-VAVE's MIT Package.cs.
//! See THIRD_PARTY.md. Keys and layout constants are supplied by that profile,
//! not recovered by an AI. No target code or vendor updater is executed.
use crate::domain::{FileOffset, MappedAddress, V15_LENGTH, V15_SHA256};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const HEADER_SIZE: usize = 64;
const ENTRY_SIZE: usize = 80;
pub const AREA_START: u64 = 0x4000;
pub const SFC_KEY: u16 = 0x980f;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageOffset(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlashOffset(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub offset: FileOffset,
    pub length: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CrcCheck {
    pub stored: u16,
    pub calculated: u16,
}

impl CrcCheck {
    pub fn matches(self) -> bool {
        self.stored == self.calculated
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageHeader {
    pub length: u32,
    pub count: u16,
    pub format: u16,
    pub block_size: u32,
    pub chip: String,
    pub checksum: CrcCheck,
    pub directory_checksum: CrcCheck,
    pub decoded: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PackageEntry {
    pub kind: u16,
    pub index: u16,
    pub reserved: u16,
    pub offset: PackageOffset,
    pub length: u32,
    pub padded_length: u32,
    pub checksum: CrcCheck,
    pub name: String,
    pub decoded_header: Vec<u8>,
    pub preview: Vec<u8>,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub record_offset: FlashOffset,
    pub header_checksum: CrcCheck,
    pub data_crc: u16,
    /// A declaration whose coordinate semantics depend on the entry kind.
    pub offset: u32,
    pub length: u32,
    pub attributes: u16,
    pub last: u16,
    pub name: String,
    pub decoded: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Application {
    pub area_header: DirectoryEntry,
    pub area_checksum: CrcCheck,
    pub directory: Vec<DirectoryEntry>,
    pub app_index: usize,
    pub checksum: CrcCheck,
    pub preview: Vec<u8>,
    pub sha256: String,
}

impl Application {
    pub fn entry(&self) -> &DirectoryEntry {
        &self.directory[self.app_index]
    }

    pub fn flash_offset(&self) -> FlashOffset {
        FlashOffset(AREA_START + u64::from(self.entry().offset))
    }

    /// The area header declares the application entry address. Static metadata
    /// does not demonstrate runtime execution or instruction boundaries.
    pub fn mapped_base(&self) -> MappedAddress {
        MappedAddress(u64::from(self.area_header.offset))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirmwareView {
    Package,
    Entry(usize),
    Application,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirmwareReport {
    pub header: PackageHeader,
    pub entries: Vec<PackageEntry>,
    pub application: Option<Application>,
    pub application_error: Option<String>,
}

impl FirmwareReport {
    pub fn flash(&self) -> Option<&PackageEntry> {
        self.entries
            .iter()
            .find(|e| e.kind == 0 && e.name == "flash.bin")
    }

    pub fn contains(&self, view: FirmwareView) -> bool {
        match view {
            FirmwareView::Package => true,
            FirmwareView::Entry(index) => index < self.entries.len(),
            FirmwareView::Application => self.application.is_some(),
        }
    }

    /// Validate saved DTO structure; this is not a fresh hash/checksum of a file.
    pub fn validate(&self, file_length: u64) -> Result<(), String> {
        let length = u64::from(self.header.length);
        if length.checked_add(20) != Some(file_length)
            || self.entries.is_empty()
            || self.entries.len() > 64
            || self.entries.len() != usize::from(self.header.count)
            || self.header.decoded.len() != HEADER_SIZE
            || !self.header.checksum.matches()
            || !self.header.directory_checksum.matches()
            || crc16(&self.header.decoded[2..]) != self.header.checksum.stored
        {
            return Err("Invalid saved package header or directory".into());
        }
        for entry in &self.entries {
            checked_range(length, entry.offset.0, u64::from(entry.padded_length))?;
            if entry.length > entry.padded_length
                || entry.decoded_header.len() != ENTRY_SIZE
                || entry.preview.len() != (entry.length as usize).min(256)
                || !valid_digest(&entry.sha256)
                || entry.name.len() > 64
            {
                return Err("Invalid saved package entry".into());
            }
        }
        if let Some(app) = &self.application {
            let flash = self.flash().ok_or("Application has no flash.bin parent")?;
            if app.directory.is_empty()
                || app.directory.len() > 16
                || app.app_index >= app.directory.len()
            {
                return Err("Invalid saved application directory".into());
            }
            for entry in std::iter::once(&app.area_header).chain(&app.directory) {
                checked_range(u64::from(flash.length), entry.record_offset.0, 32)?;
                if entry.decoded.len() != 32
                    || !entry.header_checksum.matches()
                    || crc16(&entry.decoded[2..]) != entry.header_checksum.stored
                    || entry.name.len() > 64
                {
                    return Err("Invalid saved application record".into());
                }
            }
            checked_range(
                u64::from(flash.length),
                AREA_START,
                u64::from(app.area_header.length),
            )?;
            checked_range(
                u64::from(app.area_header.length),
                u64::from(app.entry().offset),
                u64::from(app.entry().length),
            )?;
            if app.area_header.name != "app_area_head"
                || app.entry().name != "app.bin"
                || !app.area_checksum.matches()
                || !app.checksum.matches()
                || app.preview.len() != (app.entry().length as usize).min(256)
                || !valid_digest(&app.sha256)
                || app
                    .mapped_base()
                    .0
                    .checked_add(u64::from(app.entry().length))
                    .is_none()
            {
                return Err("Invalid saved application span".into());
            }
        }
        Ok(())
    }
}

fn valid_digest(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// CRC-16/XMODEM: polynomial 0x1021, initial 0, no reflection or final XOR.
pub fn crc16(bytes: &[u8]) -> u16 {
    let mut value = 0u16;
    for byte in bytes {
        value ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            value = (value << 1) ^ if value & 0x8000 != 0 { 0x1021 } else { 0 };
        }
    }
    value
}

fn cipher(bytes: &mut [u8], mut key: u16) {
    for byte in bytes {
        *byte ^= key as u8;
        key = (key << 1) ^ if key & 0x8000 != 0 { 0x1021 } else { 0 };
    }
}

fn checked_range(total: u64, offset: u64, length: u64) -> Result<std::ops::Range<usize>, String> {
    let end = offset.checked_add(length).ok_or("Byte range overflow")?;
    if end > total || end > usize::MAX as u64 {
        return Err(format!(
            "Range 0x{offset:X}+0x{length:X} exceeds 0x{total:X}"
        ));
    }
    Ok(offset as usize..end as usize)
}

pub fn file_offset(offset: PackageOffset) -> Result<FileOffset, String> {
    offset
        .0
        .checked_add((offset.0 / 47).min(20))
        .map(FileOffset)
        .ok_or_else(|| "Source mapping overflow".into())
}

/// Source spans exclude the 20 interleaved marker bytes. A decoded header can
/// span more than one original file range; it must not be shown as contiguous.
pub fn source_spans(
    offset: PackageOffset,
    length: u64,
    logical_length: u64,
) -> Result<Vec<SourceSpan>, String> {
    checked_range(logical_length, offset.0, length)?;
    let end = offset.0 + length;
    let mut cursor = offset.0;
    let mut spans = Vec::new();
    while cursor < end {
        let next = if cursor < 940 {
            end.min((cursor / 47 + 1) * 47)
        } else {
            end
        };
        spans.push(SourceSpan {
            offset: file_offset(PackageOffset(cursor))?,
            length: next - cursor,
        });
        cursor = next;
    }
    Ok(spans)
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, String> {
    let range = checked_range(bytes.len() as u64, offset as u64, 2)?;
    Ok(u16::from_le_bytes(bytes[range].try_into().unwrap()))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, String> {
    let range = checked_range(bytes.len() as u64, offset as u64, 4)?;
    Ok(u32::from_le_bytes(bytes[range].try_into().unwrap()))
}

fn name(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|b| **b != 0 && **b != 0xff)
        .map(|b| {
            if b.is_ascii_graphic() || *b == b' ' {
                char::from(*b)
            } else {
                '?'
            }
        })
        .collect()
}

/// The public entry point independently gates the supplied bytes, not a saved
/// identity DTO. Unknown firmware still supports generic byte inspection.
pub fn decode_v15(raw: &[u8]) -> Result<FirmwareReport, String> {
    decode_v15_with_image(raw).map(|(report, _)| report)
}

/// A runtime image can only come from a fresh exact-identity read and decode.
/// It is deliberately not deserializable from an investigation file.
pub fn decode_v15_with_image(
    raw: &[u8],
) -> Result<(FirmwareReport, Option<crate::decoded_image::DecodedImage>), String> {
    if raw.len() as u64 != V15_LENGTH || digest(raw) != V15_SHA256 {
        return Err("The V15 profile requires the exact reference length and SHA-256".into());
    }
    let (report, bytes) = decode_package_bytes(raw)?;
    let image = bytes
        .map(|bytes| crate::decoded_image::DecodedImage::from_verified_v15(bytes, &report))
        .transpose()?;
    Ok((report, image))
}

#[cfg(test)]
fn decode_package(raw: &[u8]) -> Result<FirmwareReport, String> {
    decode_package_bytes(raw).map(|(report, _)| report)
}

fn decode_package_bytes(raw: &[u8]) -> Result<(FirmwareReport, Option<Vec<u8>>), String> {
    if raw.len() < 960 {
        return Err("Truncated FWSC marker area".into());
    }
    let logical: Vec<u8> = raw
        .iter()
        .enumerate()
        .filter_map(|(i, b)| (!(i < 960 && i % 48 == 47)).then_some(*b))
        .collect();
    let mut decoded = logical[..HEADER_SIZE].to_vec();
    cipher(&mut decoded, 0xffff);
    let count = u16_at(&decoded, 8)?;
    if count == 0 || count > 64 {
        return Err("Unsupported package entry count".into());
    }
    let directory_range = checked_range(
        logical.len() as u64,
        HEADER_SIZE as u64,
        u64::from(count) * ENTRY_SIZE as u64,
    )?;
    let header = PackageHeader {
        length: u32_at(&decoded, 4)?,
        count,
        format: u16_at(&decoded, 10)?,
        block_size: u32_at(&decoded, 12)?,
        chip: name(&decoded[16..32]),
        checksum: CrcCheck {
            stored: u16_at(&decoded, 0)?,
            calculated: crc16(&decoded[2..]),
        },
        directory_checksum: CrcCheck {
            stored: u16_at(&decoded, 2)?,
            calculated: crc16(&logical[directory_range.clone()]),
        },
        decoded,
    };
    if header.length as usize != logical.len()
        || !header.checksum.matches()
        || !header.directory_checksum.matches()
    {
        return Err(
            "Package length, header CRC, or encrypted-directory CRC did not validate".into(),
        );
    }
    let mut entries = Vec::new();
    for record in logical[directory_range].chunks_exact(ENTRY_SIZE) {
        let mut decoded = record.to_vec();
        cipher(&mut decoded, 0xffff);
        let offset = u64::from(u32_at(&decoded, 8)?);
        let length = u32_at(&decoded, 12)?;
        let padded_length = u32_at(&decoded, 16)?;
        if length > padded_length {
            return Err("Entry length exceeds padded length".into());
        }
        checked_range(logical.len() as u64, offset, u64::from(padded_length))?;
        let range = checked_range(logical.len() as u64, offset, u64::from(length))?;
        let payload = &logical[range];
        entries.push(PackageEntry {
            kind: u16_at(&decoded, 0)?,
            index: u16_at(&decoded, 2)?,
            reserved: u16_at(&decoded, 6)?,
            offset: PackageOffset(offset),
            length,
            padded_length,
            checksum: CrcCheck {
                stored: u16_at(&decoded, 4)?,
                calculated: crc16(payload),
            },
            name: name(&decoded[64..80]),
            decoded_header: decoded,
            preview: payload[..payload.len().min(256)].to_vec(),
            sha256: digest(payload),
        });
    }
    let mut report = FirmwareReport {
        header,
        entries,
        application: None,
        application_error: None,
    };
    let bytes = match decode_application(&logical, &report) {
        Ok((app, bytes)) => {
            report.application = Some(app);
            Some(bytes)
        }
        Err(error) => {
            report.application_error = Some(error);
            None
        }
    };
    report.validate(raw.len() as u64)?;
    Ok((report, bytes))
}

fn directory_entry(bytes: &[u8], offset: FlashOffset) -> Result<DirectoryEntry, String> {
    if bytes.len() != 32 {
        return Err("Truncated JLFS record".into());
    }
    let header_checksum = CrcCheck {
        stored: u16_at(bytes, 0)?,
        calculated: crc16(&bytes[2..]),
    };
    if !header_checksum.matches() {
        return Err(format!(
            "JLFS header CRC mismatch at flash+0x{:X}",
            offset.0
        ));
    }
    Ok(DirectoryEntry {
        record_offset: offset,
        header_checksum,
        data_crc: u16_at(bytes, 2)?,
        offset: u32_at(bytes, 4)?,
        length: u32_at(bytes, 8)?,
        attributes: u16_at(bytes, 12)?,
        last: u16_at(bytes, 14)?,
        name: name(&bytes[16..32]),
        decoded: bytes.to_vec(),
    })
}

fn decode_application(
    logical: &[u8],
    report: &FirmwareReport,
) -> Result<(Application, Vec<u8>), String> {
    let flash = report.flash().ok_or("No flash.bin entry")?;
    if !flash.checksum.matches() {
        return Err("flash.bin raw CRC did not validate".into());
    }
    let range = checked_range(
        logical.len() as u64,
        flash.offset.0,
        u64::from(flash.length),
    )?;
    let flash_bytes = &logical[range];
    let range = checked_range(
        flash_bytes.len() as u64,
        AREA_START,
        flash_bytes.len() as u64 - AREA_START.min(flash_bytes.len() as u64),
    )?;
    let mut area = flash_bytes[range].to_vec();
    for (index, block) in area.chunks_mut(32).enumerate() {
        cipher(block, SFC_KEY ^ (((index * 32) >> 2) as u16));
    }
    let head = area.get(..32).ok_or("Missing application area header")?;
    let area_header = directory_entry(head, FlashOffset(AREA_START))?;
    if area_header.name != "app_area_head" || area_header.length < 64 {
        return Err("Application area signature/length not supported by the V15 profile".into());
    }
    let area_range = checked_range(area.len() as u64, 32, u64::from(area_header.length) - 32)?;
    let area_checksum = CrcCheck {
        stored: area_header.data_crc,
        calculated: crc16(&area[area_range]),
    };
    if !area_checksum.matches() {
        return Err("Application area data CRC mismatch".into());
    }
    // V15's application directory precedes app.bin, whose offset declares its end.
    let first = directory_entry(
        area.get(32..64).ok_or("Missing app.bin record")?,
        FlashOffset(AREA_START + 32),
    )?;
    if first.name != "app.bin" || first.offset < 64 || first.offset > 512 || first.offset % 32 != 0
    {
        return Err("Unsupported application directory boundary".into());
    }
    let directory_range = checked_range(area.len() as u64, 32, u64::from(first.offset) - 32)?;
    let mut directory = Vec::new();
    for (index, bytes) in area[directory_range].chunks_exact(32).enumerate() {
        if bytes.iter().all(|b| *b == 0xff) || bytes.iter().all(|b| *b == 0) {
            continue;
        }
        directory.push(directory_entry(
            bytes,
            FlashOffset(AREA_START + (index as u64 + 1) * 32),
        )?);
    }
    let app_index = directory
        .iter()
        .position(|e| e.name == "app.bin")
        .ok_or("Missing app.bin")?;
    let entry = &directory[app_index];
    let range = checked_range(
        u64::from(area_header.length),
        u64::from(entry.offset),
        u64::from(entry.length),
    )?;
    let payload = &area[range];
    let checksum = CrcCheck {
        stored: entry.data_crc,
        calculated: crc16(payload),
    };
    if !checksum.matches() {
        return Err("app.bin CRC mismatch".into());
    }
    Ok((
        Application {
            area_header,
            area_checksum,
            directory,
            app_index,
            checksum,
            preview: payload[..payload.len().min(256)].to_vec(),
            sha256: digest(payload),
        },
        payload.to_vec(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_standard_vector_and_source_mapping_boundaries() {
        assert_eq!(crc16(b"123456789"), 0x31c3);
        assert_eq!(file_offset(PackageOffset(46)).unwrap(), FileOffset(46));
        assert_eq!(file_offset(PackageOffset(47)).unwrap(), FileOffset(48));
        assert_eq!(file_offset(PackageOffset(940)).unwrap(), FileOffset(960));
        assert_eq!(
            source_spans(PackageOffset(46), 3, 1000).unwrap(),
            vec![
                SourceSpan {
                    offset: FileOffset(46),
                    length: 1
                },
                SourceSpan {
                    offset: FileOffset(48),
                    length: 2
                }
            ]
        );
        assert_eq!(
            source_spans(PackageOffset(939), 3, 1000).unwrap(),
            vec![
                SourceSpan {
                    offset: FileOffset(958),
                    length: 1
                },
                SourceSpan {
                    offset: FileOffset(960),
                    length: 2
                }
            ]
        );
        assert!(source_spans(PackageOffset(999), 2, 1000).is_err());
        assert!(file_offset(PackageOffset(u64::MAX)).is_err());
        assert!(checked_range(u64::MAX, u64::MAX, 1).is_err());
    }

    fn synthetic_package() -> Vec<u8> {
        synthetic_entry(1024, 3, 32)
    }

    fn synthetic_entry(offset: u32, length: u32, padded: u32) -> Vec<u8> {
        let mut logical = vec![0xff; 1056];
        let mut entry = vec![0u8; 80];
        entry[8..12].copy_from_slice(&offset.to_le_bytes());
        entry[12..16].copy_from_slice(&length.to_le_bytes());
        entry[16..20].copy_from_slice(&padded.to_le_bytes());
        entry[64..72].copy_from_slice(b"test.bin");
        entry[4..6].copy_from_slice(&crc16(b"abc").to_le_bytes());
        cipher(&mut entry, 0xffff);
        logical[64..144].copy_from_slice(&entry);
        logical[1024..1027].copy_from_slice(b"abc");
        let mut head = vec![0u8; 64];
        head[4..8].copy_from_slice(&1056u32.to_le_bytes());
        head[8..10].copy_from_slice(&1u16.to_le_bytes());
        head[16..22].copy_from_slice(b"AC791N");
        head[2..4].copy_from_slice(&crc16(&entry).to_le_bytes());
        let crc = crc16(&head[2..]);
        head[..2].copy_from_slice(&crc.to_le_bytes());
        cipher(&mut head, 0xffff);
        logical[..64].copy_from_slice(&head);
        let mut raw = Vec::new();
        for (index, byte) in logical.iter().enumerate() {
            raw.push(*byte);
            if index < 940 && (index + 1) % 47 == 0 {
                raw.push(0x7d);
            }
        }
        raw
    }

    #[test]
    fn parses_synthetic_container_and_rejects_bad_headers_and_ranges() {
        let raw = synthetic_package();
        let parsed = decode_package(&raw).unwrap();
        assert_eq!(parsed.entries[0].preview, b"abc");
        assert!(parsed.entries[0].checksum.matches());
        assert!(parsed.application.is_none());
        assert!(decode_v15(&raw).is_err()); // no profile bypass through a renamed file
        for end in [0, 63, 959, 1050] {
            assert!(decode_package(&raw[..end]).is_err());
        }
        let mut damaged = raw.clone();
        damaged[0] ^= 1;
        assert!(decode_package(&damaged).is_err());
        // Structurally authentic headers cannot make an invalid span readable.
        assert!(decode_package(&synthetic_entry(u32::MAX, 3, 32)).is_err());
        assert!(decode_package(&synthetic_entry(1024, 33, 32)).is_err());
        assert!(decode_package(&synthetic_entry(1057, 0, 0)).is_err());
        // Unexplained member checksums remain observations, not corruption verdicts.
        let mut differing_payload = raw.clone();
        differing_payload[1044] ^= 1;
        let unresolved = decode_package(&differing_payload).unwrap();
        assert!(!unresolved.entries[0].checksum.matches());
        let mut saved = parsed;
        saved.entries[0].offset = PackageOffset(u64::MAX);
        assert!(saved.validate(raw.len() as u64).is_err());
    }
}
