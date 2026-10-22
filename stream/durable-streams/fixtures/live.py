#!/usr/bin/env python3
"""Run a real filesystem server, kill/restart it, and retain inspectable evidence."""
import argparse
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", type=Path, required=True)
    parser.add_argument("--client", type=Path, required=True)
    parser.add_argument("--scratch", type=Path, required=True)
    args = parser.parse_args()
    server, client = args.server.resolve(), args.client.resolve()
    args.scratch.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix="probe-", dir=args.scratch.resolve()))
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    base = f"http://127.0.0.1:{port}"
    root, fork = base + "/v1/stream/root", base + "/v1/stream/fork"
    config = run / "server.toml"
    config.write_text(f'[server]\nbind_address = "127.0.0.1:{port}"\n'
                      f'[storage]\nmode = "file"\ndata_dir = {json.dumps(str(run / "store"))}\n')
    env = {k: v for k, v in os.environ.items() if not k.startswith("DS_")}
    process = None
    log = (run / "server.log").open("wb")
    transcript = []
    outcome = {"result": "failed", "evidence": str(run / "evidence.json")}
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

    def start():
        nonlocal process
        process = subprocess.Popen([str(server), "--config", str(config), "serve"], cwd=run, env=env, stdout=log, stderr=log)
        for _ in range(100):
            if process.poll() is not None:
                raise RuntimeError(f"server exited; inspect {run / 'server.log'}")
            try:
                with socket.create_connection(("127.0.0.1", port), timeout=.1):
                    return
            except OSError:
                time.sleep(.05)
        raise RuntimeError("server startup timeout")

    def stop():
        nonlocal process
        if process is not None:
            if process.poll() is None:
                process.kill()  # process crash, not graceful server shutdown
            process.wait(timeout=10)
            process = None

    def invoke(argv, data=None):
        attempt = run / f"attempt-{len(transcript):03}"
        attempt.mkdir()
        request = {"argv": argv, "input_bytes": len(data) if data else 0}
        (attempt / "request.json").write_text(json.dumps(request, indent=2) + "\n")
        (attempt / "stdin.bin").write_bytes(data or b"")
        entry = {"attempt": attempt.name, **request}
        transcript.append(entry)
        with (attempt / "stdout.bin").open("xb") as stdout, (attempt / "stderr.bin").open("xb") as stderr:
            result = subprocess.run(argv, input=data or b"", stdout=stdout, stderr=stderr,
                                    cwd=run, env=env, timeout=40)
        entry["exit_code"] = result.returncode
        (attempt / "outcome.json").write_text(json.dumps(entry, indent=2) + "\n")
        if result.returncode:
            raise RuntimeError(f"command failed; inspect {attempt}")
        return (attempt / "stdout.bin").read_bytes()

    def cli(*command, data=None):
        return invoke([str(client), *command], data)

    def read(url, name):
        return cli("read", url, "--state", str(run / name), "--incarnation", "fixture-generation-1")

    try:
        start()
        cli("create", root)
        cli("append", root, data=b"alpha\n")
        assert read(root, "a.json") == b"alpha\n"
        cli("append", root, data=b"beta\n")
        assert read(root, "a.json") == b"beta\n"
        assert read(root, "b.json") == b"alpha\nbeta\n"
        cli("head", root)
        cli("create", fork, "--fork-from", "/v1/stream/root")
        cli("append", fork, data=b"child\n")
        assert read(fork, "fork.json") == b"alpha\nbeta\nchild\n"
        # Delete the parent's public name; fork prefix must remain readable.
        with opener.open(urllib.request.Request(root, method="DELETE"), timeout=5) as response:
            transcript.append({"delete_parent_status": response.status})
        stop()
        start()
        assert read(fork, "fresh-fork.json") == b"alpha\nbeta\nchild\n"
        assert read(fork, "fork.json") == b""
        cli("append", fork, data=b"after-restart\n")
        assert read(fork, "fork.json") == b"after-restart\n"
        cli("status", "--state", str(run / "fork.json"))
        stop()
        for command in [("list", "--json"), ("export", "--output", str(run / "export.json"))]:
            invoke([str(server), "--config", str(config), *command])
        files = []
        for path in sorted((run / "store").rglob("*")):
            if path.is_file():
                entry = {"path": str(path.relative_to(run)), "bytes": path.stat().st_size}
                if path.name == "meta.json":
                    entry["metadata"] = json.loads(path.read_text())
                if path.name == "data.log":
                    data, records = path.read_bytes(), []
                    while data:
                        assert len(data) >= 4
                        size = struct.unpack("<I", data[:4])[0]
                        assert len(data) >= 4 + size
                        records.append(data[4:4 + size].decode())
                        data = data[4 + size:]
                    entry["length_prefixed_records"] = records
                files.append(entry)
        outcome.update(result="passed", files=files)
    except BaseException as error:
        outcome["error"] = f"{type(error).__name__}: {error}"
        raise
    finally:
        stop()
        log.close()
        with (run / "evidence.json").open("x") as evidence:
            json.dump({**outcome, "transcript": transcript}, evidence, indent=2)
            evidence.write("\n")
        print(json.dumps({k: v for k, v in outcome.items() if k != "files"}, indent=2), file=sys.stderr)


if __name__ == "__main__":
    main()
