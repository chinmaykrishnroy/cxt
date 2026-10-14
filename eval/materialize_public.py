"""Materialize pinned public pre-fix trees; never execute upstream build/test code."""
import argparse
import concurrent.futures
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys


def relative(value):
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "\\" in value or not path.parts:
        raise ValueError("unsafe task path")
    return path


def git(directory, *arguments):
    environment = dict(os.environ, GIT_CONFIG_GLOBAL=os.devnull, GIT_CONFIG_NOSYSTEM="1",
                       GIT_TERMINAL_PROMPT="0", GIT_LFS_SKIP_SMUDGE="1")
    hooks = directory.parent / ".cxt-control-hooks"
    hooks.mkdir(parents=True, exist_ok=True)
    if any(hooks.iterdir()):
        raise ValueError("benchmark hook directory must be empty")
    command = ["git", "-c", "core.autocrlf=false", "-c", "core.hooksPath=" + str(hooks.resolve()),
               "-c", "core.fsmonitor=false", "-C", str(directory), *arguments]
    result = subprocess.run(command, capture_output=True, timeout=180, env=environment)
    if result.returncode:
        raise RuntimeError("git operation failed: " + result.stderr.decode("utf-8", errors="replace")[-2000:])
    return result.stdout


def initialize(path, repository):
    if path.exists():
        if not (path / ".git").is_dir():
            raise ValueError("existing destination is not a benchmark checkout")
        if git(path, "remote", "get-url", "origin").decode().strip() != repository + ".git":
            raise ValueError("existing checkout has a different remote")
        if git(path, "status", "--porcelain"):
            raise ValueError("existing checkout is dirty; refusing to overwrite it")
        return False
    path.mkdir(parents=True)
    git(path, "init", "-q")
    # Repository .gitattributes can force CRLF even with core.autocrlf=false.
    # This checkout is a raw-byte retrieval fixture, not an upstream build tree.
    attributes = path / ".git" / "info" / "attributes"
    attributes.parent.mkdir(exist_ok=True)
    attributes.write_text("* -text -filter -ident -working-tree-encoding\n", encoding="utf-8")
    git(path, "remote", "add", "origin", repository + ".git")
    return True


def materialize(task, destination):
    name = task["id"]
    repository = task["repository"]
    if not re.fullmatch(r"[a-z0-9][a-z0-9-]{0,96}", name):
        raise ValueError("invalid task identifier")
    if not re.fullmatch(r"https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("only canonical public GitHub repository URLs are accepted")
    base, fix = task["base_commit"], task["fix_commit"]
    if not all(re.fullmatch(r"[a-f0-9]{40}", sha) for sha in (base, fix)):
        raise ValueError("commits must be full SHA-1 object identities")
    evidence = destination / "evidence" / name
    inputs = destination / "inputs" / name
    initialize(evidence, repository)
    git(evidence, "fetch", "--depth=2", "--no-tags", "--recurse-submodules=no", "origin", fix)
    parents = git(evidence, "rev-list", "--parents", "-n", "1", fix).decode().split()
    if parents != [fix, base]:
        raise ValueError("recorded fix is not a single-parent child of the recorded base")
    fresh = initialize(inputs, repository)
    if fresh:
        git(inputs, "fetch", "--depth=1", "--no-tags", "--recurse-submodules=no", "origin", base)
        tree = git(inputs, "ls-tree", "-r", base)
        if any(line.startswith((b"120000 ", b"160000 ")) for line in tree.splitlines()):
            raise ValueError("task contains symlinks/submodules; explicit sandbox review is required")
        git(inputs, "checkout", "--detach", "FETCH_HEAD")
    if git(inputs, "rev-parse", "HEAD").decode().strip() != base:
        raise ValueError("input checkout revision mismatch; refusing to change existing checkout")
    license_path = str(relative(task["license"]["path"]))
    license_bytes = git(inputs, "show", base + ":" + license_path)
    if not license_bytes:
        raise ValueError("license missing at pinned base")
    labels = []
    for label in task["required_source_spans"]:
        path = str(relative(label["path"]))
        original = git(inputs, "show", base + ":" + path)
        local_path = inputs.joinpath(*PurePosixPath(path).parts)
        if original != local_path.read_bytes():
            raise ValueError("checkout changed original source bytes")
        original.decode("utf-8")
        lines = original.splitlines(keepends=True)
        start, end = label["start_line"], label["end_line"]
        if not (1 <= start <= end <= len(lines)):
            raise ValueError("label is outside the source revision")
        if sum(map(len, lines[:start-1])) != label["start_byte"] or sum(map(len, lines[:end])) != label["end_byte"]:
            raise ValueError("research label byte coordinates do not match pinned raw source")
        labels.append(dict(path=path, name=label["name"], start_line=start, end_line=end,
                           source_sha256=hashlib.sha256(original).hexdigest()))
    return dict(id=name, root="inputs/" + name, query=task["query"],
                required_files=task["required_files"], required_symbols=labels,
                label_status=task["label_status"], admitted_for_scoring=task["admitted_for_scoring"],
                repository=repository, base_commit=base,
                license_sha256=hashlib.sha256(license_bytes).hexdigest())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, default=Path(__file__).with_name("public-tasks.json"))
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--download", action="store_true", help="explicitly fetch public repositories")
    parser.add_argument("--include-provisional", action="store_true", help="allow exploratory non-gold labels; quarantines stay excluded")
    parser.add_argument("--limit", type=int, default=30)
    options = parser.parse_args()
    manifest = json.loads(options.manifest.read_text(encoding="utf-8"))
    if manifest["schema"] != "cxt.public-task-research.v1" or not 1 <= options.limit <= 30:
        raise ValueError("invalid manifest or task limit")
    tasks = [t for t in manifest["tasks"] if not t["label_status"].startswith("quarantined")
             and (t["admitted_for_scoring"] or options.include_provisional)][:options.limit]
    if not tasks:
        raise ValueError("no admitted gold tasks; exploratory runs require --include-provisional")
    if not options.download:
        print(json.dumps(dict(plan_only=True, task_ids=[t["id"] for t in tasks]), indent=2))
        return
    destination = options.output.resolve()
    if destination == Path(destination.anchor) or destination == Path.home() or destination == Path.cwd():
        raise ValueError("choose a dedicated benchmark output directory, not a broad directory")
    destination.mkdir(parents=True, exist_ok=True)
    failures, completed = [], []
    with concurrent.futures.ThreadPoolExecutor(max_workers=3) as executor:
        futures = {executor.submit(materialize, task, destination): task["id"] for task in tasks}
        for future in concurrent.futures.as_completed(futures):
            name = futures[future]
            try:
                completed.append(future.result())
                print("Verified " + name, flush=True)
            except Exception as error:
                failures.append(dict(task=name, error=str(error)))
                print("Failed " + name + ": " + str(error), file=sys.stderr, flush=True)
    completed.sort(key=lambda t: t["id"])
    corpus = dict(schema="cxt.eval.v1", dataset_kind="provisional public bug-fix source-span candidates; NOT independently reviewed gold or downstream task success",
                  budgets=manifest["budgets"], tasks=completed, materialization_failures=failures,
                  excluded_quarantines=[t["id"] for t in manifest["tasks"] if t["label_status"].startswith("quarantined")])
    for filename, data in [("corpus.json", corpus), ("materialization.json", dict(completed=len(completed), failures=failures))]:
        path = destination / filename
        temporary = path.with_suffix(".tmp")
        temporary.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        temporary.replace(path)
    return 1 if failures else 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print("materialization: " + str(error), file=sys.stderr)
        raise SystemExit(2)
