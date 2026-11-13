#!/usr/bin/env python3
"""Verify packaged bytes and, optionally, launch the real Mac egui application.

The smoke uses the synthetic workshop only; it sends no model requests and runs
no analysis tools. Its screenshot is actual desktop output, not a mockup.
"""
import argparse
import hashlib
import json
import pathlib
import struct
import subprocess
import sys
import tempfile
import time
import zipfile


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def verify(archive_path, output, smoke=False):
    output.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="divulge-native-") as temporary:
        # macOS temporary paths commonly spell /private/var as /var. Compare
        # resolved spellings on both sides of the archive containment check.
        root = pathlib.Path(temporary).resolve()
        with zipfile.ZipFile(archive_path) as archive:
            names = archive.namelist()
            for name in names:
                path = (root / name).resolve()
                if not path.is_relative_to(root) or "\\" in name:
                    raise ValueError("Unsafe archive path")
                raw = archive.read(name)
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(raw)
                path.chmod((archive.getinfo(name).external_attr >> 16) & 0o777 or 0o644)
        package = next(root.iterdir())
        checksums = (package / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines()
        for entry in checksums:
            expected, name = entry.split("  ", 1)
            path = (package / name).resolve()
            if not path.is_relative_to(package) or sha(path.read_bytes()) != expected:
                raise ValueError("Package digest mismatch: " + name)
        identity = json.loads((package / "release.json").read_text(encoding="utf-8"))
        target = identity["target"]
        mac = target.endswith("apple-darwin")
        binary = package / ("DIVULGE.app/Contents/MacOS/divulge" if mac else
                            "divulge.exe" if "windows" in target else "divulge")
        raw = binary.read_bytes()
        if sha(raw) != identity["executable_sha256"]:
            raise ValueError("Executable differs from release identity")
        if mac:
            expected = 0x100000C if target.startswith("aarch64") else 0x1000007
            magic, cpu, _, kind = struct.unpack_from("<IIII", raw)
            if (magic, cpu, kind) != (0xFEEDFACF, expected, 2):
                raise ValueError("Wrong Mach-O executable architecture")
            if not binary.stat().st_mode & 0o111:
                raise ValueError("Mac executable permission was lost in ZIP")
        report = {"status": "passed", "target": target, "source_commit": identity["source_commit"],
                  "package_files_verified": len(checksums), "executable_sha256": sha(raw),
                  "native_ui_smoke": "not_run", "model_requests": 0, "analysis_tools": 0}
        if smoke:
            if not mac or sys.platform != "darwin":
                raise ValueError("Native UI smoke requires the matching Mac")
            subprocess.run(["codesign", "--verify", "--deep", "--strict", str(package / "DIVULGE.app")], check=True)
            fixture = package / "workshop/fixtures/synthetic-workshop.json"
            before = fixture.read_bytes()
            with (output / "native.stdout.log").open("wb") as stdout, (output / "native.stderr.log").open("wb") as stderr:
                process = subprocess.Popen([str(binary), "--compact", "--session", str(fixture)],
                                           cwd=package, stdout=stdout, stderr=stderr)
                try:
                    time.sleep(5)
                    if process.poll() is not None:
                        raise ValueError("Native app exited before the window smoke")
                    screenshot = output / "native-mac.png"
                    subprocess.run(["/usr/sbin/screencapture", "-x", str(screenshot)], check=True)
                    if not screenshot.is_file() or screenshot.stat().st_size < 1024:
                        raise ValueError("No native desktop screenshot was captured")
                    report["native_ui_smoke"] = "process_alive_and_desktop_captured"
                    report["screenshot"] = str(screenshot)
                    report["fixture_unchanged"] = before == fixture.read_bytes()
                    if not report["fixture_unchanged"]:
                        raise ValueError("Read-only smoke changed its teaching fixture")
                finally:
                    if process.poll() is None:
                        process.terminate()
                        try:
                            process.wait(timeout=5)
                        except subprocess.TimeoutExpired:
                            process.kill()
                            process.wait(timeout=5)
        (output / "native-smoke.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report), flush=True)
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path("output/native-validation"))
    parser.add_argument("--smoke", action="store_true")
    args = parser.parse_args()
    verify(args.archive.resolve(), args.output.resolve(), args.smoke)


if __name__ == "__main__":
    main()
