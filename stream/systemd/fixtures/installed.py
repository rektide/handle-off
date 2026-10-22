#!/usr/bin/env python3
"""Retain installed-reader succession, independent-reader and refusal evidence."""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def json_file(path, value):
    with path.open("x") as output:
        json.dump(value, output, indent=2)
        output.write("\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--client", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    parser.add_argument("--fixture", type=Path,
                        default=Path(__file__).with_name("journal.jsonl"))
    args = parser.parse_args()
    client = args.client.resolve(strict=True)
    fixture = args.fixture.resolve(strict=True)
    args.scratch.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="installed-", dir=args.scratch.resolve()))
    transcript, relationships = [], []
    outcome = {"schema": 1, "result": "failed", "evidence": str(run / "evidence.json")}

    def snapshot(attempt, phase, checkpoints):
        result = {}
        for name, path in checkpoints.items():
            if path.exists():
                data = path.read_bytes()
                artifact = f"{phase}-{name}.json"
                with (attempt / artifact).open("xb") as output:
                    output.write(data)
                result[name] = {"artifact": artifact, "bytes": len(data),
                                "sha256": hashlib.sha256(data).hexdigest()}
            else:
                result[name] = {"absent": True}
        return result

    def invoke(label, arguments, checkpoints=None, success=True):
        checkpoints = checkpoints or {}
        attempt = run / f"attempt-{len(transcript):03}-{label}"
        attempt.mkdir()
        argv = [str(run / "ho-journal"), *map(str, arguments)]
        request = {"argv": argv, "expected_success": success,
                   "before": snapshot(attempt, "before", checkpoints)}
        json_file(attempt / "request.json", request)
        entry = {"attempt": attempt.name, "label": label, "result": "incomplete"}
        transcript.append(entry)
        child = None
        try:
            with (attempt / "stdout.bin").open("xb") as stdout, \
                    (attempt / "stderr.bin").open("xb") as stderr:
                child = subprocess.Popen(argv, cwd=run, stdin=subprocess.DEVNULL,
                                         stdout=stdout, stderr=stderr)
                entry["pid"] = child.pid
                try:
                    child.wait(timeout=30)
                except BaseException:
                    child.kill()
                    child.wait()
                    raise
            entry["exit_code"] = child.returncode
            entry["result"] = "exited"
            assert (child.returncode == 0) == success, f"unexpected exit; inspect {attempt}"
            return attempt.name, (attempt / "stdout.bin").read_bytes()
        except BaseException as error:
            entry["error"] = f"{type(error).__name__}: {error}"
            raise
        finally:
            entry["after"] = snapshot(attempt, "after", checkpoints)
            json_file(attempt / "outcome.json", entry)

    def rows(data):
        return [json.loads(line) for line in data.splitlines()]

    try:
        shutil.copy2(client, run / "ho-journal")
        shutil.copyfile(fixture, run / "input.jsonl")
        outcome["inputs"] = {
            "client": str(client),
            "client_sha256": hashlib.sha256(client.read_bytes()).hexdigest(),
            "fixture": str(fixture),
            "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest(),
        }
        source = {"backend": {"kind": "fixture", "path": str(run / "input.jsonl")},
                  "machine_id": "a" * 32, "unit": "worker.service",
                  "unit_field": "systemd_unit", "selection": {"kind": "follow_unit"},
                  "matches": {}}
        source_path = run / "source.json"
        json_file(source_path, source)
        a, b = run / "reader a #.json", run / "reader-b.json"
        invoke("version", ["--version"])
        prefix_attempt, prefix = invoke("a-prefix", ["read", "--source", source_path,
            "--checkpoint", a, "--start", "head", "--limit", "2"], {"a": a})
        prefix = rows(prefix)
        assert [row["event"]["__CURSOR"] for row in prefix] == ["fixture:1", "fixture:2"]
        _, reference = invoke("a-reference", ["reference", "--checkpoint", a], {"a": a})
        reference = reference.decode().strip()
        assert "%23" in reference
        successor_attempt, suffix = invoke("a-successor", ["resume", "--checkpoint", reference], {"a": a})
        suffix = rows(suffix)
        assert [row["observation"] for row in suffix] == ["invocation_changed", "boot_changed", "unknown"]
        relationships.append({"kind": "reader-process-succession", "prefix": prefix_attempt,
                              "successor": successor_attempt, "checkpoint_reference": reference,
                              "service_authority": "not-established"})
        b_attempt, all_rows = invoke("b-independent", ["read", "--source", source_path,
            "--checkpoint", b, "--start", "head", "--limit", "100"], {"a": a, "b": b})
        all_rows = rows(all_rows)
        assert prefix + suffix == all_rows
        expected = [event for event in rows((run / "input.jsonl").read_bytes())
                    if event.get("_MACHINE_ID") == source["machine_id"]
                    and event.get("_SYSTEMD_UNIT") == source["unit"]]
        assert [row["event"] for row in all_rows] == expected
        assert prefix[0]["identity"]["pid"] != prefix[1]["identity"]["pid"]
        assert prefix[0]["identity"]["pid"] == suffix[0]["identity"]["pid"]
        assert prefix[0]["identity"]["invocation"] != suffix[0]["identity"]["invocation"]
        assert suffix[0]["identity"]["boot_id"] != suffix[1]["identity"]["boot_id"]
        assert prefix[1]["event"]["MESSAGE"] == [0, 255, 10]
        assert prefix[1]["event"]["TAG"] == ["one", "two"]
        assert suffix[-1]["event"]["MESSAGE"] is None
        relationships.append({"kind": "independent-reader", "attempt": b_attempt,
                              "compared_to": [prefix_attempt, successor_attempt]})
        _, empty = invoke("a-caught-up", ["resume", "--checkpoint", reference], {"a": a})
        assert empty == b""
        _, status = invoke("a-status", ["status", "--checkpoint", reference], {"a": a})
        assert json.loads(status)["emitted"] == 5

        changed = {**source, "matches": {"_BOOT_ID": "c" * 32}}
        changed_path = run / "other-source.json"
        json_file(changed_path, changed)
        before = a.read_bytes()
        _, output = invoke("wrong-source", ["resume", "--source", changed_path,
            "--checkpoint", reference], {"a": a}, success=False)
        assert output == b"" and a.read_bytes() == before

        missing = run / "missing-cursor.json"
        state = json.loads(before)
        state["cursor"] = "fixture:unavailable"
        json_file(missing, state)
        before_missing = missing.read_bytes()
        _, output = invoke("missing-cursor", ["resume", "--checkpoint", missing],
                           {"missing": missing}, success=False)
        assert output == b"" and missing.read_bytes() == before_missing
        offline = {**source, "backend": {"kind": "fixture", "path": str(run / "absent.jsonl")}}
        offline_path = run / "offline-source.json"
        json_file(offline_path, offline)
        _, description = invoke("describe-offline", ["describe", "--source", offline_path])
        assert json.loads(description)["backend_profile"] == "journal-jsonl-v1"
        assert not (run / "absent.jsonl").exists()
        outcome.update(result="passed", selected_events=len(expected),
                       limitation="Synthetic journalctl-JSON fixture; no native journal or service takeover proved.")
    except BaseException as error:
        outcome["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        json_file(run / "evidence.json", {**outcome, "attempts": transcript,
                                          "relationships": relationships})
        print(json.dumps(outcome, indent=2), file=sys.stderr)


if __name__ == "__main__":
    main()
