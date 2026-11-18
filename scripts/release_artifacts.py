#!/usr/bin/env python3
"""Create and authenticate raw macOS build artifact receipts."""

import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import struct
import subprocess
import sys

REPO = "ClickHouse/clickhousectl"
REPO_ID = 1150809931
OWNER_ID = 54801242
WORKFLOW = REPO + "/.github/workflows/macos-build.yml"
PREDICATE = "https://slsa.dev/provenance/v1"
TARGETS = {"aarch64-apple-darwin": 0x0100000C, "x86_64-apple-darwin": 0x01000007}
MAX_BINARY = 128 * 1024 * 1024
MAX_RECEIPT = 1024 * 1024
HEX40 = re.compile(r"[0-9a-f]{40}\Z")
HEX64 = re.compile(r"[0-9a-f]{64}\Z")
REF = re.compile(r"refs/tags/v[0-9]+\.[0-9]+\.[0-9]+\Z")


class ProvenanceError(Exception):
    pass


def fail(message):
    raise ProvenanceError(message)


def command(args):
    try:
        result = subprocess.run(
            args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True
        )
        return result.stdout.decode("utf-8")
    except (OSError, subprocess.CalledProcessError, UnicodeError) as exc:
        raise ProvenanceError("command failed: " + args[0]) from exc


def github_api(path, *, paginate=False):
    if not isinstance(path, str) or not path or path.startswith("-"):
        fail("invalid API path")
    args = ["gh", "api", "--hostname", "github.com", "--method", "GET"]
    if paginate:
        args += ["--paginate", "--slurp"]
    try:
        data = json.loads(command(args + [path]))
    except (ValueError, TypeError) as exc:
        raise ProvenanceError("invalid GitHub API JSON") from exc
    if paginate:
        if not isinstance(data, list) or any(
            not isinstance(page, list) for page in data
        ):
            fail("invalid paginated API response")
        return [item for page in data for item in page]
    if not isinstance(data, (dict, list)):
        fail("invalid API response")
    return data


def positive(value):
    return type(value) is int and value > 0


def check_context(source, ref, run, builder):
    if (
        not isinstance(source, str)
        or not HEX40.fullmatch(source)
        or not isinstance(builder, str)
        or not HEX40.fullmatch(builder)
    ):
        fail("invalid source or builder SHA")
    if not isinstance(ref, str) or not REF.fullmatch(ref) or not positive(run):
        fail("invalid ref or run")


def regular(path, limit):
    try:
        st = path.lstat()
    except OSError as exc:
        raise ProvenanceError("missing file") from exc
    if not stat.S_ISREG(st.st_mode) or st.st_size > limit:
        fail("file is not regular or exceeds size limit")
    return st.st_size


def read_bounded(path, limit):
    regular(path, limit)
    # O_NOFOLLOW closes the lstat/open symlink race on supported runner OSes.
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            st = os.fstat(stream.fileno())
            if not stat.S_ISREG(st.st_mode) or st.st_size > limit:
                fail("invalid file")
            data = stream.read(limit + 1)
    except OSError as exc:
        raise ProvenanceError("cannot read file") from exc
    if len(data) > limit or len(data) != st.st_size:
        fail("file changed or is oversized")
    return data


def binary_info(path, target):
    regular(path, MAX_BINARY)
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            st = os.fstat(stream.fileno())
            if not stat.S_ISREG(st.st_mode) or not 32 <= st.st_size <= MAX_BINARY:
                fail("invalid binary size or type")
            header = stream.read(32)
            if (
                len(header) != 32
                or header[:4] != b"\xcf\xfa\xed\xfe"
                or struct.unpack_from("<I", header, 4)[0] != TARGETS[target]
            ):
                fail("invalid Mach-O architecture")
            digest = hashlib.sha256(header)
            size = len(header)
            while chunk := stream.read(1024 * 1024):
                size += len(chunk)
                if size > MAX_BINARY:
                    fail("binary exceeds size limit")
                digest.update(chunk)
    except OSError as exc:
        raise ProvenanceError("cannot read binary") from exc
    if size != st.st_size:
        fail("binary changed while being read")
    return {"sha256": digest.hexdigest(), "size": size, "cputype": TARGETS[target]}


def inventory(directory):
    try:
        if not directory.is_dir() or directory.is_symlink():
            fail("invalid directory")
        return {p.name: p for p in directory.iterdir()}
    except OSError as exc:
        raise ProvenanceError("cannot enumerate directory") from exc


