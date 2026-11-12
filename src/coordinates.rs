//! Checked coordinates for one decoded image and its original artifact.
//!
//! These serializable types describe a recorded mapping, not fresh verification
//! of either file. A valid digest string does not prove that bytes were hashed.
//! The runtime must obtain executable-analysis input from an actual file read.
use crate::{
    domain::MappedAddress,
    fwsc::{self, PackageOffset, SourceSpan},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImageOffset(pub u64);

/// SHA-256 of the original artifact. Distinct from a decoded image's identity.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ArtifactId(String);

/// SHA-256 of a complete decoded image, independent of its declared address.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImageId(String);

fn validate_digest(value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("Identity must be a 64-character lowercase SHA-256 digest".into());
    }
    Ok(())
}

impl ArtifactId {
    pub fn from_sha256(value: &str) -> Result<Self, String> {
        validate_digest(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_digest(&self.0)
    }
}

impl ImageId {
    pub fn from_sha256(value: &str) -> Result<Self, String> {
        validate_digest(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_digest(&self.0)
    }
}

/// A nonempty half-open byte range tied to one image. Deserialization alone
/// cannot establish that it is in bounds; resolve it against its ImageMap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSpan {
    pub image_id: ImageId,
    pub offset: ImageOffset,
    pub length: u64,
}

/// V15's length-preserving decoded-image mapping into logical FWSC bytes.
/// Original-file spans may fragment around marker bytes. This is a declared
/// static address map and does not claim runtime execution or code coverage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageMap {
    pub artifact_id: ArtifactId,
    pub image_id: ImageId,
    pub image_length: u64,
    pub package_length: u64,
    pub package_base: PackageOffset,
    pub mapped_base: MappedAddress,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub address: MappedAddress,
    pub source: Vec<SourceSpan>,
}

impl ImageMap {
    pub fn new(
        artifact_id: ArtifactId,
        image_id: ImageId,
        image_length: u64,
        package_length: u64,
        package_base: PackageOffset,
        mapped_base: MappedAddress,
    ) -> Result<Self, String> {
        let map = Self {
            artifact_id,
            image_id,
            image_length,
            package_length,
            package_base,
            mapped_base,
        };
        map.validate()?;
        Ok(map)
    }

    /// Checks structure and arithmetic only. It neither reads nor hashes files.
    pub fn validate(&self) -> Result<(), String> {
        self.artifact_id.validate()?;
        self.image_id.validate()?;
        if self.image_length == 0 {
            return Err("An image mapping must contain at least one byte".into());
        }
        let package_end = self
            .package_base
            .0
            .checked_add(self.image_length)
            .ok_or("Image package range overflows")?;
        if package_end > self.package_length {
            return Err("Decoded image extends beyond its logical package".into());
        }
        // Validate the end as well as the start: a source span length must not
        // push an otherwise representable file offset past u64::MAX.
        fwsc::file_offset(PackageOffset(self.package_length))?;
        fwsc::source_spans(self.package_base, self.image_length, self.package_length)?;
        let mapped_end = self
            .mapped_base
            .0
            .checked_add(self.image_length)
            .ok_or("Mapped image range overflows")?;
        if self.mapped_base.0 > u64::from(u32::MAX) || mapped_end > u64::from(u32::MAX) + 1 {
            return Err("Mapped image exceeds the pi32v2 32-bit address space".into());
        }
        Ok(())
    }

    pub fn span(&self, offset: ImageOffset, length: u64) -> Result<ImageSpan, String> {
        self.validate()?;
        self.check_span_bounds(offset, length)?;
        Ok(ImageSpan {
            image_id: self.image_id.clone(),
            offset,
            length,
        })
    }

    pub fn resolve(&self, span: &ImageSpan) -> Result<Location, String> {
        self.validate()?;
        span.image_id.validate()?;
        if span.image_id != self.image_id {
            return Err("Byte span belongs to a different decoded image".into());
        }
        self.check_span_bounds(span.offset, span.length)?;
        let package_offset = self
            .package_base
            .0
            .checked_add(span.offset.0)
            .ok_or("Resolved package offset overflows")?;
        let crate::mapping::Resolution::Resolved(mapped) =
            crate::mapping::LoadMap::for_image(self).resolve_image(span)?
        else {
            return Err("The image span has no unique declared load mapping".into());
        };
        let package_span = crate::mapping::PackageSpan::new(
            self.artifact_id.clone(),
            PackageOffset(package_offset),
            span.length,
            self.package_length,
        )?;
        let source =
            crate::mapping::SourceMap::fwsc(self.artifact_id.clone(), self.package_length)?
                .resolve(&package_span)?
                .into_iter()
                .map(|part| SourceSpan {
                    offset: part.offset,
                    length: part.length,
                })
                .collect();
        Ok(Location {
            address: mapped.address,
            source,
        })
    }

