#!/usr/bin/env python3
"""Compare standalone vp profiles through the same bundled Vite and Rolldown.

Use a built packages/cli directory and the src directory from three.js r108,
the revision used by Rolldown's own threejs10x benchmark. No downloads occur
during the benchmark. Both binaries use the supplied Node.js executable.
"""

import argparse
import hashlib
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
import time


CONFIG = """
import { writeFileSync } from 'node:fs';

export default {
  build: {
    target: 'es2015',
    minify: process.env.BENCH_MINIFY === '1' ? 'oxc' : false,
    sourcemap: process.env.BENCH_SOURCEMAP === '1',
    reportCompressedSize: false,
    chunkSizeWarningLimit: 100000,
  },
  plugins: process.env.BENCH_PROBE ? [{
    name: 'benchmark-probe',
    generateBundle(_options, bundle) {
      const chunks = Object.values(bundle).filter(item => item.type === 'chunk');
      writeFileSync(process.env.BENCH_PROBE, JSON.stringify({
        node: process.version,
        nodeExecutable: process.execPath,
        rolldown: this.meta.rolldownVersion,
        modules: [...this.getModuleIds()].length,
        chunks: chunks.length,
        dynamicChunks: chunks.filter(chunk => chunk.isDynamicEntry).length,
        nativeLibraries: process.report.getReport().sharedObjects.filter(name => name.endsWith('.node')),
      }));
    },
  }] : [],
};
"""


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_manifest(root):
    files = [
        {"path": str(path.relative_to(root)), "bytes": path.stat().st_size, "sha256": digest(path)}
        for path in sorted(root.rglob("*")) if path.is_file()
    ]
    return {
        "files": len(files),
        "bytes": sum(item["bytes"] for item in files),
        "sha256": hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest(),
    }


def cpu_time():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def make_project(project, cli, node_version):
    project.mkdir()
    (project / "node_modules").mkdir()
    (project / "node_modules/vite-plus").symlink_to(cli, target_is_directory=True)
    (project / "package.json").write_text('{"name":"rolldown-benchmark","private":true,"type":"module"}\n')
    (project / ".node-version").write_text(node_version + "\n")
    (project / "vite.config.mjs").write_text(CONFIG)
    (project / "index.html").write_text('<!doctype html><html><body><script type="module" src="/entry.ts"></script></body></html>\n')


def make_three(project, source):
    entries = []
    for index in range(10):
        shutil.copytree(source, project / f"copy{index}")
        entries.append(f"import * as three{index} from './copy{index}/Three.js';")
    entries.append("globalThis.benchmark = [" + ",".join(f"three{i}" for i in range(10)) + "];")
    (project / "entry.ts").write_text("\n".join(entries) + "\n")


