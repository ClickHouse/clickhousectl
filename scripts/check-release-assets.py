#!/usr/bin/env python3
"""Verify authenticated macOS release evidence and packaged release archives."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import sys
import tarfile

ARCHES = ("arm64", "x86_64")
TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin",
           "aarch64-unknown-linux-musl", "x86_64-unknown-linux-musl")
MAX_JSON = 64 * 1024
MAX_BINARY = 512 * 1024 * 1024


class Rejected(Exception):
    pass


def require(condition, reason):
    if not condition:
        raise Rejected(reason)


def unique_keys(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def read_file(path, limit):
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(fd, "rb") as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= limit, "invalid file")
        data = stream.read(limit + 1)
        require(len(data) == info.st_size, "file size changed")
        return data


def json_file(path):
    return json.loads(read_file(path, MAX_JSON), object_pairs_hook=unique_keys)


def keys(value, names):
    return isinstance(value, dict) and value.keys() == set(names.split())


def hex_value(value, length):
    return isinstance(value, str) and re.fullmatch(rf"[a-f0-9]{{{length}}}", value) is not None


def version_for_ref(ref):
    match = re.fullmatch(r"refs/tags/v([0-9]+\.[0-9]+\.[0-9]+(?:[-+][A-Za-z0-9.+-]+)?)", ref)
    require(match is not None, "invalid release ref")
    return match.group(1)


def manifest_file(path, ref, expected_sha256=None):
    raw = read_file(path, MAX_JSON)
    if expected_sha256 is not None:
        require(hashlib.sha256(raw).hexdigest() == expected_sha256, "manifest digest mismatch")
    manifest = json.loads(raw, object_pairs_hook=unique_keys)
    require(keys(manifest, "schema version source signer binaries")
            and type(manifest["schema"]) is int and manifest["schema"] == 1
            and manifest["version"] == version_for_ref(ref), "invalid manifest")
    for name in ("source", "signer"):
        identity = manifest[name]
        require(keys(identity, "repository_id sha run_id run_attempt")
                and hex_value(identity["sha"], 40)
                and all(type(identity[k]) is int and identity[k] > 0
                        for k in ("repository_id", "run_id", "run_attempt")), "invalid identity")
    require(keys(manifest["binaries"], "arm64 x86_64")
            and all(hex_value(value, 64) for value in manifest["binaries"].values()), "invalid binary digests")
    return manifest


def positive(value):
    require(isinstance(value, str) and re.fullmatch(r"[1-9][0-9]*", value) is not None, "invalid run number")
    return int(value)


def verify_macos(args):
    require(hex_value(args.manifest_sha256, 64) and hex_value(args.leaf_sha256, 64)
            and hex_value(args.source_sha, 40) and hex_value(args.signer_sha, 40), "invalid pin")
    expected_files = {"manifest.json", "notarization.json", "arm64", "x86_64",
                      "arm64-receipt.json", "x86_64-receipt.json"}
    require({p.name for p in args.directory.iterdir()} == expected_files, "invalid bundle inventory")
    manifest_path = args.directory / "manifest.json"
    manifest = manifest_file(manifest_path, args.ref, args.manifest_sha256)
    source, signer = manifest["source"], manifest["signer"]
    require(source["repository_id"] == 1150809931 and source["sha"] == args.source_sha
            and source["run_id"] == positive(args.run_id)
            and source["run_attempt"] == positive(args.receipt_run_attempt), "source identity mismatch")
    require(signer["repository_id"] == 1464494971 and signer["sha"] == args.signer_sha,
            "signer identity mismatch")
    notarization = json_file(args.directory / "notarization.json")
    require(keys(notarization, "schema submission_id status manifest_sha256")
            and type(notarization["schema"]) is int and notarization["schema"] == 1
            and isinstance(notarization["submission_id"], str)
            and re.fullmatch(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", notarization["submission_id"])
            and notarization["status"] == "Accepted" and notarization["manifest_sha256"] == args.manifest_sha256,
            "invalid notarization evidence")
    for arch in ARCHES:
        digest = manifest["binaries"][arch]
        fd = os.open(args.directory / arch, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            info = os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and 0 < info.st_size <= MAX_BINARY, "invalid binary")
            binary_digest = hashlib.sha256()
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                binary_digest.update(chunk)
            require(binary_digest.hexdigest() == digest, "binary digest mismatch")
        receipt = json_file(args.directory / f"{arch}-receipt.json")
        require(keys(receipt, "schema arch macos manifest_sha256 source signer binary_sha256 leaf_sha256 gatekeeper version")
                and type(receipt["schema"]) is int and receipt["schema"] == 1
                and receipt["arch"] == arch and isinstance(receipt["macos"], str)
                and re.fullmatch(r"[0-9]+(?:\.[0-9]+){1,2}", receipt["macos"])
                and receipt["manifest_sha256"] == args.manifest_sha256
                and json.dumps(receipt["source"], sort_keys=True) == json.dumps(source, sort_keys=True)
                and json.dumps(receipt["signer"], sort_keys=True) == json.dumps(signer, sort_keys=True)
                and receipt["binary_sha256"] == digest and receipt["leaf_sha256"] == args.leaf_sha256
                and receipt["gatekeeper"] == "Notarized Developer ID"
                and receipt["version"] == manifest["version"], "invalid native receipt")


def verify_archives(args):
    manifest = manifest_file(args.manifest, args.ref)
    tag = args.ref.removeprefix("refs/tags/")
    names = {f"clickhousectl-{target}-{tag}.tar.gz" for target in TARGETS}
    present = {p.name for p in args.directory.iterdir()}
    require(present == names or present == names | {"SHA256SUMS"}, "invalid archive inventory")
    sums = {}
    for target in TARGETS:
        root = f"clickhousectl-{target}-{tag}"
        path = args.directory / f"{root}.tar.gz"
        # Open without following links, and reject special files before parsing.
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            info = os.fstat(stream.fileno())
            require(stat.S_ISREG(info.st_mode) and info.st_size > 0, "invalid archive file")
            digest = hashlib.sha256()
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
            sums[path.name] = digest.hexdigest()
            stream.seek(0)
            with tarfile.open(fileobj=stream, mode="r:gz") as archive:
                found = set()
                for member in archive:
                    name = member.name.rstrip("/") if member.isdir() else member.name
                    require(name not in found, "duplicate archive member")
                    found.add(name)
                    if name == root:
                        require(member.isdir(), "invalid archive root")
                    elif name in (f"{root}/clickhousectl", f"{root}/LICENSE"):
                        require(member.isfile() and 0 < member.size <= MAX_BINARY, "invalid archive member")
                        contents = archive.extractfile(member)
                        require(contents is not None, "unreadable archive member")
                        member_digest = hashlib.sha256()
                        for chunk in iter(lambda: contents.read(1024 * 1024), b""):
                            member_digest.update(chunk)
                        if name.endswith("/clickhousectl") and target.endswith("apple-darwin"):
                            arch = "arm64" if target.startswith("aarch64") else "x86_64"
                            require(member_digest.hexdigest() == manifest["binaries"][arch], "archive binary mismatch")
                    else:
                        raise Rejected("unexpected archive member")
                require(found == {root, f"{root}/clickhousectl", f"{root}/LICENSE"}, "missing archive member")
    if "SHA256SUMS" in present:
        raw = read_file(args.directory / "SHA256SUMS", MAX_JSON).decode("ascii")
        lines = raw.splitlines()
        require(len(lines) == len(sums), "invalid checksums")
        parsed = {}
        for line in lines:
            match = re.fullmatch(r"([a-f0-9]{64})  ([^/]+)", line)
            require(match is not None and match.group(2) not in parsed, "invalid checksums")
            parsed[match.group(2)] = match.group(1)
        require(parsed == sums, "checksum mismatch")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    macos = sub.add_parser("verify-macos")
    macos.add_argument("--directory", type=Path, required=True)
    for name in ("manifest-sha256", "source-sha", "ref", "run-id", "receipt-run-attempt", "leaf-sha256", "signer-sha"):
        macos.add_argument("--" + name, required=True)
    archives = sub.add_parser("verify-archives")
    archives.add_argument("--directory", type=Path, required=True)
    archives.add_argument("--ref", required=True)
    archives.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    try:
        (verify_macos if args.command == "verify-macos" else verify_archives)(args)
    except (Rejected, OSError, ValueError, RecursionError, tarfile.TarError) as error:
        print(f"release assets rejected: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
