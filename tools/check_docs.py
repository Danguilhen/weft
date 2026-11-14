#!/usr/bin/env python3
"""Check relative Markdown links and required design documents.

This is deliberately dependency-free: it runs on a clean checkout with any
Python 3 interpreter. Absolute URLs are not fetched; the check is offline.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent

REQUIRED = [
    "README.md",
    "AGENTS.md",
    "LICENSING.md",
    "LICENSE",
    "LICENSE-MIT",
    "CONTRIBUTING.md",
    "SECURITY.md",
    "docs/STATUS.md",
    "docs/PLAN.md",
    "docs/README.md",
    "docs/ARCHITECTURE.md",
    "docs/PROTOCOL.md",
    "docs/FEDERATION.md",
    "docs/API.md",
    "docs/DATA_MODEL.md",
    "docs/CONFIG.md",
    "docs/MVP.md",
    "docs/THREAT_MODEL.md",
    "docs/PRIVACY.md",
    "docs/OPERATIONS.md",
    "docs/CONTENT_LICENSING.md",
    "docs/GOVERNANCE.md",
    "docs/XMPP.md",
    "docs/OPEN_QUESTIONS.md",
    "docs/DESIGN_REVIEW.md",
    "docs/openapi.yaml",
    "docs/adr/README.md",
]

LINK_RE = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE_RE = re.compile(r"```.*?```", re.DOTALL)
INLINE_CODE_RE = re.compile(r"`[^`]*`")


def markdown_files() -> list[Path]:
    skip = {".git", "target", "node_modules", "graphify-out", ".graphify"}
    files = []
    for path in sorted(ROOT.rglob("*.md")):
        relative = path.relative_to(ROOT)
        if any(part in skip for part in relative.parts):
            continue
        files.append(path)
    return files


def check_required() -> list[str]:
    missing = [path for path in REQUIRED if not (ROOT / path).is_file()]
    return [f"missing required file: {path}" for path in missing]


def check_links(path: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    text = FENCE_RE.sub("", text)
    text = INLINE_CODE_RE.sub("", text)

    for match in LINK_RE.finditer(text):
        target = match.group(1).strip()
        if not target or target.startswith(("#", "http://", "https://", "mailto:", "tel:")):
            continue
        target = target.split("#", 1)[0]
        if not target:
            continue
        target = unquote(target)
        resolved = (path.parent / target).resolve()
        if not resolved.exists():
            line = text[: match.start()].count("\n") + 1
            errors.append(f"{path.relative_to(ROOT)}:{line}: broken link -> {target}")
    return errors


def main() -> int:
    errors = check_required()
    for path in markdown_files():
        errors.extend(check_links(path))

    if errors:
        print("Documentation check failed:\n")
        for error in errors:
            print(f"  {error}")
        print(f"\n{len(errors)} problem(s).")
        return 1

    print(f"Documentation check passed ({len(markdown_files())} Markdown files).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
