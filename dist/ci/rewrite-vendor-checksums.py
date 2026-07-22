#!/usr/bin/env python3
"""Rewrite vendor/*/.cargo-checksum.json after pruning files from a cargo vendor tree."""
from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path


def rewrite(vendor: Path) -> int:
    updated = 0
    for ck in sorted(vendor.glob("*/.cargo-checksum.json")):
        root = ck.parent
        data = json.loads(ck.read_text(encoding="utf-8"))
        files: dict[str, str] = {}
        for path in root.rglob("*"):
            if not path.is_file() or path.name == ".cargo-checksum.json":
                continue
            rel = path.relative_to(root).as_posix()
            files[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
        data["files"] = files
        ck.write_text(json.dumps(data, sort_keys=True, indent=4) + "\n", encoding="utf-8")
        updated += 1
    return updated


def main() -> int:
    vendor = Path(sys.argv[1] if len(sys.argv) > 1 else "vendor")
    if not vendor.is_dir():
        print(f"error: {vendor} is not a directory", file=sys.stderr)
        return 1
    n = rewrite(vendor)
    print(f"rewrote checksums for {n} vendored crates under {vendor}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