def make_typescript(project):
    source = project / "src"
    source.mkdir()
    (source / "shared.ts").write_text("export const adjust = (n: number): number => Math.sin(n) * 0.5;\n")
    for index in range(2000):
        lines = ["import { adjust } from './shared';", "interface Input { value?: number; label?: string }"]
        for function in range(12):
            lines.append(
                f"export function f{function}(input: Input): string {{"
                f" const value = (input.value ?? {index + function}) + adjust({function} + (input.value ?? 0));"
                f" return (input.label?.toUpperCase() ?? 'module{index}') + ':' + value.toFixed({function % 5}); }}"
            )
        lines.append(f"export function unused() {{ return 'unused-sentinel-{index}'; }}")
        (source / f"module{index}.ts").write_text("\n".join(lines) + "\n")
    for chunk in range(20):
        lines = []
        calls = []
        for index in range(chunk * 100, (chunk + 1) * 100):
            names = [f"f{function} as m{index}f{function}" for function in range(12)]
            lines.append(f"import {{ {', '.join(names)} }} from './module{index}';")
            calls.extend(f"m{index}f{function}(input)" for function in range(12))
        lines.append("export const run = (input: { value?: number; label?: string }) => [" + ",".join(calls) + "];")
        (source / f"chunk{chunk}.ts").write_text("\n".join(lines) + "\n")
    (project / "entry.ts").write_text(
        "globalThis.benchmark = [" + ",".join(f"() => import('./src/chunk{i}')" for i in range(20)) + "];\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--binary-source-commit", required=True)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--node", type=Path, required=True)
    parser.add_argument("--three-src", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=20)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--seed", type=int, default=20261017)
    args = parser.parse_args()
    if min(args.samples, args.rounds) < 1 or args.samples % 2:
        parser.error("Use a positive, even sample count and a positive round count")
    node = args.node.resolve()
    cli = args.cli.resolve()
    three_src = args.three_src.resolve()
    core = Path(subprocess.check_output([
        str(node), "--input-type=module", "-e",
        "import { createRequire } from 'node:module'; console.log(createRequire(process.argv[1]).resolve('vite/package.json'));",
        str(cli / "package.json"),
    ], text=True).strip()).parent
    node_version = subprocess.check_output([str(node), "--version"], text=True).strip().removeprefix("v")
    if not (three_src / "Three.js").is_file():
        parser.error("--three-src must contain Three.js from three.js r108")
    binaries_metadata = {
        label: {"bytes": path.stat().st_size, "sha256": digest(path)}
        for label, path in [("baseline", args.baseline), ("candidate", args.candidate)]
    }
    report = {
        "binary_source_commit": args.binary_source_commit,
        "runner_sha256": digest(Path(__file__)),
        "platform": platform.platform(),
        "cpu": subprocess.check_output(["sysctl", "-n", "machdep.cpu.brand_string"], text=True).strip()
        if platform.system() == "Darwin" else platform.processor(),
        "binaries": binaries_metadata,
        "node": {"version": node_version, "sha256": digest(node)},
        "toolchain": json.loads((cli / "dist/toolchain.json").read_text()),
        "cli_dist": tree_manifest(cli / "dist"),
        "core_dist": tree_manifest(core / "dist"),
        "three_source": tree_manifest(three_src),
        "seed": args.seed,
        "method": (
            "Fresh vp build process per sample; same JS, NAPI and Node for both profiles; "
            "balanced randomized adjacent pairs; three warmups per variant per round; "
            "output directory removed before each invocation; warm OS caches; no task cache. "
            "Wall time includes process launch, CLI/Node startup, bundling, minification, "
            "source maps and output writes as configured. Cleanup and output hash checks "
            "are outside the timer. Probe hooks run only in untimed preflight builds. "
            "Confidence interval bootstraps paired observations within each round."
        ),
        "sample_columns": ["wall_ms", "cpu_ms"],
        "results": {},
    }
    rng = random.Random(args.seed)
    with tempfile.TemporaryDirectory(prefix="vp-rolldown-benchmark-") as temp:
        root = Path(temp)
        home = root / "vp-home"
        home.mkdir()
        # Use a fixed Node executable without downloads or user configuration.
        (home / "config.json").write_text('{"nodeShimMode":"system_first"}\n')
        node_bin = root / "node-bin"
        node_bin.mkdir()
        (node_bin / "node").symlink_to(node)
        env = {
            key: value for key, value in os.environ.items()
            if not key.startswith(("VP_", "VITE_", "TOKIO_", "NAPI_", "BENCH_"))
            and key not in ("NODE_OPTIONS", "NODE_PATH", "FORCE_COLOR", "RUST_LOG", "RAYON_NUM_THREADS")
        }
        env.update({"VP_HOME": str(home), "VP_NO_UPDATE_CHECK": "1", "NO_COLOR": "1", "CI": "1",
                    "PATH": str(node_bin) + os.pathsep + "/usr/bin:/bin:/usr/sbin:/sbin"})
        binaries = {}
        for label, source in [("baseline", args.baseline), ("candidate", args.candidate)]:
            directory = root / label
            directory.mkdir()
            binaries[label] = directory / "vp"
            shutil.copy2(source.resolve(), binaries[label])
            (directory / ".vp-setup-complete").touch()
        three = root / "three10x"
        typescript = root / "typescript"
        for project in [three, typescript]:
            make_project(project, cli, node_version)
        make_three(three, three_src)
        make_typescript(typescript)
        cases = [
            ("three10x-bundle", three, False, False),
            ("three10x-minify-sourcemap", three, True, True),
            ("typescript-code-split-minify-sourcemap", typescript, True, True),
        ]
        for name, project, minify, sourcemap in cases:
            case_env = {**env, "BENCH_MINIFY": str(int(minify)), "BENCH_SOURCEMAP": str(int(sourcemap))}
            expected_output = None
            probes = {}

            def execute(label, probe=False):
                nonlocal expected_output
                shutil.rmtree(project / "dist", ignore_errors=True)
                run_env = dict(case_env)
                if probe:
                    run_env["BENCH_PROBE"] = str(root / "probe.json")
                start_cpu = cpu_time()
                start = time.perf_counter_ns()
                result = subprocess.run(
                    [str(binaries[label]), "build", "--logLevel", "silent"], cwd=project,
                    env=run_env, stdin=subprocess.DEVNULL, capture_output=True, timeout=120,
                )
                sample = [(time.perf_counter_ns() - start) / 1e6, (cpu_time() - start_cpu) * 1000]
                if result.returncode:
                    raise RuntimeError(f"{name} {label}: {result.stdout.decode()}\n{result.stderr.decode()}")
                output = tree_manifest(project / "dist")
                if output["files"] < (3 if sourcemap else 2):
                    raise RuntimeError(f"Missing build output: {name}")
                if expected_output is None:
                    expected_output = output
                if output != expected_output:
                    raise RuntimeError(f"Build output differs: {name}, {label}")
                if probe:
                    data = json.loads((root / "probe.json").read_text())
                    if not data.get("rolldown") or Path(data["nodeExecutable"]).resolve() != node:
                        raise RuntimeError(f"Unexpected bundler/runtime: {data}")
                    native = [Path(path) for path in data.pop("nativeLibraries") if "vite-plus" in path]
                    if not native:
                        raise RuntimeError("The build did not load the bundled Vite+ native addon")
                    data["native_addons"] = sorted({digest(path) for path in native})
                    data.pop("nodeExecutable")
                    probes[label] = data
                return sample

            for label in binaries:
                execute(label, probe=True)
            if probes["baseline"] != probes["candidate"]:
                raise RuntimeError(f"Different bundlers/runtimes: {probes}")
            case = {"minify": minify, "sourcemap": sourcemap, "probe": probes["baseline"], "rounds": []}
            print(f"{name}: {case['probe']}; output={expected_output}", flush=True)
            for round_index in range(args.rounds):
                samples = {"baseline": [], "candidate": []}
                orders = [i % 2 == 0 for i in range(args.samples)]
                rng.shuffle(orders)
                for iteration in range(-3, args.samples):
                    first = iteration % 2 == 0 if iteration < 0 else orders[iteration]
                    for label in (["baseline", "candidate"] if first else ["candidate", "baseline"]):
                        sample = execute(label)
                        if iteration >= 0:
                            samples[label].append(sample)
                medians = {label: statistics.median(s[0] for s in values) for label, values in samples.items()}
                change = (medians["candidate"] / medians["baseline"] - 1) * 100
                case["rounds"].append({"samples": samples, "baseline_first": orders,
                                       "wall_medians_ms": medians, "change_percent": change})
                print(f"{name} round {round_index + 1}: {medians['baseline']:.3f} -> {medians['candidate']:.3f} ms ({change:+.2f}%)", flush=True)
            summary = {}
            for label in binaries:
                values = [sample for result in case["rounds"] for sample in result["samples"][label]]
                summary[label] = {"count": len(values), "wall_median_ms": statistics.median(s[0] for s in values),
                                  "cpu_median_ms": statistics.median(s[1] for s in values)}
            summary["change_percent"] = (summary["candidate"]["wall_median_ms"] / summary["baseline"]["wall_median_ms"] - 1) * 100
            boot_rng = random.Random(args.seed)
            estimates = []
            for _ in range(10000):
                baseline, candidate = [], []
                for result in case["rounds"]:
                    for index in boot_rng.choices(range(args.samples), k=args.samples):
                        baseline.append(result["samples"]["baseline"][index][0])
                        candidate.append(result["samples"]["candidate"][index][0])
                estimates.append((statistics.median(candidate) / statistics.median(baseline) - 1) * 100)
            estimates.sort()
            summary["change_percent_95pct_bootstrap_ci"] = [estimates[250], estimates[9749]]
            case.update({"summary": summary, "output": expected_output})
            report["results"][name] = case
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
