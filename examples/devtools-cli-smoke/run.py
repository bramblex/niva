#!/usr/bin/env python3
"""Exercise the built Devtools CLI through a real Niva Native binary."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import signal
import subprocess
import time
import uuid


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DIST = REPO_ROOT / "packages/devtools/build"
DEFAULT_CONFIG = REPO_ROOT / "packages/devtools/niva.json"
KIT_STORAGE_KEY = "niva-devtools-packager-kit"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def tree_digest(root: Path) -> dict:
    records = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        relative = path.relative_to(root).as_posix()
        records.append({"path": relative, "sha256": sha256_file(path)})
    joined = "\n".join(f'{item["path"]}:{item["sha256"]}' for item in records)
    return {"sha256": hashlib.sha256(joined.encode()).hexdigest(), "files": len(records)}


def write_devtools_config(template: Path, path: Path, name: str) -> str:
    config = json.loads(template.read_text(encoding="utf-8"))
    app_uuid = str(uuid.uuid4())
    config["name"] = name
    config["uuid"] = app_uuid
    config.setdefault("window", {})["visible"] = False
    path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")
    return app_uuid


def create_project(path: Path, valid_config: bool) -> None:
    path.mkdir(parents=True)
    if not valid_config:
        return
    resources = path / "resources"
    resources.mkdir()
    (resources / "index.html").write_text(
        "<!doctype html><meta charset='utf-8'><title>Devtools CLI fixture</title>\n",
        encoding="utf-8",
    )
    project = {
        "name": "Devtools CLI fixture",
        "uuid": str(uuid.uuid4()),
        "build": {"resource": "resources"},
    }
    (path / "niva.json").write_text(json.dumps(project, indent=2) + "\n", encoding="utf-8")


def structured_records(log_path: Path) -> list:
    records = []
    for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict) and value.get("status") in {"complete", "failed"}:
            records.append(value)
    return records


def stop_process(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    if os.name == "posix":
        try:
            os.killpg(process.pid, signal.SIGTERM)
        except ProcessLookupError:
            return
    else:
        process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        if os.name == "posix":
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        else:
            process.kill()
        process.wait(timeout=5)


def run_case(
    binary: Path,
    devtools_dist: Path,
    config_template: Path,
    case_root: Path,
    project_is_valid: bool,
    expected_error: str,
    timeout_seconds: int,
) -> dict:
    case_root.mkdir(parents=True)
    project = case_root / "project"
    create_project(project, project_is_valid)
    app_config = case_root / "devtools.niva.json"
    app_uuid = write_devtools_config(config_template, app_config, f"Devtools CLI smoke {case_root.name}")
    output = case_root / "build-output"
    stdout_path = case_root / "stdout.log"
    stderr_path = case_root / "stderr.log"
    command = [
        str(binary),
        f"--config={app_config}",
        f"--resource={devtools_dist}",
        f"--build={output}",
        f"--project={project}",
    ]

    timed_out = False
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            command,
            cwd=project,
            stdout=stdout,
            stderr=stderr,
            start_new_session=(os.name == "posix"),
        )
        try:
            exit_code = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            stop_process(process)
            exit_code = process.returncode
    elapsed_ms = round((time.monotonic() - started) * 1000)

    stderr_records = structured_records(stderr_path)
    stdout_records = structured_records(stdout_path)
    expected_record = next((item for item in stderr_records if item.get("status") == "failed"), None)
    error_text = str(expected_record.get("error", "")) if expected_record else ""
    no_output_artifact = not output.exists()
    ok = (
        not timed_out
        and exit_code == 1
        and len([item for item in stderr_records if item.get("status") == "failed"]) == 1
        and not any(item.get("status") == "failed" for item in stdout_records)
        and expected_error in error_text
        and no_output_artifact
    )
    return {
        "name": case_root.name,
        "ok": ok,
        "exitCode": exit_code,
        "timedOut": timed_out,
        "timeoutSeconds": timeout_seconds,
        "elapsedMs": elapsed_ms,
        "stoppedBySignal": signal.Signals(-exit_code).name if exit_code is not None and exit_code < 0 else None,
        "isolatedDevtoolsUuid": app_uuid,
        "devtoolsResourceTree": tree_digest(devtools_dist),
        "expectedErrorContains": expected_error,
        "structuredStderr": expected_record,
        "stdoutRecords": stdout_records,
        "noBuildOutputCreated": no_output_artifact,
        "command": command,
        "stdoutPath": str(stdout_path),
        "stderrPath": str(stderr_path),
    }


def seed_kit_selection(source_dist: Path, destination: Path, kit_directory: Path | None) -> None:
    shutil.copytree(source_dist, destination)
    index_path = destination / "index.html"
    index = index_path.read_text(encoding="utf-8")
    head = re.search(r"<head\b[^>]*>", index, flags=re.IGNORECASE)
    if not head:
        raise ValueError("Devtools dist index.html has no head element for test storage setup")
    seed_file = destination / "devtools-cli-smoke-seed.js"
    if kit_directory is None:
        raise ValueError("Kit selection seeding requires a real test kit")
    setup = f"localStorage.setItem({json.dumps(KIT_STORAGE_KEY)}, {json.dumps(str(kit_directory))});\n"
    seed_file.write_text(setup, encoding="utf-8")
    script = '<script src="/devtools-cli-smoke-seed.js"></script>\n'
    index_path.write_text(index[: head.end()] + "\n" + script + index[head.end() :], encoding="utf-8")


def native_build_target() -> str:
    if os.name == "nt":
        return "windows-x86_64"
    if platform.system() == "Darwin":
        return "macos-aarch64" if platform.machine().lower() in {"arm64", "aarch64"} else "macos-x86_64"
    raise ValueError(f"Unsupported Native CLI acceptance host: {platform.system()}")


def run_success_case(
    binary: Path,
    source_dist: Path,
    config_template: Path,
    kit_directory: Path,
    case_root: Path,
    timeout_seconds: int,
) -> dict:
    kit_manifest_path = kit_directory / "manifest.json"
    manifest = json.loads(kit_manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schemaVersion") != 1 or not isinstance(manifest.get("runtimes"), dict):
        raise ValueError("Kit manifest must use schemaVersion 1 and declare runtimes")
    expected_target = native_build_target()
    if expected_target not in manifest["runtimes"]:
        raise ValueError(f"The kit does not contain the host build runtime {expected_target}")
    host_packager = kit_directory / ("niva-packager.exe" if os.name == "nt" else "niva-packager")
    if not host_packager.is_file() or not os.access(host_packager, os.X_OK):
        raise ValueError(f"Kit has no executable host packager: {host_packager}")

    case_root.mkdir(parents=True)
    seeded_dist = case_root / "devtools-resource"
    seed_kit_selection(source_dist, seeded_dist, kit_directory)
    project = case_root / "project"
    create_project(project, True)
    app_config = case_root / "devtools.niva.json"
    app_uuid = write_devtools_config(config_template, app_config, "Devtools CLI success smoke")
    output = case_root / "build-output"
    stdout_path = case_root / "stdout.log"
    stderr_path = case_root / "stderr.log"
    command = [
        str(binary),
        f"--config={app_config}",
        f"--resource={seeded_dist}",
        f"--build={output}",
        f"--project={project}",
    ]

    timed_out = False
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            command,
            cwd=project,
            stdout=stdout,
            stderr=stderr,
            start_new_session=(os.name == "posix"),
        )
        try:
            exit_code = process.wait(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            stop_process(process)
            exit_code = process.returncode
    elapsed_ms = round((time.monotonic() - started) * 1000)

    stdout_records = structured_records(stdout_path)
    stderr_records = structured_records(stderr_path)
    result = next((item for item in stdout_records if item.get("status") == "complete"), None)
    artifact = Path(result["path"]) if result and result.get("path") else None
    artifact_exists = bool(artifact and artifact.is_file())
    artifact_actual_sha256 = sha256_file(artifact) if artifact_exists and artifact else None
    artifact_hash_matches = bool(result and artifact_actual_sha256 and result.get("sha256") == artifact_actual_sha256)
    ok = (
        not timed_out
        and exit_code == 0
        and result is not None
        and result.get("target") == expected_target
        and result.get("resourceLayout") == "embedded"
        and artifact_exists
        and artifact_hash_matches
    )
    return {
        "name": case_root.name,
        "ok": ok,
        "exitCode": exit_code,
        "timedOut": timed_out,
        "timeoutSeconds": timeout_seconds,
        "elapsedMs": elapsed_ms,
        "stoppedBySignal": signal.Signals(-exit_code).name if exit_code is not None and exit_code < 0 else None,
        "isolatedDevtoolsUuid": app_uuid,
        "devtoolsResourceTree": tree_digest(seeded_dist),
        "kitDirectory": str(kit_directory),
        "kitManifestSha256": sha256_file(kit_manifest_path),
        "kitRuntimes": manifest["runtimes"],
        "expectedTarget": expected_target,
        "hostPackager": str(host_packager),
        "hostPackagerSha256": sha256_file(host_packager),
        "structuredStdout": result,
        "stderrRecords": stderr_records,
        "artifactExists": artifact_exists,
        "artifactActualSha256": artifact_actual_sha256,
        "artifactHashMatchesPackagerReport": artifact_hash_matches,
        "artifactPath": str(artifact) if artifact else None,
        "command": command,
        "stdoutPath": str(stdout_path),
        "stderrPath": str(stderr_path),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, help="New macOS Native Niva binary under acceptance")
    parser.add_argument("--devtools-dist", default=str(DEFAULT_DIST), help="Built Devtools resource directory")
    parser.add_argument("--config-template", default=str(DEFAULT_CONFIG), help="Devtools niva.json template")
    parser.add_argument("--output", required=True, help="New directory for fixture files, logs, and result.json")
    parser.add_argument("--kit-directory", help="Optional real single-host kit; enables the success build case")
    parser.add_argument(
        "--confirm-isolated-storage",
        action="store_true",
        help="Allow the success fixture to seed localStorage only after Native UUID storage isolation is confirmed",
    )
    parser.add_argument("--timeout-seconds", type=int, default=90, help="Per-case process timeout")
    args = parser.parse_args()

    binary = Path(args.binary).resolve()
    devtools_dist = Path(args.devtools_dist).resolve()
    config_template = Path(args.config_template).resolve()
    output = Path(args.output).resolve()
    kit_directory = Path(args.kit_directory).resolve() if args.kit_directory else None
    if not binary.is_file() or not os.access(binary, os.X_OK):
        parser.error(f"Native binary is missing or not executable: {binary}")
    if not (devtools_dist / "index.html").is_file():
        parser.error(f"Devtools dist is missing index.html; build it first: {devtools_dist}")
    if not config_template.is_file():
        parser.error(f"Devtools config template does not exist: {config_template}")
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    if output.exists():
        parser.error(f"Use a fresh evidence directory; it already exists: {output}")
    if kit_directory and not (kit_directory / "manifest.json").is_file():
        parser.error(f"Kit is missing manifest.json: {kit_directory}")
    if kit_directory and not args.confirm_isolated_storage:
        parser.error("--kit-directory requires --confirm-isolated-storage after Native UUID WebView storage isolation is verified")

    output.mkdir(parents=True)
    source_dist_digest = tree_digest(devtools_dist)
    binary_digest = sha256_file(binary)
    cases = [
        run_case(
            binary,
            devtools_dist,
            config_template,
            output / "missing-project-config",
            project_is_valid=False,
            expected_error="No niva.json found",
            timeout_seconds=args.timeout_seconds,
        ),
        run_case(
            binary,
            devtools_dist,
            config_template,
            output / "missing-kit",
            project_is_valid=True,
            expected_error="Select a packaging kit",
            timeout_seconds=args.timeout_seconds,
        ),
    ]
    if kit_directory:
        cases.append(
            run_success_case(
                binary,
                devtools_dist,
                config_template,
                kit_directory,
                output / "real-packager-success",
                timeout_seconds=args.timeout_seconds,
            )
        )

    report = {
        "ok": all(case["ok"] for case in cases),
        "nativeBinary": str(binary),
        "nativeBinarySha256": binary_digest,
        "devtoolsDist": str(devtools_dist),
        "devtoolsDistTree": source_dist_digest,
        "kitDirectory": str(kit_directory) if kit_directory else None,
        "engine": "real Niva Native WebView",
        "cases": cases,
    }
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
