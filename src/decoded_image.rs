//! Fresh decoded bytes are an in-memory capability, never a saved DTO.
use crate::{
    coordinates::{ImageId, ImageMap, ImageOffset, ImageSpan},
    fwsc::{self, FirmwareReport},
};

#[derive(Debug)]
/// A saved DTO cannot manufacture this fresh-byte capability.
/// ```compile_fail
/// let image: divulge::decoded_image::DecodedImage = serde_json::from_str("{}").unwrap();
/// ```
pub struct DecodedImage {
    map: ImageMap,
    bytes: Vec<u8>,
}

impl DecodedImage {
    pub(crate) fn from_verified_v15(
        matched: &fwsc::MatchedV15<'_>,
        bytes: Vec<u8>,
        report: &FirmwareReport,
    ) -> Result<Self, String> {
        let app = report
            .application
            .as_ref()
            .ok_or("No checked application")?;
        if fwsc::digest(&bytes) != app.sha256 || bytes.len() as u64 != u64::from(app.entry().length)
        {
            return Err("Decoded image identity does not match its report".into());
        }
        let flash = report.flash().ok_or("No flash parent")?;
        let flash_map = crate::mapping::FlashMap {
            artifact: matched.artifact_id(),
            flash: ImageId::from_sha256(&flash.sha256)?,
            flash_length: u64::from(flash.length),
            package_base: flash.offset,
            package_length: u64::from(report.header.length),
        };
        let package_span = flash_map.resolve(&crate::mapping::FlashSpan::new(
            flash_map.flash.clone(),
            app.flash_offset(),
            bytes.len() as u64,
            flash_map.flash_length,
        )?)?;
        let map = ImageMap::new(
            matched.artifact_id(),
            ImageId::from_sha256(&app.sha256)?,
            bytes.len() as u64,
            u64::from(report.header.length),
            package_span.offset,
            app.mapped_base(),
        )?;
        Ok(Self { map, bytes })
    }

    pub fn map(&self) -> &ImageMap {
        &self.map
    }

    pub fn selection(
        &self,
        offset: ImageOffset,
        length: u64,
    ) -> Result<(ImageSpan, &[u8]), String> {
        let span = self.map.span(offset, length)?;
        Ok((
            span,
            &self.bytes[offset.0 as usize..(offset.0 + length) as usize],
        ))
    }
}
