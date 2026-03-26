#!/usr/bin/env python3

from __future__ import annotations

import hashlib
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path


ARCHIVE_RE = re.compile(r"^vera-(?P<target>.+)\.(?P<extension>tar\.gz|zip)$")


def build_manifest(release_dir: Path, tag: str, repo: str) -> dict[str, object]:
    assets: dict[str, object] = {}

    for archive_path in sorted(release_dir.iterdir()):
        match = ARCHIVE_RE.match(archive_path.name)
        if not match:
            continue

        target = match.group("target")
        checksum = hashlib.sha256(archive_path.read_bytes()).hexdigest()
        assets[target] = {
            "archive": archive_path.name,
            "download_url": f"https://github.com/{repo}/releases/download/{tag}/{archive_path.name}",
            "sha256": checksum,
            "size": archive_path.stat().st_size,
        }

    if not assets:
        raise ValueError(f"no Vera archives found in {release_dir}")

    version = tag[1:] if tag.startswith("v") else tag
    return {
        "version": version,
        "tag": tag,
        "repo": repo,
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "assets": assets,
    }


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: generate_release_manifest.py <release-dir> <tag> <owner/repo>",
            file=sys.stderr,
        )
        return 1

    release_dir = Path(sys.argv[1]).resolve()
    manifest = build_manifest(release_dir, sys.argv[2], sys.argv[3])
    output_path = release_dir / "release-manifest.json"
    output_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(output_path)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