def artifact(id_, source, run, target, digest):
    if not positive(id_) or not isinstance(digest, str) or not HEX64.fullmatch(digest):
        fail("invalid artifact ID or digest")
    obj = github_api(f"repos/{REPO}/actions/artifacts/{id_}")
    if not isinstance(obj, dict):
        fail("invalid artifact response")
    wr = obj.get("workflow_run")
    if (
        not isinstance(wr, dict)
        or not positive(wr.get("id"))
        or wr["id"] != run
        or wr.get("head_sha") != source
    ):
        fail("artifact run or source mismatch")
    if (
        not positive(obj.get("id"))
        or obj["id"] != id_
        or obj.get("expired") is not False
        or obj.get("digest") != "sha256:" + digest
    ):
        fail("artifact identity, expiration or digest mismatch")
    name = obj.get("name")
    if not isinstance(name, str) or not re.fullmatch(
        re.escape(f"clickhousectl-{target}-{run}-") + r"[1-9][0-9]*", name
    ):
        fail("artifact name mismatch")
    # GitHub has returned repository identity both at the top level and in workflow_run.
    for container in (obj, wr):
        for key in ("repository_id", "head_repository_id"):
            if key in container and container[key] != REPO_ID:
                fail("artifact repository mismatch")
        for key in ("repository", "head_repository"):
            if key in container:
                repo = container[key]
                if not isinstance(repo, dict) or repo.get("id") != REPO_ID:
                    fail("artifact repository mismatch")
                if "owner" in repo and (
                    not isinstance(repo["owner"], dict)
                    or repo["owner"].get("id") != OWNER_ID
                ):
                    fail("artifact owner mismatch")
    return name


def lock_digest(source):
    obj = github_api(f"repos/{REPO}/contents/Cargo.lock?ref={source}")
    if (
        not isinstance(obj, dict)
        or obj.get("encoding") != "base64"
        or not isinstance(obj.get("content"), str)
    ):
        fail("invalid Cargo.lock response")
    try:
        content = base64.b64decode("".join(obj["content"].split()), validate=True)
    except (ValueError, TypeError) as exc:
        raise ProvenanceError("invalid Cargo.lock encoding") from exc
    return hashlib.sha256(content).hexdigest()


def receipt(directory, source, ref, run, attempt, builder, inputs):
    check_context(source, ref, run, builder)
    if not positive(attempt) or set(inputs) != set(TARGETS):
        fail("invalid receipt inputs")
    ids = [inputs[t][0] for t in TARGETS]
    if len(set(ids)) != 2:
        fail("artifact IDs must be distinct")
    files = inventory(directory)
    if len(files) != 2:
        fail("expected exactly two artifacts")
    for path in files.values():
        regular(path, MAX_BINARY)
    records = {}
    for target in TARGETS:
        id_, digest = inputs[target]
        name = artifact(id_, source, run, target, digest)
        if name not in files:
            fail("artifact file missing")
        info = binary_info(files[name], target)
        if info["sha256"] != digest:
            fail("artifact content digest mismatch")
        records[target] = {"artifact_id": id_, "name": name, **info}
    result = {
        "schema": 1,
        "repository": REPO,
        "repository_id": REPO_ID,
        "owner_id": OWNER_ID,
        "source_sha": source,
        "source_ref": ref,
        "builder_workflow": WORKFLOW,
        "builder_sha": builder,
        "run_id": run,
        "run_attempt": attempt,
        "cargo_lock_sha256": lock_digest(source),
        "targets": records,
    }
    path = directory / f"macos-build-receipt-{run}-{attempt}.json"
    try:
        with path.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, sort_keys=True, separators=(",", ":"))
            stream.write("\n")
    except OSError as exc:
        raise ProvenanceError("cannot create receipt") from exc
    return result


def gh_version():
    text = command(["gh", "--version"])
    match = re.match(r"gh version (\d+)\.(\d+)\.(\d+)(?:\s|$)", text)
    if not match or tuple(map(int, match.groups())) < (2, 102, 0):
        fail("gh 2.102.0 or newer required")


def verify_attestation(path, sha, source, ref, run, digest):
    identity = "https://github.com/" + WORKFLOW + "@" + sha
    args = [
        "gh",
        "attestation",
        "verify",
        str(path),
        "--hostname",
        "github.com",
        "--repo",
        REPO,
        "--cert-identity",
        identity,
        "--signer-digest",
        sha,
        "--source-digest",
        source,
        "--source-ref",
        ref,
        "--cert-oidc-issuer",
        "https://token.actions.githubusercontent.com",
        "--predicate-type",
        PREDICATE,
        "--deny-self-hosted-runners",
        "--format",
        "json",
    ]
    try:
        results = json.loads(command(args))
    except (ValueError, TypeError) as exc:
        raise ProvenanceError("invalid attestation verification output") from exc
    if not isinstance(results, list):
        fail("invalid attestation results")
    for entry in results:
        try:
            result = entry["verificationResult"]
            cert = result["signature"]["certificate"]
            statement = result["statement"]
            invocation = cert["runInvocationURI"]
            if (
                cert["subjectAlternativeName"] == identity
                and cert["buildSignerURI"] == identity
                and cert["buildSignerDigest"] == sha
                and cert["sourceRepositoryURI"] == "https://github.com/" + REPO
                and cert["sourceRepositoryIdentifier"] == str(REPO_ID)
                and cert["sourceRepositoryOwnerIdentifier"] == str(OWNER_ID)
                and cert["sourceRepositoryDigest"] == source
                and cert["sourceRepositoryRef"] == ref
                and cert["buildTrigger"] == "push"
                and cert["runnerEnvironment"] == "github-hosted"
                and isinstance(invocation, str)
                and re.fullmatch(
                    re.escape(
                        "https://github.com/" + REPO + f"/actions/runs/{run}/attempts/"
                    )
                    + r"[1-9][0-9]*",
                    invocation,
                )
                and isinstance(result["verifiedTimestamps"], list)
                and result["verifiedTimestamps"]
                and statement["predicateType"] == PREDICATE
                and isinstance(statement["subject"], list)
                and any(
                    isinstance(s, dict)
                    and s.get("name") == path.name
                    and s.get("digest", {}).get("sha256") == digest
                    for s in statement["subject"]
                )
            ):
                return
        except (KeyError, TypeError, AttributeError):
            continue
    fail("no attestation matches trusted identity and run")


