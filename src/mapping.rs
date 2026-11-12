//! Checked mappings are recorded declarations, never proof of loaded runtime memory.
use crate::{
    coordinates::{ArtifactId, ImageId, ImageMap, ImageOffset, ImageSpan},
    domain::{FileOffset, MappedAddress},
    fwsc::{self, FlashOffset, PackageOffset},
};
use serde::{Deserialize, Serialize};

fn bounded(offset: u64, length: u64, total: u64) -> Result<(), String> {
    if offset.checked_add(length).is_none_or(|end| end > total) {
        Err("Span overflows or extends beyond its coordinate space".into())
    } else {
        Ok(())
    }
}

macro_rules! checked_span {
    ($name:ident,$identity:ident,$coordinate:ident,$field:ident) => {
        /// A half-open range. Empty ranges are permitted; analysis operations may require bytes.
        /// Deserialized DTOs must be validated against the owning space before use.
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name {
            pub identity: $identity,
            pub $field: $coordinate,
            pub length: u64,
        }
        impl $name {
            pub fn new(
                identity: $identity,
                offset: $coordinate,
                length: u64,
                total: u64,
            ) -> Result<Self, String> {
                let span = Self {
                    identity,
                    $field: offset,
                    length,
                };
                span.validate(total)?;
                Ok(span)
            }
            pub fn validate(&self, total: u64) -> Result<(), String> {
                self.identity.validate()?;
                bounded(self.$field.0, self.length, total)
            }
        }
    };
}
checked_span!(FileSpan, ArtifactId, FileOffset, offset);
checked_span!(PackageSpan, ArtifactId, PackageOffset, offset);
checked_span!(FlashSpan, ImageId, FlashOffset, offset);
checked_span!(MappedSpan, ImageId, MappedAddress, address);

/// V15 FWSC marker-removal mapping. The identity names the original carrier;
/// the format/marker layout is a supplied declaration, not recovered behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceMap {
    pub artifact: ArtifactId,
    pub package_length: u64,
    pub file_length: u64,
}
impl SourceMap {
    pub fn fwsc(artifact: ArtifactId, package_length: u64) -> Result<Self, String> {
        let map = Self {
            artifact,
            package_length,
            file_length: fwsc::file_offset(PackageOffset(package_length))?.0,
        };
        map.validate()?;
        Ok(map)
    }
    pub fn validate(&self) -> Result<(), String> {
        self.artifact.validate()?;
        if fwsc::file_offset(PackageOffset(self.package_length))?.0 != self.file_length {
            return Err("FWSC source-map lengths disagree".into());
        }
        Ok(())
    }
    pub fn resolve(&self, span: &PackageSpan) -> Result<Vec<FileSpan>, String> {
        self.validate()?;
        span.validate(self.package_length)?;
        if span.identity != self.artifact {
            return Err("Package span belongs to another artifact".into());
        }
        fwsc::source_spans(span.offset, span.length, self.package_length)?
            .into_iter()
            .map(|part| {
                FileSpan::new(
                    self.artifact.clone(),
                    part.offset,
                    part.length,
                    self.file_length,
                )
            })
            .collect()
    }
}

/// A flash member's identity and location inside a package. The reference SFC
/// transformation preserves byte positions; the bytes may have different values.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlashMap {
    pub artifact: ArtifactId,
    pub flash: ImageId,
    pub flash_length: u64,
    pub package_base: PackageOffset,
    pub package_length: u64,
}
impl FlashMap {
    pub fn validate(&self) -> Result<(), String> {
        self.artifact.validate()?;
        self.flash.validate()?;
        bounded(self.package_base.0, self.flash_length, self.package_length)
    }
    pub fn resolve(&self, span: &FlashSpan) -> Result<PackageSpan, String> {
        self.validate()?;
        span.validate(self.flash_length)?;
        if span.identity != self.flash {
            return Err("Flash span belongs to another image".into());
        }
        PackageSpan::new(
            self.artifact.clone(),
            PackageOffset(
                self.package_base
                    .0
                    .checked_add(span.offset.0)
                    .ok_or("Flash mapping overflow")?,
            ),
            span.length,
            self.package_length,
        )
    }
}

