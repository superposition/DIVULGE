#!/usr/bin/env python3
"""Build a clean native desktop release with source, workbook and notices."""
import argparse
import hashlib
import io
import json
import os
import pathlib
import plistlib
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(args, **kwargs):
    return subprocess.check_output(args, cwd=ROOT, **kwargs)


def sha(data):
    return hashlib.sha256(data).hexdigest()


TARGETS = {"x86_64-pc-windows-msvc": ("win32", "windows-x86_64"),
           "aarch64-apple-darwin": ("darwin", "macos-arm64"),
           "x86_64-apple-darwin": ("darwin", "macos-x86_64"),
           "x86_64-unknown-linux-gnu": ("linux", "linux-x86_64"),
           "aarch64-unknown-linux-gnu": ("linux", "linux-arm64")}


def app_bundle(staging, executable, version):
    """Construct and locally sign the same egui binary as a Finder-launchable app."""
    app = staging / "DIVULGE.app"
    contents = app / "Contents"
    (contents / "MacOS").mkdir(parents=True)
    binary = contents / "MacOS/divulge"
    shutil.copy2(executable, binary)
    binary.chmod(0o755)
    info = {"CFBundleName": "DIVULGE", "CFBundleDisplayName": "DIVULGE",
            "CFBundleIdentifier": "com.superposition.divulge", "CFBundleExecutable": "divulge",
            "CFBundlePackageType": "APPL", "CFBundleVersion": version,
            "CFBundleShortVersionString": version, "LSMinimumSystemVersion": "11.0",
            "NSHighResolutionCapable": True, "NSPrincipalClass": "NSApplication"}
    (contents / "Info.plist").write_bytes(plistlib.dumps(info))
    subprocess.run(["codesign", "--force", "--sign", "-", str(app)], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
    return app


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--target", choices=sorted(TARGETS), help="Native build target; defaults to the rustc host")
    args = parser.parse_args()
    host = next(line[6:] for line in run(["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
    target = args.target or host
    if target not in TARGETS or sys.platform != TARGETS[target][0] or host != target:
        raise SystemExit("Package on the matching native host. Supported targets: " + ", ".join(sorted(TARGETS)))
    if run(["git", "status", "--porcelain"]).strip():
        raise SystemExit("Commit the reviewed source before creating a release.")
    revision = run(["git", "rev-parse", "HEAD"], text=True).strip()
    environment = os.environ.copy()
    flags = environment.get("CARGO_ENCODED_RUSTFLAGS", "").split("\x1f")
    flags = [value for value in flags if value]
    if environment.get("RUSTFLAGS"):
        raise SystemExit("Use CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS for this recipe.")
    # Remap both project and cached dependency locations before compiling all crates.
    for source, replacement in [(ROOT, "/divulge"), (pathlib.Path.home(), "/build")]:
        flags.append("--remap-path-prefix={0}={1}".format(source, replacement))
        flags.append("--remap-path-prefix={0}={1}".format(source.as_posix(), replacement))
    environment["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(flags)
    target_directory = pathlib.Path(json.loads(run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version=1"], text=True))["target_directory"])
    subprocess.run(["cargo", "build", "--release", "--locked", "--bin", "divulge"],
                   cwd=ROOT, env=environment, check=True)
    executable = target_directory / "release" / ("divulge.exe" if sys.platform == "win32" else "divulge")
    native = executable.read_bytes()
    private_strings = [str(ROOT), ROOT.as_posix(), str(pathlib.Path.home()), pathlib.Path.home().as_posix()]
    for value in private_strings:
        for encoding in ["utf-8", "utf-16-le"]:
            if value.lower().encode(encoding) in native.lower():
                raise SystemExit("Release executable contains an unremapped private source path")
    print("Release executable: " + sha(native), flush=True)
    if args.build_only:
        return
    subprocess.run([sys.executable, "scripts/build_workbook.py", "--source-ref", revision], cwd=ROOT, check=True)
    subprocess.run([sys.executable, "scripts/check_workbook.py"], cwd=ROOT, check=True)
    metadata = json.loads(run(["cargo", "metadata", "--locked", "--format-version=1",
                              "--filter-platform", target], text=True))
    package = next(item for item in metadata["packages"] if item["name"] == "divulge" and item["source"] is None)
    version = package["version"]
    fallback_notices = {item["crate"]: item for item in json.loads(
        (ROOT / "third-party/notice-sources.json").read_text(encoding="utf-8"))}
    name = "DIVULGE-{}-{}".format(version, TARGETS[target][1])
    destination = ROOT / "output/release"
    destination.mkdir(parents=True, exist_ok=True)
    (ROOT / "target").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="divulge-release-", dir=ROOT / "target") as temp:
        staging = pathlib.Path(temp) / name
        staging.mkdir()
        archive = zipfile.ZipFile(io.BytesIO(run(["git", "archive", "--format=zip", revision])))
        archive.extractall(staging)
        if sys.platform == "darwin":
            app_bundle(staging, executable, version)
            native = (staging / "DIVULGE.app/Contents/MacOS/divulge").read_bytes()
        else:
            shutil.copy2(executable, staging / executable.name)
        pdf = ROOT / "output/pdf/DIVULGE-workshop.pdf"
        shutil.copy2(pdf, staging / "workshop/DIVULGE-workshop.pdf")
        licenses = []
        for dependency in metadata["packages"]:
            if dependency["source"] is None:
                continue
            directory = pathlib.Path(dependency["manifest_path"]).parent
            candidates = {p for p in directory.rglob("*") if p.is_file() and
                          p.name.lower().startswith(("license", "licence", "copying", "notice", "ofl", "copyright"))}
            if dependency.get("license_file"):
                declared = (directory / dependency["license_file"]).resolve()
                if not declared.is_relative_to(directory.resolve()):
                    raise SystemExit("Dependency notice is outside its package")
                if declared.is_file():
                    candidates.add(declared)
            notice_root = staging / "licenses" / (dependency["name"] + "-" + dependency["version"])
            files = []
            for item in sorted(candidates):
                if item.stat().st_size > 2 * 1024 * 1024:
                    raise SystemExit("Unexpectedly large dependency notice")
                notice = notice_root / item.relative_to(directory)
                notice.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(item, notice)
                files.append(notice.relative_to(staging).as_posix())
            if not files:
                fallback = fallback_notices.get(dependency["name"] + "-" + dependency["version"])
                if not fallback:
                    raise SystemExit("No dependency notices for " + dependency["name"])
                vcs = json.loads((directory / ".cargo_vcs_info.json").read_text(encoding="utf-8"))
                if vcs["git"]["sha1"] != fallback["upstream_commit"]:
                    raise SystemExit("Notice revision does not match the packaged crate")
                for item in fallback["files"]:
                    source = (ROOT / item["path"]).resolve()
                    if not source.is_relative_to((ROOT / "third-party/notices").resolve()):
                        raise SystemExit("Notice path is outside the reviewed snapshot")
                    if sha(source.read_bytes()) != item["sha256"]:
                        raise SystemExit("Dependency notice hash mismatch")
                    files.append(item["path"])
            licenses.append({"name": dependency["name"], "version": dependency["version"],
                             "license": dependency["license"], "repository": dependency["repository"],
                             "notice_files": files})
        (staging / "licenses").mkdir(exist_ok=True)
        (staging / "licenses/dependencies.json").write_text(json.dumps(licenses, indent=2) + "\n", encoding="utf-8")
        import reportlab
        font_notice = pathlib.Path(reportlab.__file__).parent / "fonts/bitstream-vera-license.txt"
        shutil.copy2(font_notice, staging / "licenses/bitstream-vera-license.txt")
        identity = {"version": version, "source_commit": revision, "target": target,
                    "rustc": run(["rustc", "--version"], text=True).strip(),
                    "executable_sha256": sha(native), "workbook_sha256": sha(pdf.read_bytes()),
                    "platform_status": "Native build; see native-smoke.json and docs/native-validation.md for runtime coverage",
                    "signature": "local ad-hoc; not Developer ID signed or notarized" if sys.platform == "darwin" else "unsigned",
                    "contains_firmware": False, "contains_vendor_tools": False}
        (staging / "release.json").write_text(json.dumps(identity, indent=2) + "\n", encoding="utf-8")
        checksums = []
        for file in sorted(staging.rglob("*")):
            if not file.is_file():
                continue
            relative = file.relative_to(staging).as_posix()
            if file.suffix.lower() in {".fwsc", ".bin", ".elf", ".key", ".pem"} or file.name.startswith(".env"):
                raise SystemExit("Excluded artifact in package: " + relative)
            checksums.append(sha(file.read_bytes()) + "  " + relative)
        (staging / "SHA256SUMS.txt").write_text("\n".join(checksums) + "\n", encoding="utf-8")
        zip_path = destination / (name + ".zip")
        with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as bundle:
            for file in sorted(staging.rglob("*")):
                if file.is_file():
                    entry = zipfile.ZipInfo(name + "/" + file.relative_to(staging).as_posix(), (2026, 1, 1, 0, 0, 0))
                    entry.compress_type = zipfile.ZIP_DEFLATED
                    entry.create_system = 3
                    entry.external_attr = (0o100755 if file.stat().st_mode & 0o111 else 0o100644) << 16
                    bundle.writestr(entry, file.read_bytes())
        shutil.copy2(pdf, destination / "DIVULGE-workshop.pdf")
        artifacts = [zip_path, destination / "DIVULGE-workshop.pdf"]
        (destination / "SHA256SUMS.txt").write_text("\n".join(sha(p.read_bytes()) + "  " + p.name for p in artifacts) + "\n", encoding="utf-8")
        print(json.dumps({"archive": str(zip_path), "identity": identity, "files": len(checksums) + 1,
                          "dependency_notices": len(licenses)}, indent=2))


if __name__ == "__main__":
    main()
