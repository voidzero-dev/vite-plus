#!/usr/bin/env python3
"""Compare two macOS ARM64 vp binaries with isolated, reproducible workloads.

The mirror directory must contain the official Node 22.13.1 Darwin ARM64
tarball and its signed SHASUMS256.txt.asc under v22.13.1/. All timed downloads
use a loopback server. Signature and archive checksum verification stay enabled.
"""

import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import platform
import random
import resource
import shutil
import statistics
import subprocess
import tempfile
import threading
import time


VERSION = "22.13.1"
ARCHIVE = f"node-v{VERSION}-darwin-arm64.tar.gz"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def cpu_time():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--mirror", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=120)
    parser.add_argument("--install-samples", type=int, default=10)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--seed", type=int, default=20261016)
    args = parser.parse_args()
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        parser.error("This fixture uses a macOS ARM64 Node.js archive")
    if min(args.samples, args.install_samples, args.rounds) < 1:
        parser.error("Sample and round counts must be positive")

    files = {
        f"/v{VERSION}/{name}": (args.mirror / f"v{VERSION}" / name).read_bytes()
        for name in [ARCHIVE, "SHASUMS256.txt.asc"]
    }
    requests = []

    class Mirror(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            requests.append(self.path)
            data = files.get(self.path)
            if data is None:
                self.send_error(404)
                return
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def log_message(self, *_):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Mirror)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        with tempfile.TemporaryDirectory(prefix="vp-cli-benchmark-") as temp:
            root = Path(temp)
            vp_home = root / "vp-home"
            vp_home.mkdir()
            project = root / "project"
            project.mkdir()
            (project / "package.json").write_text('{"name":"cli-benchmark","private":true}\n')
            (project / ".node-version").write_text(VERSION + "\n")
            deep = project.joinpath(*[f"level{i}" for i in range(20)])
            deep.mkdir(parents=True)
            binaries = {}
            binary_metadata = {}
            for label, source in [("baseline", args.baseline), ("candidate", args.candidate)]:
                directory = root / label
                directory.mkdir()
                binary = directory / "vp"
                shutil.copy2(source.resolve(), binary)
                (directory / ".vp-setup-complete").touch()
                binaries[label] = binary
                binary_metadata[label] = {"bytes": binary.stat().st_size, "sha256": digest(binary)}

            # Do not inherit shell selections, test hooks, or signature bypasses.
            env = {
                key: value for key, value in os.environ.items()
                if not key.startswith(("VP_", "VITE_", "TOKIO_"))
            }
            env.update({
                "VP_HOME": str(vp_home),
                "VP_NO_UPDATE_CHECK": "1",
                "VP_NODE_DIST_MIRROR": f"http://127.0.0.1:{server.server_port}",
                "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
                "NO_COLOR": "1",
                "CI": "1",
            })
            env.pop("FORCE_COLOR", None)
            env.pop("RUST_LOG", None)

            def execute(label, argv, cwd):
                start_cpu = cpu_time()
                start = time.perf_counter_ns()
                result = subprocess.run(
                    [str(binaries[label]), *argv], cwd=cwd, env=env,
                    stdin=subprocess.DEVNULL, capture_output=True, timeout=90,
                )
                elapsed_ms = (time.perf_counter_ns() - start) / 1e6
                cpu_ms = (cpu_time() - start_cpu) * 1000
                if result.returncode:
                    raise RuntimeError(
                        f"{label} {argv}: {result.returncode}\n"
                        f"{result.stdout.decode()}\n{result.stderr.decode()}"
                    )
                # Paths are identical for both profiles but temporary across runs.
                output = (result.stdout + b"\0" + result.stderr).replace(str(root).encode(), b"<FIXTURE>")
                return {"wall_ms": elapsed_ms, "cpu_ms": cpu_ms}, hashlib.sha256(output).hexdigest()

            # Install a real, verified runtime once before steady-state measurements.
            execute("baseline", ["env", "install", VERSION], project)
            runtime = vp_home / "js_runtime" / "node" / VERSION
            node_hash = digest(runtime / "bin" / "node")
            cases = [
                ("help", ["--help"], project),
                ("env-current", ["env", "current", "node", "--json"], project),
                ("env-current-deep", ["env", "current", "node", "--json"], deep),
                ("env-which", ["env", "which", "node"], project),
                ("node-startup", ["node", "-e", ""], project),
                ("npm-version", ["env", "exec", "npm", "--version"], project),
                ("install-cached", ["env", "install", VERSION], project),
                ("install-fresh", ["env", "install", VERSION], project),
            ]
            rng = random.Random(args.seed)
            results = {}
            for name, argv, cwd in cases:
                results[name] = {"argv": argv, "cwd_depth": 20 if cwd == deep else 0, "rounds": []}
                expected_hash = None
                count = args.install_samples if name == "install-fresh" else args.samples
                for round_index in range(args.rounds):
                    samples = {"baseline": [], "candidate": []}
                    orders = [i % 2 == 0 for i in range(count)]
                    rng.shuffle(orders)
                    request_count = len(requests)
                    for iteration in range(-3, count):
                        baseline_first = iteration % 2 == 0 if iteration < 0 else orders[iteration]
                        order = ["baseline", "candidate"] if baseline_first else ["candidate", "baseline"]
                        for label in order:
                            if name == "install-fresh":
                                shutil.rmtree(runtime)
                            sample, output_hash = execute(label, argv, cwd)
                            if expected_hash is None:
                                expected_hash = output_hash
                            if output_hash != expected_hash:
                                raise RuntimeError(f"Output mismatch: {name}, {label}")
                            if name == "install-fresh" and digest(runtime / "bin" / "node") != node_hash:
                                raise RuntimeError("Installed Node.js binary differs")
                            if iteration >= 0:
                                samples[label].append(sample)
                    request_delta = len(requests) - request_count
                    expected_requests = (count + 3) * 2 * 2 if name == "install-fresh" else 0
                    if request_delta != expected_requests:
                        raise RuntimeError(
                            f"Unexpected mirror traffic: {name}: "
                            f"{request_delta} vs {expected_requests}"
                        )
                    medians = {
                        label: statistics.median(s["wall_ms"] for s in values)
                        for label, values in samples.items()
                    }
                    change = (medians["candidate"] / medians["baseline"] - 1) * 100
                    results[name]["rounds"].append({
                        "samples": samples, "wall_medians_ms": medians, "change_percent": change,
                    })
                    print(
                        f"{name} round {round_index + 1}: {medians['baseline']:.3f} -> "
                        f"{medians['candidate']:.3f} ms ({change:+.1f}%)", flush=True,
                    )
                results[name]["output_sha256"] = expected_hash
                summary = {}
                for label in binaries:
                    samples = [s for r in results[name]["rounds"] for s in r["samples"][label]]
                    summary[label] = {
                        "count": len(samples),
                        "wall_median_ms": statistics.median(s["wall_ms"] for s in samples),
                        "cpu_median_ms": statistics.median(s["cpu_ms"] for s in samples),
                    }
                baseline_ms = summary["baseline"]["wall_median_ms"]
                candidate_ms = summary["candidate"]["wall_median_ms"]
                summary["change_ms"] = candidate_ms - baseline_ms
                summary["change_percent"] = (candidate_ms / baseline_ms - 1) * 100
                results[name]["summary"] = summary

            # Keep the raw paired observations without repeating field names.
            for case in results.values():
                for round_result in case["rounds"]:
                    round_result["samples"] = {
                        label: [[s["wall_ms"], s["cpu_ms"]] for s in samples]
                        for label, samples in round_result["samples"].items()
                    }
            report = {
                "source_commit": subprocess.check_output(
                    ["git", "rev-parse", "HEAD"], text=True,
                ).strip(),
                "platform": platform.platform(),
                "cpu": subprocess.check_output(
                    ["sysctl", "-n", "machdep.cpu.brand_string"], text=True,
                ).strip(),
                "python": platform.python_version(),
                "seed": args.seed,
                "binaries": binary_metadata,
                "node_archive_bytes": len(files[f"/v{VERSION}/{ARCHIVE}"]),
                "node_archive_sha256": hashlib.sha256(files[f"/v{VERSION}/{ARCHIVE}"]).hexdigest(),
                "installed_node_sha256": node_hash,
                "method": (
                    "Balanced randomized adjacent pairs; three warmups per variant per round; "
                    "wall time includes process launch and output capture; CPU time includes "
                    "children. Fresh installs use loopback HTTP with PGP/SHA-256 verification "
                    "and extraction; removal and output/hash checks are outside the timer. "
                    "OS caches remain warm."
                ),
                "sample_columns": ["wall_ms", "cpu_ms"],
                "results": results,
            }
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


if __name__ == "__main__":
    main()
