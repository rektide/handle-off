#!/usr/bin/env python3
"""Run a real filesystem server, kill/restart it, and retain inspectable evidence."""
import argparse
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
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

    def cli(*command, data=None):
        result = subprocess.run([str(client), *command], input=data, capture_output=True, timeout=40, check=True)
        transcript.append({"command": command, "stdout": result.stdout.decode(), "stderr": result.stderr.decode()})
        return result.stdout

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
        with urllib.request.urlopen(urllib.request.Request(root, method="DELETE"), timeout=5) as response:
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
            result = subprocess.run([str(server), "--config", str(config), *command], cwd=run, env=env, capture_output=True, check=True)
            transcript.append({"server_command": command, "stdout": result.stdout.decode()})
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
        (run / "evidence.json").write_text(json.dumps({"transcript": transcript, "files": files}, indent=2) + "\n")
        print(json.dumps({"result": "passed", "evidence": str(run / "evidence.json")}, indent=2))
    finally:
        stop()
        log.close()


if __name__ == "__main__":
    main()
