//! REA evidence keeps its original semantic identity. DIVULGE adds run order.
use crate::{domain::*, fwsc::digest};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const MAX_BYTES: usize = 1024 * 1024;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub node: String,
    pub package: String,
}
impl Config {
    pub fn discover() -> Self {
        if let (Ok(node), Ok(package)) = (
            std::env::var("DIVULGE_REA_NODE"),
            std::env::var("DIVULGE_REA_PACKAGE"),
        ) {
            return Self { node, package };
        }
        // Read the installed launcher as data; never execute a shell or its commands.
        if let Some(paths) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&paths) {
                if let Ok(text) = std::fs::read_to_string(dir.join("rea.cmd")) {
                    for line in text.lines() {
                        let quoted: Vec<_> = line.split('"').skip(1).step_by(2).collect();
                        if quoted.len() >= 2
                            && quoted[0].ends_with("node.exe")
                            && quoted[1].ends_with("scripts\\rea.mjs")
                            && Path::new(quoted[0]).is_file()
                            && let Some(root) = Path::new(quoted[1]).parent().and_then(Path::parent)
                        {
                            return Self {
                                node: quoted[0].into(),
                                package: root.to_string_lossy().into_owned(),
                            };
                        }
                    }
                }
            }
        }
        Self::default()
    }
    fn validate(&self) -> Result<(), String> {
        if !Path::new(&self.node).is_file()
            || !Path::new(&self.package).join("package.json").is_file()
        {
            return Err("REA is unavailable. Select its existing Node executable and rea-agents package directory, or set DIVULGE_REA_NODE / DIVULGE_REA_PACKAGE. No setup is run automatically.".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Discover,
    Entropy {
        path: PathBuf,
        artifact_sha256: String,
        offset: u64,
        length: u64,
        block_size: u64,
    },
    ImportBundle {
        path: PathBuf,
        artifact_sha256: String,
    },
}
impl Request {
    pub fn entropy(
        path: PathBuf,
        artifact: &Artifact,
        offset: u64,
        length: u64,
        block_size: u64,
    ) -> Result<Self, String> {
        if !path.is_absolute()
            || !(256..=65536).contains(&block_size)
            || !block_size.is_power_of_two()
            || length > 1024 * 1024
            || offset
                .checked_add(length)
                .is_none_or(|end| end > artifact.length)
        {
            return Err("Choose an in-file window of at most 1 MiB and a power-of-two block size from 256 to 65536.".into());
        }
        Ok(Self::Entropy {
            path,
            artifact_sha256: artifact.sha256.clone(),
            offset,
            length,
            block_size,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub v: u32,
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub rea_version: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub bundle_json: Option<String>,
    #[serde(default)]
    pub semantic: Vec<String>,
    #[serde(default)]
    pub error: String,
    /// Bounded diagnostic bytes from invalid/partial transport; never evidence.
    #[serde(default)]
    pub partial_output: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReaRun {
    pub record: RecordId,
    pub request: Request,
    pub receipt: Receipt,
}

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("REA field {key} is missing; missing does not mean examined-empty"))
}
fn semantic_projection(e: &Value) -> Result<Value, String> {
    let mut result = serde_json::Map::new();
    for key in [
        "provider",
        "analysis_profile",
        "predicate_type",
        "operation",
        "parameters",
        "raw_result",
        "normalized_result",
        "confidence",
        "authority",
        "environment",
        "limitations",
        "relationship_coverage",
        "locations",
        "evidence_links",
    ] {
        result.insert(key.into(), required(e, key)?.clone());
    }
    let subject = required(e, "subject")?;
    result.insert("subject".into(),if subject.is_null(){Value::Null}else{json!({"digest":required(subject,"digest")?,"format":required(subject,"format")?,"architecture":required(subject,"architecture")?})});
    Ok(Value::Object(result))
}
impl Receipt {
    pub fn bundle(&self) -> Result<Option<Value>, String> {
        self.bundle_json
            .as_ref()
            .map(|s| serde_json::from_str(s).map_err(|e| e.to_string()))
            .transpose()
    }
    pub fn validate(&self, artifact: &Artifact) -> Result<(), String> {
        if self.v != 1
            || self.id.is_empty()
            || ![
                "ready",
                "completed",
                "error",
                "cancelled",
                "timeout",
                "transport_error",
            ]
            .contains(&self.kind.as_str())
        {
            return Err("Invalid REA receipt protocol or outcome".into());
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > MAX_BYTES * 2 {
            return Err("REA receipt exceeds 2 MiB".into());
        }
        let Some(bundle) = self.bundle()? else {
            if self.kind == "completed" || !self.semantic.is_empty() {
                return Err("Completed REA result has no evidence bundle".into());
            }
            return Ok(());
        };
        if self.kind != "completed" || self.rea_version != "6.8.0" {
            return Err("Evidence must come from a completed supported REA receipt".into());
        }
        let records = required(&bundle, "records")?
            .as_array()
            .ok_or("REA records must be an array")?;
        if records.is_empty() || records.len() > 32 || records.len() != self.semantic.len() {
            return Err("Invalid REA record count".into());
        }
        for key in ["unknowns", "environments", "scenarios", "captures"] {
            if required(&bundle, key)? != &json!([]) {
                return Err(format!(
                    "REA entropy profile cannot validate nonempty {key}; preserve it in the original bundle"
                ));
            }
        }
        let mut providers = Vec::new();
        let mut artifacts = Vec::new();
        let mut previous = "";
        for (e, semantic) in records.iter().zip(&self.semantic) {
            let id = required(e, "evidence_id")?
                .as_str()
                .ok_or("Invalid REA identity")?;
            if id <= previous || id != format!("ev_{}", digest(semantic.as_bytes())) {
                return Err("REA identity mismatch, duplicate or noncanonical record order".into());
            }
            previous = id;
            let committed: Value = serde_json::from_str(semantic).map_err(|e| e.to_string())?;
            if semantic_projection(e)? != committed
                || serde_json::to_string(&committed).map_err(|e| e.to_string())? != *semantic
            {
                return Err("REA record differs from its semantic commitment".into());
            }
            let provider = json!({"id":"byte-entropy","name":"REA byte entropy","version":"1"});
            if e["provider"] != provider
                || e["operation"] != "inspect_byte_entropy"
                || e["subject"]["digest"]["sha256"] != artifact.sha256
                || e["subject"]["format"] != "file"
                || !e["subject"]["architecture"].is_null()
            {
                return Err("Unsupported REA evidence profile or artifact identity".into());
            }
            if !providers.contains(&provider) {
                providers.push(provider);
            }
            let manifest =
                json!({"digest":e["subject"]["digest"],"format":"file","architecture":null});
            if !artifacts.contains(&manifest) {
                artifacts.push(manifest);
            }
            if e["confidence"] != "observed"
                || e["authority"] != "shipped-artifact"
                || e["predicate_type"] != "rea.analysis"
                || !e["raw_result"].is_null()
                || !e["analysis_profile"].is_null()
                || !e["environment"].is_null()
                || !e["relationship_coverage"].is_null()
            {
                return Err("Unsupported REA entropy authority or coverage semantics".into());
            }
            for key in ["locations", "limitations", "evidence_links"] {
                if !e[key].is_array() {
                    return Err(format!("REA {key} must be present as an array"));
                }
            }
            for link in e["evidence_links"].as_array().unwrap() {
                if !link
                    .as_str()
                    .is_some_and(|s| s.starts_with("ev_") && s.len() == 67)
                {
                    return Err("Invalid REA evidence link".into());
                }
            }
            let result = &e["normalized_result"];
            let window = &result["window"];
            let offset = window["offset"]
                .as_u64()
                .ok_or("Missing REA window offset")?;
            let length = window["length"]
                .as_u64()
                .ok_or("Missing REA window length")?;
            if offset
                .checked_add(length)
                .is_none_or(|end| end > artifact.length)
                || result["artifact"]["sha256"] != artifact.sha256
                || result["artifact"]["bytes"].as_u64() != Some(artifact.length)
            {
                return Err("REA window or result belongs to another artifact".into());
            }
            let size = result["block_size"]
                .as_u64()
                .ok_or("Missing REA block size")?;
            if !(256..=65536).contains(&size)
                || !size.is_power_of_two()
                || e["parameters"]["offset"].as_u64() != Some(offset)
                || e["parameters"]["block_size"].as_u64() != Some(size)
                || e["parameters"]
                    .get("length")
                    .is_some_and(|v| v.as_u64() != Some(length))
            {
                return Err("REA parameters disagree with its result".into());
            }
            let entropy = result["file_entropy"].as_f64().ok_or("Missing entropy")?;
            if !(0.0..=8.0).contains(&entropy) {
                return Err("Invalid entropy value".into());
            }
            let mut end = offset;
            for block in required(result, "blocks")?
                .as_array()
                .ok_or("Missing examined blocks")?
            {
                let n = block["length"].as_u64().ok_or("Missing block length")?;
                if block["offset"].as_u64() != Some(end)
                    || n == 0
                    || n > size
                    || !block["entropy"]
                        .as_f64()
                        .is_some_and(|v| (0.0..=8.0).contains(&v))
                {
                    return Err("Invalid REA block coverage".into());
                }
                end = end.checked_add(n).ok_or("REA block overflow")?;
            }
            if end != offset + length
                || !result["runs"].is_array()
                || result["limitations"] != e["limitations"]
            {
                return Err("REA entropy coverage or limitations mismatch".into());
            }
        }
        if required(&bundle, "providers")? != &json!(providers)
            || required(&bundle, "artifacts")? != &json!(artifacts)
        {
            return Err("REA bundle manifests disagree with records".into());
        }
        Ok(())
    }
}
impl ReaRun {
    pub fn validate(&self, artifact: &Artifact) -> Result<(), String> {
        self.receipt.validate(artifact)?;
        match &self.request {
            Request::Discover => Err("Discovery is not an evidence run".into()),
            Request::Entropy {
                path,
                artifact_sha256,
                offset,
                length,
                block_size,
            } => {
                Request::entropy(path.clone(), artifact, *offset, *length, *block_size)?;
                if artifact_sha256 != &artifact.sha256 {
                    return Err("REA request artifact mismatch".into());
                }
                if let Some(bundle) = self.receipt.bundle()? {
                    let records = bundle["records"].as_array().unwrap();
                    if records.len() != 1
                        || records[0]["normalized_result"]["window"]
                            != json!({"offset":offset,"length":length})
                    {
                        return Err("REA result does not match the requested window".into());
                    }
                }
                Ok(())
            }
            Request::ImportBundle {
                path,
                artifact_sha256,
            } => {
                if !path.is_absolute() || artifact_sha256 != &artifact.sha256 {
                    Err("REA bundle import target mismatch".into())
                } else {
                    Ok(())
                }
            }
        }
    }
}

pub struct Job {
    pub id: String,
    pub receiver: mpsc::Receiver<Receipt>,
    pub cancel: Arc<AtomicBool>,
    pub generation: u64,
    pub request: Request,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
pub fn start(config: Config, request: Request, generation: u64) -> Job {
    let id = request_id();
    let worker_id = id.clone();
    let (send, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    let task = request.clone();
    std::thread::spawn(move || {
        let receipt = run_with_id(&config, &task, flag, worker_id);
        let _ = send.send(receipt);
    });
    Job {
        id,
        receiver,
        cancel,
        generation,
        request,
    }
}
pub fn run(config: &Config, request: &Request, cancel: Arc<AtomicBool>) -> Receipt {
    run_with_id(config, request, cancel, request_id())
}
fn request_id() -> String {
    format!(
        "rea-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}
fn run_with_id(config: &Config, request: &Request, cancel: Arc<AtomicBool>, id: String) -> Receipt {
    let empty = |kind: &str, error: String| Receipt {
        v: 1,
        id: id.clone(),
        kind: kind.into(),
        rea_version: String::new(),
        capabilities: vec![],
        bundle_json: None,
        semantic: vec![],
        error,
        partial_output: String::new(),
    };
    let mut config = config.clone();
    if config.node.is_empty() && config.package.is_empty() {
        config = Config::discover();
    }
    if let Err(e) = config.validate() {
        return empty("error", e);
    }
    let root = std::env::temp_dir().join(&id);
    let result = (|| -> Result<Receipt, String> {
        std::fs::create_dir(&root).map_err(|e| e.to_string())?;
        let script = root.join("bridge.mjs");
        std::fs::write(&script, include_str!("../adapters/rea_bridge.mjs"))
            .map_err(|e| e.to_string())?;
        let mut command = Command::new(&config.node);
        command
            .arg(script)
            .arg(&config.package)
            .current_dir(&root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn().map_err(|e| e.to_string())?;
        let mut input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let mut message = serde_json::to_value(request).unwrap();
        message["v"] = json!(1);
        message["id"] = json!(id);
        if let Err(error) = writeln!(input, "{message}") {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.to_string());
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let result = BufReader::new(output)
                .take((MAX_BYTES * 2 + 1) as u64)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
        let started = Instant::now();
        let mut stopping = None;
        let mut outcome = "cancelled";
        let bytes = loop {
            if stopping.is_none()
                && (cancel.load(Ordering::Relaxed) || started.elapsed() > Duration::from_secs(30))
            {
                outcome = if cancel.load(Ordering::Relaxed) {
                    "cancelled"
                } else {
                    "timeout"
                };
                stopping = Some(Instant::now());
                let _ = writeln!(input, "{}", json!({"id":id,"op":"cancel"}));
            }
            if stopping.is_some_and(|t: Instant| t.elapsed() > Duration::from_secs(2)) {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(empty(
                    outcome,
                    "Owned REA process stopped; no complete evidence was published.".into(),
                ));
            }
            match rx.recv_timeout(Duration::from_millis(25)) {
                Ok(result) => break result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("REA output reader disconnected".into());
                }
            }
        };
        let _ = child.kill();
        let _ = child.wait();
        if stopping.is_some() {
            return Ok(empty(
                outcome,
                "Cancelled before accepting completion; no evidence published.".into(),
            ));
        }
        let bytes = bytes?;
        Ok(decode_receipt(&id, &bytes))
    })();
    // Only this request's unique directory is removed; no user path is a cleanup target.
    if root.is_dir() {
        let _ = std::fs::remove_dir_all(&root);
    }
    result.unwrap_or_else(|error| empty("transport_error", error))
}

fn decode_receipt(id: &str, bytes: &[u8]) -> Receipt {
    let parsed = if bytes.len() > MAX_BYTES * 2 {
        Err("REA output exceeds 2 MiB".into())
    } else {
        serde_json::from_slice::<Receipt>(bytes)
            .map_err(|e| format!("Incomplete or invalid REA response: {e}"))
            .and_then(|receipt| {
                if receipt.v == 1 && receipt.id == id {
                    Ok(receipt)
                } else {
                    Err("Stale or incompatible REA response".into())
                }
            })
    };
    parsed.unwrap_or_else(|error| Receipt {
        v: 1,
        id: id.into(),
        kind: "transport_error".into(),
        rea_version: String::new(),
        capabilities: vec![],
        bundle_json: None,
        semantic: vec![],
        error,
        partial_output: String::from_utf8_lossy(&bytes[..bytes.len().min(65536)]).into_owned(),
    })
}

impl Investigation {
    pub fn append_rea(&mut self, mut run: ReaRun) -> Result<usize, String> {
        run.validate(
            self.artifact
                .as_ref()
                .ok_or("Open an artifact before REA analysis")?,
        )?;
        if self.rea_runs.len() >= 16 || self.records.len() >= 200 {
            return Err("REA run/record limit reached".into());
        }
        if self.rea_runs.iter().any(|r| r.receipt.id == run.receipt.id) {
            return Err("Duplicate REA request ID".into());
        }
        let id = RecordId(
            self.records
                .iter()
                .map(|r| r.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("Record ID overflow")?,
        );
        run.record = id;
        let complete = run.receipt.kind == "completed";
        let summary = if complete {
            let b = run.receipt.bundle()?.unwrap();
            format!(
                "REA / {} canonical evidence record(s)",
                b["records"].as_array().unwrap().len()
            )
        } else {
            format!("REA / {} / no completed evidence", run.receipt.kind)
        };
        let record=Record{id,title:if complete{"Byte distribution observed by REA".into()}else{"REA request incomplete".into()},summary:summary.clone(),data:RecordData::Rea(run.receipt.id.clone()),note:String::new(),question_id:None,question_plan:None,insight:HarnessInsight{
            question:"How does byte distribution vary within the selected file window?".into(),purpose:"Locate boundaries worth comparing with the package map and decoded application.".into(),action:"REA inspect_byte_entropy reads the original file. It never executes firmware.".into(),observation:summary,explanation:if complete{"Entropy measures distribution in the selected bytes. It does not identify code, encryption or an original data type.".into()}else{run.receipt.error.clone()},uncertainty:"Static byte distribution cannot distinguish compression from encryption or prove runtime behavior. File offsets are not decoded-image addresses.".into(),next_step:"Compare the selected original-file range with the source map, then inspect a bounded decoded-image span.".into()}};
        self.records.push(record);
        self.rea_runs.push(run);
        if let Err(error) = self.validate() {
            self.records.pop();
            self.rea_runs.pop();
            return Err(error);
        }
        Ok(self.records.len() - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(empty: bool) -> (Artifact, Receipt) {
        let receipt: Receipt = serde_json::from_str(if empty {
            include_str!("../tests/fixtures/rea/entropy-0.json")
        } else {
            include_str!("../tests/fixtures/rea/entropy-512.json")
        })
        .unwrap();
        let bundle = receipt.bundle().unwrap().unwrap();
        let e = &bundle["records"][0];
        (
            Artifact {
                name: "synthetic.bin".into(),
                length: 512,
                sha256: e["subject"]["digest"]["sha256"].as_str().unwrap().into(),
                preview: vec![],
                firmware: None,
            },
            receipt,
        )
    }
    #[test]
    fn canonical_rea_fixture_roundtrips_without_rea_or_a_live_target() {
        let (artifact, receipt) = fixture(false);
        receipt.validate(&artifact).unwrap();
        let original = receipt.bundle_json.clone();
        let mut session = Investigation::from_artifact(artifact.clone());
        let request = Request::ImportBundle {
            path: std::env::temp_dir().join("synthetic-entropy-bundle.json"),
            artifact_sha256: artifact.sha256,
        };
        session
            .append_rea(ReaRun {
                record: RecordId(0),
                request,
                receipt,
            })
            .unwrap();
        let restored: Investigation =
            serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.rea_runs[0].receipt.bundle_json, original);
        assert!(restored.is_evidence(restored.records.last().unwrap()));
    }
    #[test]
    fn examined_empty_is_distinct_from_unavailable_and_partial_transport() {
        let (artifact, receipt) = fixture(true);
        receipt.validate(&artifact).unwrap();
        assert_eq!(
            receipt.bundle().unwrap().unwrap()["records"][0]["normalized_result"]["blocks"],
            json!([])
        );
        let partial = decode_receipt(
            "id",
            br#"{"v":1,"id":"id","kind":"completed","bundle_json":"#,
        );
        assert_eq!(partial.kind, "transport_error");
        assert!(partial.bundle().unwrap().is_none());
        assert!(!partial.partial_output.is_empty());
        partial.validate(&artifact).unwrap();
        let stale = decode_receipt("another-id", &serde_json::to_vec(&receipt).unwrap());
        assert_eq!(stale.kind, "transport_error");
        let mut missing = receipt.clone();
        missing.bundle_json = None;
        assert!(missing.validate(&artifact).is_err());
    }
    #[test]
    fn semantic_mutations_missing_fields_and_manifest_changes_are_rejected() {
        let (artifact, receipt) = fixture(false);
        for mutation in 0..5 {
            let mut bad = receipt.clone();
            let mut bundle = bad.bundle().unwrap().unwrap();
            match mutation {
                0 => bundle["records"][0]["normalized_result"]["blocks"][0]["entropy"] = json!(5),
                1 => {
                    bundle["records"][0]
                        .as_object_mut()
                        .unwrap()
                        .remove("relationship_coverage");
                }
                2 => bundle["providers"] = json!([]),
                3 => {
                    let duplicate = bundle["records"][0].clone();
                    bundle["records"].as_array_mut().unwrap().push(duplicate);
                    bad.semantic.push(bad.semantic[0].clone());
                }
                _ => bundle["records"][0]["raw_result"] = json!("changed original output"),
            }
            bad.bundle_json = Some(bundle.to_string());
            assert!(bad.validate(&artifact).is_err(), "mutation {mutation}");
        }
        let mut other = artifact;
        other.sha256 = "a".repeat(64);
        assert!(receipt.validate(&other).is_err());
    }
    #[test]
    fn checked_requests_reject_overflow_out_of_bounds_and_invalid_block_size() {
        let (artifact, _) = fixture(false);
        let path = std::env::temp_dir().join("synthetic.bin");
        assert!(Request::entropy(path.clone(), &artifact, 0, 512, 256).is_ok());
        assert!(Request::entropy(path.clone(), &artifact, u64::MAX, 2, 256).is_err());
        assert!(Request::entropy(path.clone(), &artifact, 0, 513, 256).is_err());
        assert!(Request::entropy(path.clone(), &artifact, 0, 1, 300).is_err());
        assert!(Request::entropy(path, &artifact, 512, 0, 256).is_ok());
    }
    #[test]
    #[ignore = "Requires the locally installed REA 6.8.0 package and compatible Node runtime"]
    fn local_discovery_and_cancellation_are_terminal() {
        let config = Config::discover();
        let receipt = run(
            &config,
            &Request::Discover,
            Arc::new(AtomicBool::new(false)),
        );
        assert_eq!(receipt.kind, "ready", "{}", receipt.error);
        assert_eq!(receipt.rea_version, "6.8.0");
        assert!(
            receipt
                .capabilities
                .contains(&"inspect_byte_entropy".into())
        );
        let cancelled = run(&config, &Request::Discover, Arc::new(AtomicBool::new(true)));
        assert_eq!(cancelled.kind, "cancelled");
        assert!(cancelled.bundle_json.is_none());
        let (artifact, fixture) = fixture(false);
        let path = std::env::temp_dir().join(format!("{}-bundle.json", receipt.id));
        std::fs::write(&path, fixture.bundle_json.as_ref().unwrap()).unwrap();
        let request = Request::ImportBundle {
            path: path.clone(),
            artifact_sha256: artifact.sha256.clone(),
        };
        let imported = run(&config, &request, Arc::new(AtomicBool::new(false)));
        imported.validate(&artifact).unwrap();
        assert_eq!(imported.kind, "completed", "{}", imported.error);
        assert_eq!(imported.bundle().unwrap(), fixture.bundle().unwrap());
        std::fs::write(&path, b"{broken").unwrap();
        let malformed = run(&config, &request, Arc::new(AtomicBool::new(false)));
        assert_eq!(malformed.kind, "error");
        assert!(malformed.bundle_json.is_none());
        std::fs::remove_file(path).unwrap();
    }
}