/// Resolution is independent of tool success and of analysis coverage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution<T> {
    Resolved(T),
    Ambiguous { candidates: Vec<T>, reason: String },
    Unresolved { reason: String },
}
impl<T: PartialEq> Resolution<T> {
    fn candidates(values: Vec<T>) -> Self {
        let mut unique = Vec::new();
        for value in values {
            if !unique.contains(&value) {
                unique.push(value);
            }
        }
        match unique.len() {
            0 => Self::Unresolved {
                reason: "No declared mapping covers the whole range".into(),
            },
            1 => Self::Resolved(unique.pop().unwrap()),
            _ => Self::Ambiguous {
                candidates: unique,
                reason: "Multiple declared mappings cover this range; no candidate was selected"
                    .into(),
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoadRegion {
    pub image: ImageId,
    pub image_length: u64,
    pub image_offset: ImageOffset,
    pub length: u64,
    pub address: MappedAddress,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadMap {
    pub regions: Vec<LoadRegion>,
}
impl LoadMap {
    pub fn for_image(map: &ImageMap) -> Self {
        Self {
            regions: vec![LoadRegion {
                image: map.image_id.clone(),
                image_length: map.image_length,
                image_offset: ImageOffset(0),
                length: map.image_length,
                address: map.mapped_base,
            }],
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.regions.is_empty() || self.regions.len() > 64 {
            return Err("Load map requires 1-64 declared regions".into());
        }
        for region in &self.regions {
            region.image.validate()?;
            if region.length == 0 {
                return Err("Empty load region".into());
            }
            bounded(region.image_offset.0, region.length, region.image_length)?;
            bounded(region.address.0, region.length, u64::MAX)?;
        }
        Ok(())
    }
    pub fn resolve_address(
        &self,
        address: MappedAddress,
        length: u64,
    ) -> Result<Resolution<ImageSpan>, String> {
        self.validate()?;
        if length == 0 {
            return Err("Address resolution requires a nonempty span".into());
        }
        bounded(address.0, length, u64::MAX)?;
        let mut candidates = Vec::new();
        for region in &self.regions {
            if let Some(delta) = address.0.checked_sub(region.address.0)
                && delta
                    .checked_add(length)
                    .is_some_and(|end| end <= region.length)
            {
                candidates.push(ImageSpan {
                    image_id: region.image.clone(),
                    offset: ImageOffset(region.image_offset.0 + delta),
                    length,
                });
            }
        }
        Ok(Resolution::candidates(candidates))
    }
    pub fn resolve_image(&self, span: &ImageSpan) -> Result<Resolution<MappedSpan>, String> {
        self.validate()?;
        span.image_id.validate()?;
        if span.length == 0 {
            return Err("Image resolution requires a nonempty span".into());
        }
        bounded(span.offset.0, span.length, u64::MAX)?;
        let mut candidates = Vec::new();
        for region in &self.regions {
            if span.image_id == region.image
                && let Some(delta) = span.offset.0.checked_sub(region.image_offset.0)
                && delta
                    .checked_add(span.length)
                    .is_some_and(|end| end <= region.length)
            {
                candidates.push(MappedSpan::new(
                    region.image.clone(),
                    MappedAddress(region.address.0 + delta),
                    span.length,
                    u64::MAX,
                )?);
            }
        }
        Ok(Resolution::candidates(candidates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn artifact() -> ArtifactId {
        ArtifactId::from_sha256(&"a".repeat(64)).unwrap()
    }
    fn image() -> ImageId {
        ImageId::from_sha256(&"b".repeat(64)).unwrap()
    }
    #[test]
    fn marker_and_flash_maps_preserve_all_fragments_and_owning_identity() {
        let source = SourceMap::fwsc(artifact(), 1000).unwrap();
        let flash = FlashMap {
            artifact: artifact(),
            flash: image(),
            flash_length: 100,
            package_base: PackageOffset(40),
            package_length: 1000,
        };
        let span = FlashSpan::new(image(), FlashOffset(6), 3, 100).unwrap();
        let parts = source.resolve(&flash.resolve(&span).unwrap()).unwrap();
        assert_eq!(
            parts
                .iter()
                .map(|s| (s.offset.0, s.length))
                .collect::<Vec<_>>(),
            vec![(46, 1), (48, 2)]
        );
        assert!(parts.iter().all(|s| s.identity == artifact()));
        let mut foreign = span;
        foreign.identity = ImageId::from_sha256(&"c".repeat(64)).unwrap();
        assert!(flash.resolve(&foreign).is_err());
        let mut foreign = flash
            .resolve(&FlashSpan::new(image(), FlashOffset(0), 1, 100).unwrap())
            .unwrap();
        foreign.identity = ArtifactId::from_sha256(&"c".repeat(64)).unwrap();
        assert!(source.resolve(&foreign).is_err());
        let mut malformed = source;
        malformed.file_length = 1;
        assert!(malformed.validate().is_err());
    }
    #[test]
    fn load_map_distinguishes_unique_ambiguous_unresolved_and_invalid() {
        let region = LoadRegion {
            image: image(),
            image_length: 64,
            image_offset: ImageOffset(4),
            length: 16,
            address: MappedAddress(0x100),
        };
        let mut map = LoadMap {
            regions: vec![region.clone()],
        };
        let Resolution::Resolved(span) = map.resolve_address(MappedAddress(0x102), 2).unwrap()
        else {
            panic!("unique mapping expected")
        };
        assert_eq!(span.offset, ImageOffset(6));
        assert_eq!(span.image_id, image());
        assert!(matches!(
            map.resolve_address(MappedAddress(0xff), 2).unwrap(),
            Resolution::Unresolved { .. }
        ));
        assert!(matches!(
            map.resolve_image(&span).unwrap(),
            Resolution::Resolved(_)
        ));
        let mut alias = region;
        alias.image = ImageId::from_sha256(&"c".repeat(64)).unwrap();
        map.regions.push(alias);
        assert!(
            matches!(map.resolve_address(MappedAddress(0x102),2).unwrap(),Resolution::Ambiguous{candidates,..} if candidates.len()==2)
        );
        assert!(map.resolve_address(MappedAddress(u64::MAX), 2).is_err());
        map.regions[0].image_offset = ImageOffset(u64::MAX);
        assert!(map.validate().is_err());
    }
    #[test]
    fn all_coordinate_spaces_reject_overflow_and_out_of_bounds() {
        assert!(FileSpan::new(artifact(), FileOffset(u64::MAX), 2, u64::MAX).is_err());
        assert!(PackageSpan::new(artifact(), PackageOffset(9), 2, 10).is_err());
        assert!(FlashSpan::new(image(), FlashOffset(9), 2, 10).is_err());
        assert!(MappedSpan::new(image(), MappedAddress(9), 2, 10).is_err());
        assert!(FileSpan::new(artifact(), FileOffset(10), 0, 10).is_ok());
    }
}
