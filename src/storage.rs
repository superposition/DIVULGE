use crate::domain::{Artifact, Investigation, SCHEMA_VERSION};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

const MAX_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_SESSION_BYTES: u64 = 8 * 1024 * 1024;

/// Pure file inspection: never executes or modifies the selected artifact.
pub fn inspect_artifact(path: &Path) -> Result<Artifact, String> {
    inspect_with_image(path).map(|inspection| inspection.artifact)
}

pub struct Inspection {
    pub artifact: Artifact,
    pub image: Option<crate::decoded_image::DecodedImage>,
}

pub fn inspect_with_image(path: &Path) -> Result<Inspection, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_ARTIFACT_BYTES {
        return Err("Choose a regular file no larger than 128 MiB".into());
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err("File grew beyond the inspection limit".into());
    }
    let mut artifact = Artifact {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        length: bytes.len() as u64,
        sha256: crate::fwsc::digest(&bytes),
        preview: bytes[..bytes.len().min(256)].to_vec(),
        firmware: None,
    };
    let image = if artifact.matches_v15_reference() {
        // Hashing and parsing use the same read, avoiding a file-change race.
        let (report, image) = crate::fwsc::decode_v15_with_image(&bytes)?;
        artifact.firmware = Some(report);
        image
    } else {
        None
    };
    Ok(Inspection { artifact, image })
}

pub fn load(path: &Path) -> Result<Investigation, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_SESSION_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SESSION_BYTES {
        return Err("Investigation exceeds the 8 MiB load limit".into());
    }
    let mut investigation: Investigation =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    // Older files have no decoded data. Do not invent results on migration.
    if matches!(investigation.schema_version, 1..=4) {
        investigation.schema_version = SCHEMA_VERSION;
    }
    investigation.validate()?;
    Ok(investigation)
}

/// Writes a sibling temporary file before replacement, preserving an existing save on errors.
pub fn save(path: &Path, investigation: &Investigation) -> Result<(), String> {
    investigation.validate()?;
    let bytes = serde_json::to_vec_pretty(investigation).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SESSION_BYTES {
        return Err("Investigation exceeds the 8 MiB save limit".into());
    }
    let temp_name = format!(
        "{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    );
    let temp = path.with_file_name(temp_name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| error.to_string())?;
    use std::io::Write;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(error.to_string());
    }
    drop(file);
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn save_roundtrip_and_actual_file_hash() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("divulge-test-{}-{unique}", std::process::id()));
        fs::create_dir(&root).unwrap();
        let file = root.join("fixture.dat");
        fs::write(&file, b"abc").unwrap();
        let artifact = inspect_artifact(&file).unwrap();
        assert_eq!(artifact.length, 3);
        assert_eq!(
            artifact.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(!artifact.matches_v15_reference());
        let session_path = root.join("session.json");
        let session = Investigation::from_artifact(artifact);
        save(&session_path, &session).unwrap();
        save(&session_path, &session).unwrap();
        let restored = load(&session_path).unwrap();
        assert_eq!(restored.records.len(), 5);
        assert_eq!(restored.artifact.unwrap().preview, b"abc");
        let mut legacy = serde_json::to_value(&session).unwrap();
        legacy["schema_version"] = 1.into();
        for record in legacy["records"].as_array_mut().unwrap() {
            record.as_object_mut().unwrap().remove("question_id");
            record.as_object_mut().unwrap().remove("question_plan");
        }
        fs::write(&session_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let migrated = load(&session_path).unwrap();
        assert_eq!(migrated.schema_version, SCHEMA_VERSION);
        assert!(migrated.records.iter().all(|r| r.question_plan.is_none()));
        fs::write(&session_path, b"{broken").unwrap();
        assert!(load(&session_path).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
