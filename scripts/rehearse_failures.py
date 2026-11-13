#!/usr/bin/env python3
"""Exercise unavailable OMP and queued cancellation without loading auth or a model."""
import argparse
import json
import os
import pathlib
import shutil
import subprocess
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--firmware", type=pathlib.Path, help="Optional local exact-V15 missing-decoder exercise")
    args = parser.parse_args()
    bundled = pathlib.Path.home() / ".bun/bin" / ("bun.exe" if os.name == "nt" else "bun")
    bun = os.environ.get("DIVULGE_BUN") or (str(bundled) if bundled.is_file() else shutil.which("bun"))
    if not bun:
        raise SystemExit("Installed Bun is required for this host-only rehearsal; no installation was attempted.")
    with tempfile.TemporaryDirectory(prefix="divulge-failure-") as temporary:
        directory = pathlib.Path(temporary)
        bridge = directory / "bridge.ts"
        bridge.write_bytes((ROOT / "adapters/omp_bridge.ts").read_bytes())
        environment = os.environ.copy()
        environment["DIVULGE_OMP_SDK"] = str(directory / "deliberately-absent-sdk")
        first = {"v": 1, "id": "rehearsal-unavailable", "op": "discover", "context": {}}
        for cancelled in [False, True]:
            payload = json.dumps(first) + "\n"
            if cancelled:
                payload += json.dumps({"v": 1, "id": first["id"], "op": "cancel"}) + "\n"
            result = subprocess.run([bun, str(bridge)], input=payload, text=True, capture_output=True,
                                    cwd=directory, env=environment, timeout=10)
            events = [json.loads(line) for line in result.stdout.splitlines() if line.strip()]
            expected = "cancelled" if cancelled else "error"
            assert events and events[-1]["kind"] == expected, result.stdout + result.stderr
            assert [e["seq"] for e in events] == list(range(1, len(events) + 1))
            assert all(e["id"] == first["id"] for e in events)
            assert not any(e["kind"] in {"ready", "proposal", "completed", "evidence_read"} for e in events)
            print("{}: {} / no SDK, auth, model or evidence call".format(
                "Queued cancellation" if cancelled else "Unavailable OMP SDK", expected))
        if args.firmware:
            result = subprocess.run(["cargo", "run", "--quiet", "--locked", "--example", "disassemble", "--",
                                     str(args.firmware.resolve()), "--tool", "/divulge-intentionally-absent/objdump"],
                                    cwd=ROOT, text=True, capture_output=True, timeout=60)
            assert result.returncode == 0, result.stderr
            assert "Failed / 0 rows" in result.stdout, result.stdout
            assert "validated profile false" in result.stdout, result.stdout
            print("Missing vendor decoder: failed receipt / zero rows / no validated instruction semantics")
    print("Failure rehearsal completed; global tool/model configuration was not changed.")


if __name__ == "__main__":
    main()
