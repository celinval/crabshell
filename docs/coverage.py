#!/usr/bin/env python3
"""Regenerates `docs/src/build-scripts/coverage.md` from the fixtures.

Usage: python3 -I docs/coverage.py <report.html>

The report is the Crates.io Build Script Report page (see
docs/src/build-scripts/labels.md); its per-crate labels give each crate its
categories. Everything else comes from the fixtures' `requests.toml`.
"""

import collections
import json
import re
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    import pip._vendor.tomli as tomllib

ROOT = Path(__file__).resolve().parent.parent
TABLE = ROOT / "docs/src/build-scripts/coverage.md"
# A named grant implies the category whose fixture defines or needs it.
GRANT_CATEGORY = {
    "rustc": "rustc-probe",
    "cc": "cc-toolchain",
    "cxx": "cc-toolchain",
    "clang": "cc-toolchain",
    "cmake": "cc-toolchain",
    "autotools": "cc-toolchain",
    "pkg-config": "system-lib",
    "pkg-config-path": "system-lib",
    "protoc": "protoc",
    "bindgen": "bindgen",
    "git": "vcs-metadata",
    "git-repo": "vcs-metadata",
    "python": "lang-runtime",
    "java": "lang-runtime",
}
ORDER = [
    "cfg-only",
    "codegen",
    "rustc-probe",
    "vcs-metadata",
    "protoc",
    "system-lib",
    "bindgen",
    "cc-toolchain",
    "lang-runtime",
    "network-fetch",
]


def report_labels(path):
    """Each build-script crate's labels, from the report's embedded data."""
    text = Path(path).read_text(encoding="utf-8")
    start = text.index("const DATA = ") + len("const DATA = ")
    data, _ = json.JSONDecoder().raw_decode(text[start:])
    return {d["name"]: d["labels"] for d in data if d["uses_build_script"]}


def label_categories():
    """Each report label's category, from the labels chapter's table."""
    rows = (ROOT / "docs/src/build-scripts/labels.md").read_text()
    pairs = re.findall(r"\| `([a-z_]+)` \| \d+ \| `([a-z-]+)`", rows)
    return dict(pairs)


def requests():
    """What every fixture's crates request, keyed by crate name."""
    out = collections.defaultdict(
        lambda: {"fixtures": set(), "env": set(), "read": set(), "write": set(),
                 "run": set(), "network": False, "grants": set()}
    )
    for path in sorted(ROOT.glob("crabshell/fixtures/*/requests.toml")):
        parsed = tomllib.loads(path.read_text())
        for name, declared in parsed.get("package", {}).items():
            crate = out[name]
            crate["fixtures"].add(path.parent.name)
            for field in ("env", "read", "write", "run"):
                crate[field] |= set(declared.get(field, []))
            crate["grants"] |= set(declared.get("requests", []))
            crate["network"] |= bool(declared.get("network", False))
    return out


def categories(name, crate, labels, by_label):
    found = {by_label[l] for l in labels.get(name, []) if l in by_label}
    found |= {GRANT_CATEGORY[g] for g in crate["grants"] if g in GRANT_CATEGORY}
    if crate["network"]:
        found.add("network-fetch")
    return ", ".join(sorted(found, key=ORDER.index)) or "–"


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    labels, by_label, crates = report_labels(sys.argv[1]), label_categories(), requests()
    code = lambda values: ", ".join(f"`{v}`" for v in sorted(values)) or "–"
    rows = [
        "| Crate | Categories | Fixtures | Env | Read | Write | Run | Network | Grants |",
        "|---|---|---|---:|---|---|---|---|---|",
    ]
    for name in sorted(crates, key=str.lower):
        crate = crates[name]
        rows.append(
            f"| {name} | {categories(name, crate, labels, by_label)} "
            f"| {', '.join(sorted(crate['fixtures']))} | {len(crate['env']) or '–'} "
            f"| {code(crate['read'])} | {code(crate['write'])} | {code(crate['run'])} "
            f"| {'yes' if crate['network'] else '–'} | {code(crate['grants'])} |"
        )
    counted = lambda field: sum(1 for c in crates.values() if c[field])
    grants = collections.Counter(g for c in crates.values() for g in c["grants"])
    summary = (
        f"{len(crates)} crates, {sum(1 for n in crates if n in labels)} of them among the "
        f"report's {len(labels)} with build scripts. {counted('env')}\nrequest env, "
        f"{counted('read')} reads, {counted('write')} writes, {counted('run')} programs "
        f"directly and\n{sum(c['network'] for c in crates.values())} network. "
        f"Grants requested:\n"
        + ", ".join(f"`{g}` ({n})" for g, n in grants.most_common())
        + ".\n"
    )
    head = TABLE.read_text()
    head = head[: head.index("| Crate | Categories")]
    head = re.sub(
        r"\d+ crates, \d+ of them among.*?Grants requested:\n.*?\n", summary, head, flags=re.S
    )
    TABLE.write_text(head + "\n".join(rows) + "\n")
    print(f"{len(crates)} crates")


if __name__ == "__main__":
    main()
