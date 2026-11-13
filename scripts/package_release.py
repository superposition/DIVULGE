#!/usr/bin/env python3
"""Build a clean, path-remapped Windows release with source, workbook and notices."""
import argparse
import hashlib
import io
import json
import os
import pathlib
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-only", action="store_true")
    args = parser.parse_args()
    if sys.platform != "win32":
        raise SystemExit("This release recipe has only been verified on Windows.")
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
    subprocess.run(["cargo", "build", "--release", "--locked", "--bin", "divulge"],
                   cwd=ROOT, env=environment, check=True)
    executable = ROOT / "target/release/divulge.exe"
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
                              "--filter-platform", "x86_64-pc-windows-msvc"], text=True))
    package = next(item for item in metadata["packages"] if item["name"] == "divulge" and item["source"] is None)
    version = package["version"]
    name = "DIVULGE-{}-windows-x86_64".format(version)
    destination = ROOT / "output/release"
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="divulge-release-", dir=ROOT / "target") as temp:
        staging = pathlib.Path(temp) / name
        staging.mkdir()
        archive = zipfile.ZipFile(io.BytesIO(run(["git", "archive", "--format=zip", revision])))
        archive.extractall(staging)
        shutil.copy2(executable, staging / "divulge.exe")
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
            target = staging / "licenses" / (dependency["name"] + "-" + dependency["version"])
            files = []
            for item in sorted(candidates):
                if item.stat().st_size > 2 * 1024 * 1024:
                    raise SystemExit("Unexpectedly large dependency notice")
                notice = target / item.relative_to(directory)
                notice.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(item, notice)
                files.append(notice.relative_to(staging).as_posix())
            licenses.append({"name": dependency["name"], "version": dependency["version"],
                             "license": dependency["license"], "repository": dependency["repository"],
                             "notice_files": files})
        (staging / "licenses").mkdir(exist_ok=True)
        (staging / "licenses/dependencies.json").write_text(json.dumps(licenses, indent=2) + "\n", encoding="utf-8")
        import reportlab
        font_notice = pathlib.Path(reportlab.__file__).parent / "fonts/bitstream-vera-license.txt"
        shutil.copy2(font_notice, staging / "licenses/bitstream-vera-license.txt")
        identity = {"version": version, "source_commit": revision, "target": "x86_64-pc-windows-msvc",
                    "rustc": run(["rustc", "--version"], text=True).strip(),
                    "executable_sha256": sha(native), "workbook_sha256": sha(pdf.read_bytes()),
                    "platform_status": "Windows native verified; Linux/macOS unverified",
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
                    bundle.writestr(entry, file.read_bytes())
        shutil.copy2(pdf, destination / "DIVULGE-workshop.pdf")
        artifacts = [zip_path, destination / "DIVULGE-workshop.pdf"]
        (destination / "SHA256SUMS.txt").write_text("\n".join(sha(p.read_bytes()) + "  " + p.name for p in artifacts) + "\n", encoding="utf-8")
        print(json.dumps({"archive": str(zip_path), "identity": identity, "files": len(checksums) + 1,
                          "dependency_notices": len(licenses)}, indent=2))


if __name__ == "__main__":
    main()
