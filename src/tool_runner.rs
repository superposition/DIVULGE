//! Bounded transport for the installed pi32v2 disassembler. The ELF wrapper is
//! input data only; neither it nor the firmware is executed.
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const HELPER: &str = include_str!("../adapters/pi32_runner.py");
const OUTPUT_LIMIT: usize = 256 * 1024;
const OUTER_TIMEOUT: Duration = Duration::from_secs(30);
const FLAGS: [&str; 6] = [
    "-disassemble",
    "-arch-name=pi32v2",
    "-mcpu=r3",
    "-minstrev3",
    "-print-imm-hex",
    "selection.elf",
];
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolConfig {
    pub distribution: String,
    /// Empty discovers only ~/.jieli/toolchain/pi32v2/bin/objdump on the host.
    pub executable: String,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            distribution: "Ubuntu-22.04".into(),
            executable: String::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolOutcome {
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    OutputLimit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCapture {
    pub executable: String,
    pub tool_sha256: String,
    pub version: String,
    pub flags: Vec<String>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub outcome: ToolOutcome,
    pub exit_code: Option<i32>,
    pub elapsed_ms: u64,
    pub limitations: Vec<String>,
}

#[derive(Deserialize)]
struct RunnerReport {
    executable: String,
    tool_sha256: String,
    version: String,
    flags: Vec<String>,
    outcome: ToolOutcome,
    exit_code: Option<i32>,
    elapsed_ms: u64,
    limitations: Vec<String>,
    finished: bool,
}

/// Uses one owned temporary directory and fixed tool arguments. Cancellation
/// signals the helper, which kills and reaps its separate tool process group.
pub fn run(
    wrapper: &[u8],
    config: &ToolConfig,
    cancel: &AtomicBool,
) -> Result<ToolCapture, String> {
    if wrapper.is_empty() || wrapper.len() > 8192 {
        return Err("The disassembler wrapper must contain 1-8192 bytes".into());
    }
    if cancel.load(Ordering::Relaxed) {
        return Ok(empty_capture(
            config,
            ToolOutcome::Cancelled,
            "Cancelled before tool launch",
        ));
    }
    let started = Instant::now();
    let mut work = OwnedDirectory::new()?;
    fs::write(work.path.join("runner.py"), HELPER).map_err(|e| e.to_string())?;
    fs::write(work.path.join("selection.elf"), wrapper).map_err(|e| e.to_string())?;
    let request = serde_json::to_vec(&serde_json::json!({ "executable": config.executable }))
        .map_err(|e| e.to_string())?;
    fs::write(work.path.join("request.json"), request).map_err(|e| e.to_string())?;
    let mut command = helper_command(config, &work.path)?;
    command.stdout(Stdio::from(
        File::create(work.path.join("helper.stdout.bin")).map_err(|e| e.to_string())?,
    ));
    command.stderr(Stdio::from(
        File::create(work.path.join("helper.stderr.bin")).map_err(|e| e.to_string())?,
    ));
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start disassembler transport: {e}"))?;
    let mut forced_outcome = None;
    let mut stop_started = None;
    let mut transport_notes = Vec::new();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {}
            Err(error) => {
                transport_notes.push(format!("Could not query runner status: {error}"));
                forced_outcome = Some(ToolOutcome::Failed);
                break None;
            }
        }
        if stop_started.is_none() {
            let requested = if cancel.load(Ordering::Relaxed) {
                Some(ToolOutcome::Cancelled)
            } else if started.elapsed() >= OUTER_TIMEOUT {
                Some(ToolOutcome::TimedOut)
            } else {
                None
            };
            if let Some(outcome) = requested {
                forced_outcome = Some(outcome);
                if let Err(error) = fs::write(work.path.join("cancel.request"), b"cancel") {
                    transport_notes.push(format!("Could not write cancellation signal: {error}"));
                }
                stop_started = Some(Instant::now());
            }
        }
        if stop_started.is_some_and(|time| time.elapsed() >= Duration::from_secs(3)) {
            break None;
        }
        thread::sleep(Duration::from_millis(25));
    };
    let mut helper_stopped = status.is_some();
    if !helper_stopped {
        // WSL's launcher alone is not a reliable process-group cancellation
        // boundary. Ask a second bounded helper to verify PID start times and
        // terminate only this directory's recorded worker and tool group.
        let _ = fs::write(work.path.join("cancel.request"), b"cancel");
        let cleanup_result = terminate_owned(config, &work.path);
        if let Err(error) = cleanup_result {
            transport_notes.push(format!("Owned-process fallback: {error}"));
        }
        helper_stopped = poll_exit(&mut child, Duration::from_secs(2)).is_some();
        if !helper_stopped {
            let _ = child.kill();
            let _ = poll_exit(&mut child, Duration::from_secs(2));
            // A launcher kill cannot prove guest termination. Retain receipts
            // and PID records rather than remove a directory still in use.
            work.retain = true;
            transport_notes.push(format!(
                "Helper shutdown was not confirmed; capture retained at {}",
                work.path.display()
            ));
        }
    }
    let (stdout, stdout_truncated) = read_optional(&work.path.join("stdout.bin"), OUTPUT_LIMIT)?;
    let (stderr, stderr_truncated) = read_optional(&work.path.join("stderr.bin"), OUTPUT_LIMIT)?;
    let (metadata, metadata_truncated) = read_optional(&work.path.join("report.json"), 512 * 1024)?;
    let report = if metadata_truncated {
        None
    } else {
        serde_json::from_slice::<RunnerReport>(&metadata).ok()
    };
    let mut capture = if let Some(report) = report {
        let mut limitations = report.limitations;
        if !report.finished {
            limitations
                .push("Runner stopped before finalizing its report; raw output is partial".into());
        }
        ToolCapture {
            executable: report.executable,
            tool_sha256: report.tool_sha256,
            version: report.version,
            flags: report.flags,
            stdout,
            stderr,
            outcome: forced_outcome.unwrap_or(if report.finished {
                report.outcome
            } else {
                ToolOutcome::Failed
            }),
            exit_code: report.exit_code,
            elapsed_ms: report.elapsed_ms,
            limitations,
        }
    } else {
        let mut capture = empty_capture(
            config,
            forced_outcome.unwrap_or(ToolOutcome::Failed),
            "Transport did not return a valid final report; available raw output is retained",
        );
        capture.stdout = stdout;
        capture.stderr = stderr;
        capture
    };
    capture.elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    if let Some(status) = status
        && !status.success()
    {
        if forced_outcome.is_none() {
            capture.outcome = ToolOutcome::Failed;
        }
        transport_notes.push(format!("Runner transport exited with {status}"));
    }
    if stdout_truncated || stderr_truncated {
        if forced_outcome.is_none() {
            capture.outcome = ToolOutcome::OutputLimit;
        }
        capture
            .limitations
            .push("Host truncated an oversized capture to 256 KiB per stream".into());
    }
    if capture.outcome != ToolOutcome::Completed {
        let (diagnostic, _) = read_optional(&work.path.join("helper.stderr.bin"), 4096)?;
        if !diagnostic.is_empty() {
            transport_notes.push(format!(
                "Transport stderr: {}",
                String::from_utf8_lossy(&diagnostic)
            ));
        }
    }
    capture.limitations.extend(transport_notes);
    if !work.retain
        && let Err(error) = work.cleanup()
    {
        capture
            .limitations
            .push(format!("Temporary capture cleanup incomplete: {error}"));
    }
    Ok(capture)
}