def verify(directory, source, ref, run, approved):
    if not approved or any(
        not isinstance(s, str) or not HEX40.fullmatch(s) for s in approved
    ):
        fail("invalid approved builder list")
    check_context(source, ref, run, approved[0])
    gh_version()
    files = inventory(directory)
    matches = [
        name
        for name in files
        if re.fullmatch(
            re.escape(f"macos-build-receipt-{run}-") + r"[1-9][0-9]*\.json", name
        )
    ]
    if len(files) != 3 or len(matches) != 1:
        fail("expected two artifacts and one receipt")
    path = files[matches[0]]
    raw = read_bounded(path, MAX_RECEIPT)
    try:
        data = json.loads(raw)
        builder = data["builder_sha"]
    except (ValueError, TypeError, KeyError) as exc:
        raise ProvenanceError("invalid receipt") from exc
    if not isinstance(builder, str) or builder not in approved:
        fail("unapproved builder")
    verify_attestation(path, builder, source, ref, run, hashlib.sha256(raw).hexdigest())
    expected = {
        "schema": 1,
        "repository": REPO,
        "repository_id": REPO_ID,
        "owner_id": OWNER_ID,
        "source_sha": source,
        "source_ref": ref,
        "builder_workflow": WORKFLOW,
        "builder_sha": builder,
        "run_id": run,
    }
    if any(
        type(data.get(k)) is not type(v) or data.get(k) != v
        for k, v in expected.items()
    ):
        fail("receipt context mismatch")
    attempt = data.get("run_attempt")
    if (
        not positive(attempt)
        or path.name != f"macos-build-receipt-{run}-{attempt}.json"
    ):
        fail("receipt attempt mismatch")
    lock = data.get("cargo_lock_sha256")
    if (
        not isinstance(lock, str)
        or not HEX64.fullmatch(lock)
        or lock != lock_digest(source)
    ):
        fail("Cargo.lock digest mismatch")
    records = data.get("targets")
    if not isinstance(records, dict) or set(records) != set(TARGETS):
        fail("invalid target inventory")
    ids, names = set(), {path.name}
    for target, record in records.items():
        if not isinstance(record, dict) or set(record) != {
            "artifact_id",
            "name",
            "sha256",
            "size",
            "cputype",
        }:
            fail("invalid target record")
        id_, digest, name = record["artifact_id"], record["sha256"], record["name"]
        if (
            not positive(id_)
            or id_ in ids
            or not isinstance(name, str)
            or name in names
        ):
            fail("duplicate or invalid artifact")
        ids.add(id_)
        names.add(name)
        if artifact(id_, source, run, target, digest) != name or name not in files:
            fail("artifact metadata mismatch")
        info = binary_info(files[name], target)
        if any(
            type(record[k]) is not type(v) or record[k] != v for k, v in info.items()
        ):
            fail("artifact content mismatch")
        verify_attestation(files[name], builder, source, ref, run, digest)
    if names != set(files):
        fail("unexpected file")
    return data


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="mode", required=True)
    for mode in ("receipt", "verify"):
        p = sub.add_parser(mode)
        p.add_argument("--directory", type=Path, required=True)
        p.add_argument("--source-sha", required=True)
        p.add_argument("--ref", required=True)
        p.add_argument("--run-id", type=int, required=True)
        if mode == "receipt":
            p.add_argument("--run-attempt", type=int, required=True)
            p.add_argument("--builder-sha", required=True)
            for prefix in ("aarch64", "x86"):
                p.add_argument(f"--{prefix}-artifact-id", type=int, required=True)
                p.add_argument(f"--{prefix}-artifact-digest", required=True)
        else:
            p.add_argument("--approved-builder-sha", action="append", required=True)
    args = parser.parse_args()
    try:
        if args.mode == "receipt":
            result = receipt(
                args.directory,
                args.source_sha,
                args.ref,
                args.run_id,
                args.run_attempt,
                args.builder_sha,
                {
                    "aarch64-apple-darwin": (
                        args.aarch64_artifact_id,
                        args.aarch64_artifact_digest,
                    ),
                    "x86_64-apple-darwin": (
                        args.x86_artifact_id,
                        args.x86_artifact_digest,
                    ),
                },
            )
        else:
            result = verify(
                args.directory,
                args.source_sha,
                args.ref,
                args.run_id,
                args.approved_builder_sha,
            )
        print(json.dumps(result, sort_keys=True))
    except ProvenanceError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
