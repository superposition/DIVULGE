//! Fresh decoded bytes are an in-memory capability, never a saved DTO.
use crate::{
    coordinates::{ArtifactId, ImageId, ImageMap, ImageOffset, ImageSpan},
    domain::V15_SHA256,
    fwsc::{self, FirmwareReport, PackageOffset},
};

#[derive(Debug)]
pub struct DecodedImage {
    map: ImageMap,
    bytes: Vec<u8>,
}

impl DecodedImage {
    pub(crate) fn from_verified_v15(
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
        let map = ImageMap::new(
            ArtifactId::from_sha256(V15_SHA256)?,
            ImageId::from_sha256(&app.sha256)?,
            bytes.len() as u64,
            u64::from(report.header.length),
            PackageOffset(report.flash().ok_or("No flash parent")?.offset.0 + app.flash_offset().0),
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
