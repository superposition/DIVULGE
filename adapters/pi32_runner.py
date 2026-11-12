#!/usr/bin/env python3
"""Bounded subprocess transport for DIVULGE; never executes selection.elf.

The executable is an explicitly selected, already installed disassembler.
All command arguments other than that path are fixed here. No shell is used.
"""
import hashlib
import json
import os
from pathlib import Path
import selectors
import signal
import subprocess
import sys
import time
import resource


ROOT = Path(__file__).resolve().parent
FLAGS = [
    "-disassemble", "-arch-name=pi32v2", "-mcpu=r3", "-minstrev3",
    "-print-imm-hex", "selection.elf",
]
OUTPUT_LIMIT = 256 * 1024
VERSION_LIMIT = 16 * 1024
CANCEL = ROOT / "cancel.request"


def atomic_json(name, value):
    destination = ROOT / name
    temporary = ROOT / (name + ".tmp")
    with temporary.open("w", encoding="utf-8") as stream:
        json.dump(value, stream, ensure_ascii=True)
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, destination)


def process_identity(pid):
    """Linux PID plus start tick prevents killing a reused PID on fallback."""
    try:
        stat = Path(f"/proc/{pid}/stat").read_text()
        return {"pid": pid, "start": stat.rsplit(")", 1)[1].split()[19]}
    except (OSError, IndexError):
        return None


def same_process(identity):
    return identity is not None and process_identity(identity["pid"]) == identity


def record_owner(child=None):
    atomic_json("owner.json", {"worker": process_identity(os.getpid()), "child": child})


def kill_group(process):
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass


def terminate_owned():
    """Fallback invoked only by the host in this run's unique owned directory."""
    CANCEL.touch(exist_ok=True)
    try:
        owner = json.loads((ROOT / "owner.json").read_text())
    except (OSError, ValueError):
        return
    worker = owner.get("worker")
    if not same_process(worker):
        return
    # The worker itself must still run from this exact owned work directory.
    try:
        if Path(f"/proc/{worker['pid']}/cwd").resolve() != ROOT:
            return
    except OSError:
        return
    child = owner.get("child")
    if same_process(child):
        try:
            os.killpg(child["pid"], signal.SIGKILL)
        except ProcessLookupError:
            pass
    # Give the worker time to reap its child and write the final capture.
    deadline = time.monotonic() + 1.0
    while same_process(worker) and time.monotonic() < deadline:
        time.sleep(0.025)
    if same_process(worker):
        try:
            os.kill(worker["pid"], signal.SIGKILL)
        except ProcessLookupError:
            pass


def capture_process(argv, deadline_seconds, output_limit, stdout_name, stderr_name):
    started = time.monotonic()
    # A malformed instruction can crash this vendor tool. Do not leave a core
    # dump containing the selected bytes in the owned temporary directory.
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    outcome = None
    cutoff = None
    process = None
    stdout_path = ROOT / stdout_name
    stderr_path = ROOT / stderr_name
    # Open the bounded files before launch so partial output survives a host kill.
    with stdout_path.open("wb") as stdout_file, stderr_path.open("wb") as stderr_file:
        if CANCEL.exists():
            return "Cancelled", None
        try:
            process = subprocess.Popen(
                argv, cwd=ROOT, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                stderr=subprocess.PIPE, shell=False, start_new_session=True,
            )
            record_owner(process_identity(process.pid))
            streams = selectors.DefaultSelector()
            counts = {"stdout": 0, "stderr": 0}
            for pipe, destination, label in (
                (process.stdout, stdout_file, "stdout"),
                (process.stderr, stderr_file, "stderr"),
            ):
                os.set_blocking(pipe.fileno(), False)
                streams.register(pipe, selectors.EVENT_READ, (destination, label))
            try:
                while True:
                    now = time.monotonic()
                    if outcome is None:
                        if CANCEL.exists():
                            outcome = "Cancelled"
                        elif now - started >= deadline_seconds:
                            outcome = "TimedOut"
                    if outcome is not None and cutoff is None:
                        kill_group(process)
                        cutoff = now + 1.0
                    if not streams.get_map() and process.poll() is not None:
                        break
                    if cutoff is not None and now >= cutoff:
                        break
                    for key, _ in streams.select(0.025):
                        try:
                            chunk = os.read(key.fileobj.fileno(), 64 * 1024)
                        except BlockingIOError:
                            continue
                        if not chunk:
                            streams.unregister(key.fileobj)
                            key.fileobj.close()
                            continue
                        destination, label = key.data
                        remaining = output_limit - counts[label]
                        kept = chunk[:remaining]
                        destination.write(kept)
                        destination.flush()
                        counts[label] += len(kept)
                        if len(chunk) > remaining and outcome is None:
                            outcome = "OutputLimit"
            finally:
                streams.close()
                if process.poll() is None:
                    kill_group(process)
                # Reap the owned child; all members of its session were killed
                # on a bounded stop, including children holding inherited pipes.
                process.wait(timeout=2.0)
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()
                record_owner()
            if outcome is None:
                outcome = "Completed" if process.returncode == 0 else "Failed"
            return outcome, process.returncode
        except BaseException:
            if process is not None:
                kill_group(process)
                try:
                    process.wait(timeout=2.0)
                except subprocess.TimeoutExpired:
                    pass
            raise