    fn check_span_bounds(&self, offset: ImageOffset, length: u64) -> Result<(), String> {
        if length == 0 {
            return Err("An image span must contain at least one byte".into());
        }
        let end = offset.0.checked_add(length).ok_or("Image span overflows")?;
        if end > self.image_length {
            return Err("Byte span extends beyond its decoded image".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FileOffset;

    fn map() -> ImageMap {
        ImageMap::new(
            ArtifactId::from_sha256(&"a".repeat(64)).unwrap(),
            ImageId::from_sha256(&"b".repeat(64)).unwrap(),
            100,
            1000,
            PackageOffset(40),
            MappedAddress(0x0200_0120),
        )
        .unwrap()
    }

    #[test]
    fn resolves_fragmented_original_file_ranges_without_losing_image_identity() {
        let mapping = map();
        let span = mapping.span(ImageOffset(6), 3).unwrap();
        let location = mapping.resolve(&span).unwrap();
        assert_eq!(location.address, MappedAddress(0x0200_0126));
        assert_eq!(
            location.source,
            vec![
                SourceSpan {
                    offset: FileOffset(46),
                    length: 1,
                },
                SourceSpan {
                    offset: FileOffset(48),
                    length: 2,
                },
            ]
        );
        let mut foreign = span;
        foreign.image_id = ImageId::from_sha256(&"c".repeat(64)).unwrap();
        assert!(mapping.resolve(&foreign).is_err());
    }

    #[test]
    fn rejects_empty_out_of_bounds_and_overflowing_spans() {
        let mapping = map();
        assert!(mapping.span(ImageOffset(0), 0).is_err());
        assert!(mapping.span(ImageOffset(99), 1).is_ok());
        assert!(mapping.span(ImageOffset(99), 2).is_err());
        assert!(mapping.span(ImageOffset(u64::MAX), 1).is_err());
        assert!(
            mapping
                .resolve(&ImageSpan {
                    image_id: mapping.image_id.clone(),
                    offset: ImageOffset(u64::MAX),
                    length: 2,
                })
                .is_err()
        );
    }

    #[test]
    fn validates_package_and_32_bit_mapped_boundaries() {
        let mut mapping = map();
        mapping.mapped_base = MappedAddress(u64::from(u32::MAX) - 99);
        mapping.validate().unwrap();
        assert_eq!(
            mapping
                .resolve(&mapping.span(ImageOffset(99), 1).unwrap())
                .unwrap()
                .address,
            MappedAddress(u64::from(u32::MAX))
        );
        mapping.mapped_base.0 += 1;
        assert!(mapping.validate().is_err());
        mapping.mapped_base = MappedAddress(u64::MAX);
        assert!(mapping.validate().is_err());
        mapping = map();
        mapping.package_base = PackageOffset(901);
        assert!(mapping.validate().is_err());
        mapping.package_base = PackageOffset(u64::MAX);
        assert!(mapping.validate().is_err());
        mapping = map();
        mapping.package_length = u64::MAX;
        assert!(mapping.validate().is_err());
    }

    #[test]
    fn deserialized_dtos_require_validation_and_do_not_bypass_it() {
        let mapping = map();
        let mut dto = serde_json::to_value(&mapping).unwrap();
        dto["image_length"] = serde_json::json!(2000);
        let imported: ImageMap = serde_json::from_value(dto.clone()).unwrap();
        assert!(imported.validate().is_err());
        assert!(imported.span(ImageOffset(0), 2).is_err());
        dto["image_length"] = serde_json::json!(100);
        dto["image_id"] = serde_json::json!("not-a-sha256");
        let imported: ImageMap = serde_json::from_value(dto).unwrap();
        assert!(imported.validate().is_err());
        assert!(ArtifactId::from_sha256(&"A".repeat(64)).is_err());
        assert!(ImageId::from_sha256(&"g".repeat(64)).is_err());
        assert!(ImageId::from_sha256("abcd").is_err());
        let restored: ImageMap =
            serde_json::from_str(&serde_json::to_string(&mapping).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored, mapping);
        assert_eq!(restored.artifact_id.as_str(), "a".repeat(64));
    }
}
