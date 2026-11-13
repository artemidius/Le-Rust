#!/usr/bin/env python3
"""Report whether the pinned Mistral OpenAPI source has changed."""

from __future__ import annotations

import hashlib
import json
import pathlib
import sys
import urllib.error
import urllib.request


ROOT = pathlib.Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "spec" / "source.json"


def main() -> int:
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    request = urllib.request.Request(
        manifest["source"],
        headers={"User-Agent": "le-rust-openapi-check/0.1"},
    )
    try:
        with urllib.request.urlopen(request, timeout=30) as response:
            body = response.read()
    except (urllib.error.URLError, TimeoutError) as error:
        print(f"Could not fetch the Mistral OpenAPI source: {error}", file=sys.stderr)
        return 2
    actual = hashlib.sha256(body).hexdigest()
    expected = manifest["sha256"]
    if actual != expected:
        print("Mistral OpenAPI source changed; review covered operations before updating the pin.")
        print(f"expected: {expected}")
        print(f"actual:   {actual}")
        return 1
    print(f"Mistral OpenAPI source matches pinned digest {expected}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
