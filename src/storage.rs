use crate::domain::{Artifact, Investigation, SCHEMA_VERSION};
use std::{
    fs::{self, File},
    io::Read,
    path::Path,
};

const MAX_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_SESSION_BYTES: u64 = 16 * 1024 * 1024;

/// Pure file inspection: never executes or modifies the selected artifact.
pub fn inspect_artifact(path: &Path) -> Result<Artifact, String> {
    inspect_with_image(path).map(|inspection| inspection.artifact)
}

pub struct Inspection {
    pub path: std::path::PathBuf,
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
    Ok(Inspection {
        path: path.to_path_buf(),
        artifact,
        image,
    })
}

pub fn load(path: &Path) -> Result<Investigation, String> {
    load_history(path).map(|history| history.session().clone())
}

pub fn load_history(path: &Path) -> Result<crate::history::History, String> {
    let result = read_bounded(path).and_then(|bytes| decode_history(&bytes));
    result.map_err(|error| {
        if backup_path(path).is_file() {
            format!("{error}. A previous save exists; use File > Recover previous save to inspect it. No recovery was applied automatically.")
        } else { error }
    })
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(MAX_SESSION_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SESSION_BYTES {
        return Err("Investigation exceeds the 16 MiB load limit".into());
    }
    Ok(bytes)
}

fn decode_history(bytes: &[u8]) -> Result<crate::history::History, String> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if value.get("format").is_some() {
        let document: crate::history::Document =
            serde_json::from_value(value).map_err(|e| e.to_string())?;
        return crate::history::History::restore(document);
    }
    let mut investigation: Investigation =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    let original_version = investigation.schema_version;
    // Older files have no decoded data. Do not invent results on migration.
    if matches!(investigation.schema_version, 1..=7) {
        investigation.schema_version = SCHEMA_VERSION;
    }
    investigation.validate()?;
    crate::history::History::new(
        &investigation,
        &format!(
            "Imported schema {original_version} snapshot. Earlier job and edit chronology was not recorded; no events were invented."
        ),
    )
}

/// Compatibility helper for examples; the native app retains its history in memory.
pub fn save(path: &Path, investigation: &Investigation) -> Result<(), String> {
    investigation.validate()?;
    let mut history = if path.exists() {
        let prior = load_history(path)?;
        if prior.session().title == investigation.title
            && prior.session().artifact.as_ref().map(|a| &a.sha256)
                == investigation.artifact.as_ref().map(|a| &a.sha256)
        {
            prior
        } else {
            crate::history::History::new(
                investigation,
                "Starting snapshot saved by a headless caller",
            )?
        }
    } else {
        crate::history::History::new(
            investigation,
            "Starting snapshot saved by a headless caller",
        )?
    };
    history.capture(investigation)?;
    save_history(path, &history)
}

pub fn backup_path(path: &Path) -> std::path::PathBuf {
    path.with_file_name(format!(
        "{}.bak",
        path.file_name().unwrap_or_default().to_string_lossy()
    ))
}

/// Sync a unique sibling before replacement. Only a validated previous save becomes a backup.
/// A crash before replacement leaves the primary intact; after replacement the backup is retained.
pub fn save_history(path: &Path, history: &crate::history::History) -> Result<(), String> {
    crate::history::History::restore(history.document().clone())?;
    let bytes = serde_json::to_vec_pretty(history.document()).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_SESSION_BYTES {
        return Err("Investigation exceeds the 16 MiB save limit".into());
    }
    if path.exists() {
        let previous = read_bounded(path)?;
        decode_history(&previous)
            .map_err(|e| format!("Existing save is invalid; recover or save to a new file: {e}"))?;
        atomic_write(&backup_path(path), &previous)?;
    }
    atomic_write(path, &bytes)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let temp_name = format!(
        "{}.{}.{unique}.tmp",
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
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
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

    #[test]
    fn interrupted_write_backup_recovery_and_unknown_versions() {
        let root = std::env::temp_dir().join(format!(
            "divulge-recovery-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let path = root.join("case.json");
        let mut session = Investigation::workshop();
        save(&path, &session).unwrap();
        // An interrupted sibling write cannot replace the validated primary.
        fs::write(root.join("case.json.interrupted.tmp"), b"{incomplete").unwrap();
        assert_eq!(load(&path).unwrap().records.len(), 6);
        session.records[2].note = "New checkpoint".into();
        save(&path, &session).unwrap();
        assert_eq!(load(&path).unwrap().records[2].note, "New checkpoint");
        assert!(
            load(&backup_path(&path)).unwrap().records[2]
                .note
                .is_empty()
        );
        // A damaged primary is never silently recovered or allowed to overwrite the backup.
        fs::write(&path, b"{broken").unwrap();
        assert!(load(&path).unwrap_err().contains("Recover previous save"));
        assert!(save(&path, &session).is_err());
        let valid = fs::read(backup_path(&path)).unwrap();
        assert!(decode_history(&valid).is_ok());
        let mut unknown: serde_json::Value = serde_json::from_slice(&valid).unwrap();
        unknown["version"] = 99.into();
        assert!(decode_history(&serde_json::to_vec(&unknown).unwrap()).is_err());
        unknown["version"] = 1.into();
        unknown["baseline"]["edges"][0]["to"] = 999.into();
        assert!(decode_history(&serde_json::to_vec(&unknown).unwrap()).is_err());
        let oversized = root.join("oversized.json");
        File::create(&oversized)
            .unwrap()
            .set_len(MAX_SESSION_BYTES + 1)
            .unwrap();
        assert!(load(&oversized).unwrap_err().contains("16 MiB"));
        fs::remove_dir_all(root).unwrap();
    }
}