fn empty_capture(config: &ToolConfig, outcome: ToolOutcome, limitation: &str) -> ToolCapture {
    ToolCapture {
        executable: config.executable.clone(),
        tool_sha256: String::new(),
        version: String::new(),
        flags: FLAGS.iter().map(|s| (*s).to_owned()).collect(),
        stdout: Vec::new(),
        stderr: Vec::new(),
        outcome,
        exit_code: None,
        elapsed_ms: 0,
        limitations: vec![limitation.into()],
    }
}

fn helper_command(config: &ToolConfig, directory: &Path) -> Result<Command, String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if config.distribution.trim().is_empty() {
            return Err("Choose a WSL distribution for the installed pi32v2 tool".into());
        }
        let mut command = Command::new("wsl.exe");
        command.args([
            "-d",
            &config.distribution,
            "--cd",
            &wsl_path(directory)?,
            "--exec",
            "python3",
            "runner.py",
        ]);
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        command.stdin(Stdio::null());
        Ok(command)
    }
    #[cfg(not(windows))]
    {
        let _ = config;
        let mut command = Command::new("python3");
        command
            .arg(directory.join("runner.py"))
            .current_dir(directory)
            .stdin(Stdio::null());
        Ok(command)
    }
}

#[cfg(windows)]
fn wsl_path(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or("Temporary path is not Unicode")?
        .replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    let bytes = text.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' || bytes[2] != b'/' {
        return Err("WSL transport requires a temporary directory on a local drive".into());
    }
    Ok(format!(
        "/mnt/{}/{}",
        char::from(bytes[0].to_ascii_lowercase()),
        &text[3..]
    ))
}

