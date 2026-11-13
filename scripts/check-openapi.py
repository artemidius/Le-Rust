#!/usr/bin/env python3
"""Classify Mistral OpenAPI drift against Le Rust's reviewed surface."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import pathlib
import sys
import tomllib
import urllib.error
import urllib.request
from typing import Any

import yaml


ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = ROOT / "spec" / "source.json"
DEFAULT_COVERAGE = ROOT / "spec" / "coverage.toml"
DEFAULT_SNAPSHOT = ROOT / "spec" / "covered-openapi.json"
IGNORED_DOCUMENTATION_KEYS = {
    "description",
    "example",
    "examples",
    "externalDocs",
    "summary",
    "title",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--coverage", type=pathlib.Path, default=DEFAULT_COVERAGE)
    parser.add_argument("--snapshot", type=pathlib.Path, default=DEFAULT_SNAPSHOT)
    parser.add_argument("--candidate", type=pathlib.Path)
    parser.add_argument("--report", type=pathlib.Path)
    parser.add_argument(
        "--write-snapshot",
        action="store_true",
        help="review and replace the covered-surface snapshot",
    )
    return parser.parse_args()


def canonical(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: canonical(item)
            for key, item in sorted(value.items())
            if key not in IGNORED_DOCUMENTATION_KEYS
        }
    if isinstance(value, list):
        return [canonical(item) for item in value]
    return value


def digest_json(value: Any) -> str:
    encoded = json.dumps(
        canonical(value), sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def resolve_pointer(document: dict[str, Any], pointer: str) -> Any:
    if not pointer.startswith("#/"):
        raise ValueError(f"external OpenAPI reference is not supported: {pointer}")
    value: Any = document
    for raw_part in pointer[2:].split("/"):
        part = raw_part.replace("~1", "/").replace("~0", "~")
        value = value[part]
    return value


def find_references(value: Any) -> set[str]:
    references: set[str] = set()
    if isinstance(value, dict):
        reference = value.get("$ref")
        if isinstance(reference, str):
            references.add(reference)
        for item in value.values():
            references.update(find_references(item))
    elif isinstance(value, list):
        for item in value:
            references.update(find_references(item))
    return references


def operation_contract(document: dict[str, Any], method: str, path: str) -> dict[str, Any]:
    path_item = document.get("paths", {}).get(path)
    if not isinstance(path_item, dict):
        raise ValueError(f"covered path is absent from OpenAPI: {path}")
    operation = path_item.get(method.lower())
    if not isinstance(operation, dict):
        raise ValueError(f"covered operation is absent from OpenAPI: {method} {path}")

    contract = {
        "parameters": [
            *copy.deepcopy(path_item.get("parameters", [])),
            *copy.deepcopy(operation.get("parameters", [])),
        ],
        "requestBody": copy.deepcopy(operation.get("requestBody")),
        "responses": copy.deepcopy(operation.get("responses", {})),
        "security": copy.deepcopy(operation.get("security")),
    }
    references = find_references(contract)
    resolved: dict[str, Any] = {}
    pending = list(references)
    while pending:
        reference = pending.pop()
        if reference in resolved:
            continue
        value = copy.deepcopy(resolve_pointer(document, reference))
        resolved[reference] = value
        pending.extend(find_references(value) - resolved.keys())
    return {"operation": contract, "references": resolved}


def load_operations(coverage_path: pathlib.Path) -> list[dict[str, Any]]:
    coverage = tomllib.loads(coverage_path.read_text(encoding="utf-8"))
    grouped: dict[tuple[str, str], set[str]] = {}
    for operation in coverage.get("operation", []):
        key = (operation["method"].upper(), operation["path"])
        grouped.setdefault(key, set()).add(operation["public_method"])
    return [
        {"method": method, "path": path, "public_methods": sorted(public_methods)}
        for (method, path), public_methods in sorted(grouped.items())
    ]


def build_snapshot(
    document: dict[str, Any], source_sha256: str, operations: list[dict[str, Any]]
) -> dict[str, Any]:
    reviewed_operations = []
    for operation in operations:
        surface = operation_contract(document, operation["method"], operation["path"])
        references = {
            reference: digest_json(value)
            for reference, value in sorted(surface["references"].items())
        }
        reviewed_operations.append(
            {
                **operation,
                "surface_sha256": digest_json(surface),
                "references": references,
            }
        )
    return {
        "schema_version": 1,
        "source_sha256": source_sha256,
        "operations": reviewed_operations,
    }


def load_candidate(args: argparse.Namespace, source: str) -> bytes:
    if args.candidate:
        return args.candidate.read_bytes()
    request = urllib.request.Request(
        source,
        headers={"User-Agent": "le-rust-openapi-check/0.1"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            return response.read()
    except (urllib.error.URLError, TimeoutError) as error:
        raise RuntimeError(f"could not fetch the Mistral OpenAPI source: {error}") from error


def compare_snapshots(
    reviewed: dict[str, Any], candidate: dict[str, Any]
) -> list[dict[str, Any]]:
    reviewed_by_key = {
        (operation["method"], operation["path"]): operation
        for operation in reviewed["operations"]
    }
    affected = []
    for operation in candidate["operations"]:
        key = (operation["method"], operation["path"])
        previous = reviewed_by_key.get(key)
        if previous is None:
            affected.append({**operation, "change": "newly covered operation"})
            continue
        if previous["surface_sha256"] == operation["surface_sha256"]:
            continue
        old_refs = previous.get("references", {})
        new_refs = operation.get("references", {})
        changed_refs = sorted(
            reference
            for reference in old_refs.keys() | new_refs.keys()
            if old_refs.get(reference) != new_refs.get(reference)
        )
        affected.append(
            {
                "method": operation["method"],
                "path": operation["path"],
                "public_methods": operation["public_methods"],
                "change": "covered contract changed",
                "changed_references": changed_refs,
            }
        )
    return affected


def write_report(path: pathlib.Path | None, report: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    try:
        manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
        body = load_candidate(args, manifest["source"])
        actual_sha256 = hashlib.sha256(body).hexdigest()
        document = yaml.safe_load(body)
        if not isinstance(document, dict):
            raise ValueError("OpenAPI candidate is not an object")
        operations = load_operations(args.coverage)
        candidate_snapshot = build_snapshot(document, actual_sha256, operations)

        if args.write_snapshot:
            args.snapshot.write_text(
                json.dumps(candidate_snapshot, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
            report = {
                "status": "snapshot_written",
                "actual_sha256": actual_sha256,
                "affected_operations": [],
            }
            write_report(args.report, report)
            print(f"Wrote reviewed covered-surface snapshot for {actual_sha256}.")
            return 0

        reviewed = json.loads(args.snapshot.read_text(encoding="utf-8"))
        expected_sha256 = manifest["sha256"]
        if reviewed.get("source_sha256") != expected_sha256:
            raise ValueError("covered-surface snapshot does not match the pinned source digest")
        affected = compare_snapshots(reviewed, candidate_snapshot)
        if actual_sha256 == expected_sha256 and affected:
            raise ValueError("candidate digest matches the pin but its covered surface does not")

        status = (
            "covered_change"
            if affected
            else "exact_match"
            if actual_sha256 == expected_sha256
            else "unrelated_change"
        )
        report = {
            "status": status,
            "expected_sha256": expected_sha256,
            "actual_sha256": actual_sha256,
            "affected_operations": affected,
        }
        write_report(args.report, report)

        if status == "exact_match":
            print(f"Mistral OpenAPI source matches pinned digest {expected_sha256}.")
            return 0
        print("Mistral OpenAPI source changed.")
        print(f"expected: {expected_sha256}")
        print(f"actual:   {actual_sha256}")
        if status == "unrelated_change":
            print("No covered Le Rust operation or referenced schema changed.")
            return 0
        print("Covered surfaces requiring review:")
        for operation in affected:
            methods = ", ".join(operation["public_methods"])
            print(f"- {operation['method']} {operation['path']} ({methods})")
            for reference in operation.get("changed_references", []):
                print(f"  - {reference}")
        return 1
    except (OSError, KeyError, TypeError, ValueError, RuntimeError, yaml.YAMLError) as error:
        report = {"status": "error", "message": str(error), "affected_operations": []}
        write_report(args.report, report)
        print(f"OpenAPI drift check failed: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