def main():
    if sys.argv[1:] == ["--terminate-owned"]:
        terminate_owned()
        return
    if sys.argv[1:]:
        raise ValueError("Unsupported runner arguments")
    started = time.monotonic()
    record_owner()
    report = {
        "executable": "", "tool_sha256": "", "version": "", "flags": FLAGS,
        "outcome": "Failed", "exit_code": None, "elapsed_ms": 0,
        "limitations": ["Runner has not completed"], "finished": False,
    }
    (ROOT / "stdout.bin").write_bytes(b"")
    (ROOT / "stderr.bin").write_bytes(b"")
    atomic_json("report.json", report)
    try:
        request = json.loads((ROOT / "request.json").read_text(encoding="utf-8"))
        selected = request.get("executable", "")
        if not isinstance(selected, str):
            raise ValueError("Executable must be a path string")
        executable = (Path(selected).expanduser() if selected else
                      Path.home() / ".jieli/toolchain/pi32v2/bin/objdump")
        report["executable"] = str(executable)
        if not executable.is_absolute():
            raise ValueError("Configured executable must be an absolute host path")
        executable = executable.resolve(strict=True)
        report["executable"] = str(executable)
        if not executable.is_file():
            raise ValueError("Configured executable is not a regular file")
        binary_digest = hashlib.sha256()
        hashed_bytes = 0
        with executable.open("rb") as binary:
            while True:
                if CANCEL.exists():
                    report["outcome"] = "Cancelled"
                    report["limitations"] = ["Cancelled before disassembler launch"]
                    return
                block = binary.read(1024 * 1024)
                if not block:
                    break
                hashed_bytes += len(block)
                if hashed_bytes > 256 * 1024 * 1024:
                    raise ValueError("Disassembler executable exceeds 256 MiB hash limit")
                binary_digest.update(block)
        report["tool_sha256"] = binary_digest.hexdigest()
        version_outcome, version_code = capture_process(
            [str(executable), "--version"], 5.0, VERSION_LIMIT,
            "version.stdout.bin", "version.stderr.bin",
        )
        version_stdout = (ROOT / "version.stdout.bin").read_bytes()
        version_stderr = (ROOT / "version.stderr.bin").read_bytes()
        report["version"] = version_stdout.decode("utf-8", errors="replace")
        if version_stderr:
            report["version"] += "\n[version stderr]\n" + version_stderr.decode("utf-8", errors="replace")
        if version_outcome != "Completed":
            (ROOT / "stdout.bin").write_bytes(version_stdout)
            (ROOT / "stderr.bin").write_bytes(version_stderr)
            report["outcome"] = version_outcome
            report["exit_code"] = version_code
            report["limitations"] = ["Version probe did not complete; captured bytes belong to --version, not disassembly"]
            return
        atomic_json("report.json", report)
        outcome, code = capture_process(
            [str(executable)] + FLAGS, 15.0, OUTPUT_LIMIT, "stdout.bin", "stderr.bin",
        )
        report["outcome"] = outcome
        report["exit_code"] = code
        report["limitations"] = []
        if outcome != "Completed":
            report["limitations"].append({
                "Failed": "Disassembler exited unsuccessfully; output may be incomplete",
                "Cancelled": "Cancelled; only output received before termination is retained",
                "TimedOut": "15-second disassembly deadline reached; partial output retained",
                "OutputLimit": "256 KiB per-stream output limit reached; retained output is truncated",
            }[outcome])
    except Exception as error:
        report["outcome"] = "Failed"
        report["limitations"] = [f"Runner error: {type(error).__name__}: {str(error)[:1000]}"]
    finally:
        report["elapsed_ms"] = int((time.monotonic() - started) * 1000)
        report["finished"] = True
        atomic_json("report.json", report)


if __name__ == "__main__":
    main()
