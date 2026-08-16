#!/usr/bin/env python3
"""Flip docs/MAPPING.md DEFERRED->DONE for functions that have a real Rust impl.

A function is considered implemented when `src/` contains a non-stub
`pub async fn <name>` (the akshare-equivalent entrypoint). Pure stubs that
only `return Err(Error::UpstreamChanged {...})` are excluded so genuinely
deferred fns (THS v cookie, dead 404, week-permission, xq token) stay put.
"""
import os
import re

ROOT = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(ROOT, "src")
MAPPING = os.path.join(ROOT, "docs", "MAPPING.md")

# 1. Collect pub async fn -> first source file (relative).
impls = {}
src_files = []
for dirpath, _, files in os.walk(SRC):
    for fn in files:
        if fn.endswith(".rs"):
            src_files.append(os.path.join(dirpath, fn))

for path in src_files:
    rel = os.path.relpath(path, ROOT)
    with open(path, encoding="utf-8") as fh:
        content = fh.read()
    for m in re.finditer(r"pub\s+async\s+fn\s+(\w+)", content):
        name = m.group(1)
        if name.startswith("parse_"):
            continue
        impls.setdefault(name, rel)

# 2. Detect stub fns: body essentially only `Err(Error::UpstreamChanged {...})`
#    with no network/parse work.
stubs = set()
for path in src_files:
    with open(path, encoding="utf-8") as fh:
        content = fh.read()
    for m in re.finditer(
        r"pub\s+async\s+fn\s+(\w+)\s*\([^)]*\)\s*(?:->\s*[^\{]*)?\{", content
    ):
        name = m.group(1)
        start = m.end()
        depth = 1
        i = start
        body = ""
        while i < len(content) and depth > 0:
            c = content[i]
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
            if depth > 0:
                body += c
            i += 1
        body = body.strip()
        does_work = re.search(
            r"get_text|get_json|open_workbook|extract_|read_to_string|fetch_bytes|client\.|reqwest|scraper|calamine",
            body,
        )
        if re.search(r"Error::UpstreamChanged", body) and not does_work:
            stubs.add(name)

# 3. Flip MAPPING rows.
flipped = []
with open(MAPPING, encoding="utf-8") as fh:
    lines = fh.readlines()

out = []
for line in lines:
    if line.strip().startswith("|") and "DEFERRED" in line:
        # columns: | `name` | path | source | status | reason |
        parts = [p.strip() for p in line.split("|")]
        # parts[0] == '' ; parts[1] = `name`, parts[2] = path, parts[3] = source, parts[4] = status, parts[5] = reason, parts[6] == ''
        if len(parts) < 6:
            out.append(line)
            continue
        name = parts[1].strip("`").strip()
        if name in impls and name not in stubs:
            rel = impls[name]
            new_path = f"`{rel}::{name}`"
            # rebuild row: keep name, set path, keep source, status=DONE, reason empty
            new_line = f"| `{name}` | {new_path} | {parts[3]} | DONE |  |\n"
            flipped.append((name, rel))
            out.append(new_line)
            continue
    out.append(line)

with open(MAPPING, "w", encoding="utf-8") as fh:
    fh.writelines(out)

print(f"impls found: {len(impls)}")
print(f"stubs excluded: {sorted(stubs)}")
print(f"rows flipped DEFERRED->DONE: {len(flipped)}")
for n, r in flipped:
    print(f"  DONE {n} -> {r}::{n}")
