#!/usr/bin/env python3
"""Compile/run every lesson, verify the deliberate failure, and reconstruct the replay."""
import json
import pathlib
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]

def run(args, check=True):
    result = subprocess.run(args, cwd=ROOT, text=True, encoding="utf-8", errors="replace", capture_output=True)
    if check and result.returncode:
        raise RuntimeError("Command failed: " + " ".join(map(str, args)) + "\n" + result.stderr)
    return result

def main():
    started = time.monotonic()
    lessons = json.loads((ROOT / "workshop/lessons.json").read_text(encoding="utf-8"))["lessons"]
    for lesson in lessons:
        before = time.monotonic()
        result = run(["cargo", "run", "--quiet", "--locked", "--example", lesson["slug"]])
        if result.stdout != lesson["stdout"]:
            raise AssertionError(lesson["slug"] + " output differs from the lesson\n" + result.stdout)
        print("{}: expected output ({:.2f}s)".format(lesson["slug"], time.monotonic() - before))
    metadata = json.loads(run(["cargo", "metadata", "--no-deps", "--format-version=1"]).stdout)
    with tempfile.TemporaryDirectory(prefix="divulge-workshop-") as temporary:
        directory = pathlib.Path(temporary)
        # Cargo selects the toolchain, library and transitive dependencies together.
        # Direct rustc linking can pick an incompatible rlib on hosted Windows.
        manifest = directory / "Cargo.toml"
        manifest.write_text(
            '[package]\nname = "divulge-workshop-check"\nversion = "0.0.0"\nedition = "2024"\n'
            '[dependencies]\ndivulge = { path = ' + json.dumps(ROOT.as_posix()) + ' }\n'
            '[profile.dev]\ndebug = 1\n' + ''.join(
                '[[bin]]\nname = "' + name + '"\npath = ' +
                json.dumps((ROOT / "workshop/exercises" / (name + ".rs")).as_posix()) + '\n'
                for name in ["wrong_coordinate", "coordinate_answer"]), encoding="utf-8")
        (directory / "Cargo.lock").write_bytes((ROOT / "Cargo.lock").read_bytes())
        common = ["--offline", "--manifest-path", str(manifest),
                  "--target-dir", metadata["target_directory"]]
        for name, must_fail in [("wrong_coordinate", True), ("coordinate_answer", False)]:
            args = ["cargo", "build", *common, "--bin", name, "--message-format=json"]
            result = run(args, check=False)
            if must_fail:
                messages = [json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
                diagnostics = [item["message"] for item in messages if item.get("reason") == "compiler-message"]
                errors = [d for d in diagnostics if d.get("level") == "error"]
                if result.returncode == 0 or not any(d.get("code", {}).get("code") == "E0308" for d in errors if d.get("code")):
                    raise AssertionError("Expected the coordinate type mismatch\n" + result.stdout + result.stderr)
                if not all(word in json.dumps(errors) for word in ["FileOffset", "MappedAddress"]):
                    raise AssertionError("Wrong error: coordinate names missing")
                print("wrong_coordinate: expected E0308; no executable produced")
            else:
                if result.returncode:
                    raise AssertionError(result.stderr)
                if run(["cargo", "run", "--quiet", "--locked", *common, "--bin", name]).stdout != "Ok(4660)\n":
                    raise AssertionError("Instructor answer output changed")
                print("coordinate_answer: compiled and returned Ok(4660)")
        generated = directory / "fixtures"
        run(["cargo", "run", "--quiet", "--locked", "--example", "prepare_workshop", "--", str(generated)])
        for name in ["synthetic-workshop.json", "synthetic-review.json"]:
            expected = (ROOT / "workshop/fixtures" / name).read_bytes()
            if (generated / name).read_bytes() != expected:
                raise AssertionError(name + " is stale; run prepare_workshop")
        run(["cargo", "run", "--quiet", "--locked", "--example", "validate_session", "--", str(generated / "synthetic-workshop.json")])
        print("Synthetic replay/export: deterministic; offline reconstruction matches")
    print("Workshop checks passed in {:.2f}s. This is a technical check, not a timed spoken rehearsal.".format(time.monotonic() - started))

if __name__ == "__main__":
    main()