fn terminate_owned(config: &ToolConfig, directory: &Path) -> Result<(), String> {
    let mut command = helper_command(config, directory)?;
    command
        .arg("--terminate-owned")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    if let Some(status) = poll_exit(&mut child, Duration::from_secs(3)) {
        if status.success() {
            Ok(())
        } else {
            Err(format!("Process cleanup exited with {status}"))
        }
    } else {
        let _ = child.kill();
        let _ = poll_exit(&mut child, Duration::from_secs(1));
        Err("Process cleanup exceeded its bounded deadline".into())
    }
}

fn poll_exit(child: &mut Child, deadline: Duration) -> Option<ExitStatus> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if started.elapsed() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn read_optional(path: &Path, limit: usize) -> Result<(Vec<u8>, bool), String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), false));
        }
        Err(error) => return Err(error.to_string()),
    };
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Runner output is not a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let truncated = bytes.len() > limit;
    bytes.truncate(limit);
    Ok((bytes, truncated))
}

struct OwnedDirectory {
    path: PathBuf,
    retain: bool,
    cleaned: bool,
}

impl OwnedDirectory {
    fn new() -> Result<Self, String> {
        let parent = fs::canonicalize(std::env::temp_dir()).map_err(|e| e.to_string())?;
        for _ in 0..8 {
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                "divulge-pi32-{}-{stamp}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        retain: false,
                        cleaned: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.to_string()),
            }
        }
        Err("Could not create a unique analysis directory".into())
    }

    fn cleanup(&mut self) -> Result<(), String> {
        if self.cleaned || self.retain {
            return Ok(());
        }
        let resolved = fs::canonicalize(&self.path).map_err(|e| e.to_string())?;
        if resolved != self.path
            || fs::symlink_metadata(&self.path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            self.retain = true;
            return Err("Owned temporary directory changed; cleanup refused".into());
        }
        // No recursive deletion and no deletion of unexpected tool-created files.
        for name in [
            "runner.py",
            "selection.elf",
            "request.json",
            "cancel.request",
            "report.json",
            "report.json.tmp",
            "owner.json",
            "owner.json.tmp",
            "stdout.bin",
            "stderr.bin",
            "version.stdout.bin",
            "version.stderr.bin",
            "helper.stdout.bin",
            "helper.stderr.bin",
        ] {
            match fs::remove_file(self.path.join(name)) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        fs::remove_dir(&self.path).map_err(|e| e.to_string())?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for OwnedDirectory {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unbounded_wrappers_and_honors_prelaunch_cancel() {
        let config = ToolConfig::default();
        let cancel = AtomicBool::new(false);
        assert!(run(&[], &config, &cancel).is_err());
        assert!(run(&vec![0; 8193], &config, &cancel).is_err());
        cancel.store(true, Ordering::Relaxed);
        let capture = run(b"fixture", &config, &cancel).unwrap();
        assert_eq!(capture.outcome, ToolOutcome::Cancelled);
        assert!(capture.stdout.is_empty());
        assert!(capture.tool_sha256.is_empty());
    }

    #[test]
    fn output_reads_are_bounded_and_cleanup_does_not_delete_unknown_files() {
        let mut directory = OwnedDirectory::new().unwrap();
        fs::write(directory.path.join("stdout.bin"), b"abcdef").unwrap();
        assert_eq!(
            read_optional(&directory.path.join("stdout.bin"), 3).unwrap(),
            (b"abc".to_vec(), true)
        );
        fs::write(directory.path.join("unrecognized.txt"), b"preserve").unwrap();
        assert!(directory.cleanup().is_err());
        assert_eq!(
            fs::read(directory.path.join("unrecognized.txt")).unwrap(),
            b"preserve"
        );
        // This test owns that particular file; cleanup never removes it itself.
        fs::remove_file(directory.path.join("unrecognized.txt")).unwrap();
        directory.cleanup().unwrap();
    }

    struct FakeTool {
        directory: OwnedDirectory,
        config: ToolConfig,
    }

    impl FakeTool {
        fn new(body: &str) -> Self {
            let directory = OwnedDirectory::new().unwrap();
            let path = directory.path.join("fake tool.py");
            let source = format!(
                "#!/usr/bin/python3\nimport os, sys, time, signal, subprocess\nfrom pathlib import Path\nif sys.argv[1:] == ['--version']:\n    print('DIVULGE harmless test tool')\n    sys.exit(0)\n{body}\n"
            );
            fs::write(&path, source).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
            }
            #[cfg(windows)]
            let executable = wsl_path(&path).unwrap();
            #[cfg(not(windows))]
            let executable = path.to_str().unwrap().to_owned();
            Self {
                directory,
                config: ToolConfig {
                    executable,
                    ..ToolConfig::default()
                },
            }
        }
    }

    impl Drop for FakeTool {
        fn drop(&mut self) {
            for name in ["fake tool.py", "started", "heartbeat"] {
                let _ = fs::remove_file(self.directory.path.join(name));
            }
        }
    }

    #[test]
    #[ignore = "requires Python on the analysis host (WSL Ubuntu-22.04 on Windows)"]
    fn fake_tools_preserve_failures_crashes_and_bounded_output() {
        let cancel = AtomicBool::new(false);
        let failing = FakeTool::new(
            "os.write(1, b'partial output\\xff\\n')\nos.write(2, b'failure detail\\n')\nsys.exit(7)",
        );
        let capture = run(b"not executed", &failing.config, &cancel).unwrap();
        assert_eq!(capture.outcome, ToolOutcome::Failed);
        assert_eq!(capture.exit_code, Some(7));
        assert_eq!(capture.stdout, b"partial output\xff\n");
        assert_eq!(capture.stderr, b"failure detail\n");
        assert_eq!(
            capture.tool_sha256,
            crate::fwsc::digest(&fs::read(failing.directory.path.join("fake tool.py")).unwrap())
        );

        let crashing =
            FakeTool::new("os.write(1, b'before crash\\n')\nos.kill(os.getpid(), signal.SIGSEGV)");
        let capture = run(b"not executed", &crashing.config, &cancel).unwrap();
        assert_eq!(capture.outcome, ToolOutcome::Failed);
        assert_eq!(capture.exit_code, Some(-11));
        assert_eq!(capture.stdout, b"before crash\n");

        let flooding = FakeTool::new("os.write(1, b'x' * (300 * 1024))\ntime.sleep(1)");
        let capture = run(b"not executed", &flooding.config, &cancel).unwrap();
        assert_eq!(capture.outcome, ToolOutcome::OutputLimit);
        assert_eq!(capture.stdout.len(), OUTPUT_LIMIT);
        assert!(capture.stderr.len() <= OUTPUT_LIMIT);
    }

    #[test]
    #[ignore = "requires Python on the analysis host (WSL Ubuntu-22.04 on Windows)"]
    fn cancellation_stops_the_tool_group_and_retains_partial_output() {
        let tool = FakeTool::new(
            r#"child_code = "from pathlib import Path; import sys,time; p=Path(sys.argv[1]); f=p.open('ab', buffering=0); exec('while True:\\n f.write(b\"x\"); time.sleep(0.025)')"
subprocess.Popen(['/usr/bin/python3', '-c', child_code, str(Path(__file__).parent / 'heartbeat')])
os.write(1, b'started\n')
(Path(__file__).parent / 'started').write_text('ready')
time.sleep(60)
"#,
        );
        let cancel = AtomicBool::new(false);
        let marker = tool.directory.path.join("started");
        let (capture, reached_tool) = thread::scope(|scope| {
            let signal = scope.spawn(|| {
                let started = Instant::now();
                while !marker.exists() && started.elapsed() < Duration::from_secs(20) {
                    thread::sleep(Duration::from_millis(25));
                }
                let reached = marker.exists();
                thread::sleep(Duration::from_millis(200));
                cancel.store(true, Ordering::Relaxed);
                reached
            });
            (
                run(b"not executed", &tool.config, &cancel).unwrap(),
                signal.join().unwrap(),
            )
        });
        assert!(reached_tool);
        assert_eq!(capture.outcome, ToolOutcome::Cancelled);
        assert_eq!(capture.stdout, b"started\n");
        let heartbeat = tool.directory.path.join("heartbeat");
        let after = fs::read(&heartbeat).unwrap();
        assert!(!after.is_empty());
        thread::sleep(Duration::from_millis(150));
        assert_eq!(fs::read(heartbeat).unwrap(), after);
    }

    #[test]
    #[ignore = "requires analysis-host Python and exercises the 15-second deadline"]
    fn deadline_retains_output_and_reports_timeout() {
        let tool = FakeTool::new("os.write(1, b'before deadline\\n')\ntime.sleep(60)");
        let capture = run(b"not executed", &tool.config, &AtomicBool::new(false)).unwrap();
        assert_eq!(capture.outcome, ToolOutcome::TimedOut);
        assert_eq!(capture.stdout, b"before deadline\n");
        assert!(capture.elapsed_ms < 30_000);
    }
}
