#!/usr/bin/env python3
"""
batch_runner.py  N  [--prev-last CODE]
Full automated pipeline for adding one batch of EPSG codes.
Usage: python3 batch_runner.py 22
"""

import re
import sys
import json
import subprocess
import os

WBPROJ_DIR  = "/Users/johnlindsay/Documents/programming/Rust/whitebox_backend/wbprojection"
PYTHON_BIN  = "/Users/johnlindsay/Documents/programming/.venv/bin/python"
TARGET_COUNT = 100

BAD_METHOD_TOKENS = {
    # Added after first-pass corpus gate failures:
}

def run(cmd, **kw):
    kw.setdefault("cwd", WBPROJ_DIR)
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True, **kw)
    return r.stdout.strip(), r.stderr.strip(), r.returncode

def projinfo_wkt(code):
    r = subprocess.run(
        ["projinfo", f"EPSG:{code}", "-o", "WKT1:ESRI", "--single-line", "-q"],
        capture_output=True, text=True, timeout=10, cwd=WBPROJ_DIR
    )
    return r.stdout.strip()

def projinfo_json(code):
    r = subprocess.run(
        ["projinfo", f"EPSG:{code}", "-o", "PROJJSON", "-q", "--single-line"],
        capture_output=True, text=True, timeout=10, cwd=WBPROJ_DIR
    )
    try:
        return json.loads(r.stdout)
    except Exception:
        return {}

# ─── Step 0: parse args ──────────────────────────────────────────────────────

if len(sys.argv) < 2:
    print("Usage: python3 batch_runner.py N"); sys.exit(1)

N     = int(sys.argv[1])
PREV  = N - 1
SRC   = os.path.join(WBPROJ_DIR, "src")
EPSG_RS   = os.path.join(SRC, "epsg.rs")
INFO_RS   = os.path.join(SRC, "epsg_generated_info.rs")
TESTS_RS  = os.path.join(SRC, "tests", "epsg_tests.rs")
README_MD = os.path.join(WBPROJ_DIR, "README.md")
WKT_OUT   = os.path.join(SRC, f"epsg_generated_batch{N}_wkt.rs")
CODES_TMP = f"/tmp/wbprojection_batch{N}_codes.txt"

print(f"\n{'='*60}")
print(f"  BATCH {N} PIPELINE")
print(f"{'='*60}")

# ─── Step 1: refresh known codes ─────────────────────────────────────────────

print("\n[1] Refreshing known codes via dump example…")
os.makedirs(os.path.join(WBPROJ_DIR, "examples"), exist_ok=True)
dump_src = os.path.join(WBPROJ_DIR, "examples", "wbprojection_dump_known_codes.rs")
with open(dump_src, "w") as f:
    f.write("use wbprojection::epsg::known_epsg_codes;\nfn main() { for code in known_epsg_codes() { println!(\"{code}\"); } }\n")

out, err, rc = run(f"cargo run --quiet --example wbprojection_dump_known_codes > /tmp/wbprojection_known_codes.txt 2>&1")
if rc != 0:
    print(f"  FAIL: cargo run: {err}"); sys.exit(1)

# also get proj list (reuse if exists)
if not os.path.exists("/tmp/proj_projected_epsg_codes.txt"):
    run("projinfo --list-crs projected | awk '/^EPSG:/{print $1}' | sed 's/^EPSG://' > /tmp/proj_projected_epsg_codes.txt")

known = set(int(l) for l in open("/tmp/wbprojection_known_codes.txt") if l.strip())
proj  = [int(l) for l in open("/tmp/proj_projected_epsg_codes.txt") if l.strip()]
candidates = sorted(c for c in proj if c not in known)
print(f"  Known: {len(known)}, candidates available: {len(candidates)}")

# clean up dump example
os.remove(dump_src)

# ─── Step 2: generate WKT file ───────────────────────────────────────────────

print(f"\n[2] Generating WKT for batch {N}…")
accepted = []

for code in candidates:
    if len(accepted) >= TARGET_COUNT:
        break
    try:
        wkt = projinfo_wkt(code)
    except Exception:
        continue
    if not wkt or not wkt.startswith("PROJCS["):
        continue
    # Keep projected CRS definitions even if they carry trailing vertical
    # metadata blocks; we only exclude explicit compound CRS wrappers.
    if "COMPD_CS[" in wkt:
        continue
    if 'PROJECTION["Local"]' in wkt:
        continue
    norm = re.sub(r"[^a-z]+", "", wkt.lower())
    if any(t in norm for t in BAD_METHOD_TOKENS):
        continue
    safe_wkt = wkt.replace('"', '\\"')
    accepted.append((code, safe_wkt))

if len(accepted) < TARGET_COUNT:
    print(f"  WARNING: only {len(accepted)} codes accepted (target {TARGET_COUNT})")
if not accepted:
    print("  ERROR: no codes generated"); sys.exit(1)

first_code = accepted[0][0]
last_code  = accepted[-1][0]
print(f"  Accepted {len(accepted)}: first={first_code}, last={last_code}")

with open(WKT_OUT, "w") as f:
    f.write("// AUTO-GENERATED — do not edit by hand.\n")
    f.write(f"// Source: PROJ projinfo ESRI WKT1 output for selected projected EPSG codes.\n")
    f.write(f"fn generated_batch{N}_wkt(code: u32) -> Option<&'static str> {{\n")
    f.write("    match code {\n")
    for code, wkt in accepted:
        f.write(f'        {code} => Some("{wkt}"),\n')
    f.write("        _ => None,\n")
    f.write("    }\n")
    f.write("}\n")

with open(CODES_TMP, "w") as f:
    for code, _ in accepted: f.write(f"{code}\n")

# Build Rust constant
codes_only = [str(c) for c, _ in accepted]
rows = []
row = []
for c in codes_only:
    row.append(c)
    if len(row) == 8:
        rows.append("    " + ", ".join(row) + ",")
        row = []
if row:
    rows.append("    " + ", ".join(row) + ",")
codes_const = f"const GENERATED_BATCH{N}_CODES: &[u32] = &[\n" + "\n".join(rows) + "\n];"
print(f"  Wrote {WKT_OUT}")

# ─── Step 3: patch epsg.rs ────────────────────────────────────────────────────

print(f"\n[3] Patching epsg.rs…")
with open(EPSG_RS) as f:
    content = f.read()

# include line
old_include = f'include!("epsg_generated_batch{PREV}_wkt.rs");'
if old_include not in content:
    print(f"  ERROR: could not find '{old_include}'"); sys.exit(1)

new_include = old_include + f'\ninclude!("epsg_generated_batch{N}_wkt.rs");'
content = content.replace(old_include, new_include, 1)

# constant — insert before "/// EPSG code resolution behavior"
resolution_marker = "/// EPSG code resolution behavior"
content = content.replace(resolution_marker, codes_const + "\n\n" + resolution_marker, 1)

# candidate filter
old_filter = f"|| generated_batch{PREV}_wkt(code).is_some()"
new_filter = f"|| generated_batch{PREV}_wkt(code).is_some()\n                || generated_batch{N}_wkt(code).is_some()"
content = content.replace(old_filter, new_filter, 1)

# known_codes extend
old_extend = f"codes.extend_from_slice(GENERATED_BATCH{PREV}_CODES);\n    codes.sort_unstable();"
new_extend = f"codes.extend_from_slice(GENERATED_BATCH{PREV}_CODES);\n    codes.extend_from_slice(GENERATED_BATCH{N}_CODES);\n    codes.sort_unstable();"
content = content.replace(old_extend, new_extend, 1)

# build_crs fallback — insert after prev block
old_build = f"""    if let Some(raw_wkt) = generated_batch{PREV}_wkt(code) {{
        let wkt = raw_wkt
            .replace("\\\\\\\"", "\\"")
            .replace("Gauss_Kruger", "Transverse_Mercator")
            .replace("Double_Stereographic", "Stereographic");
        return crate::wkt::parse_crs_from_wkt(&wkt);
    }}"""
new_build = old_build + f"""
    if let Some(raw_wkt) = generated_batch{N}_wkt(code) {{
        let wkt = raw_wkt
            .replace("\\\\\\\"", "\\"")
            .replace("Gauss_Kruger", "Transverse_Mercator")
            .replace("Double_Stereographic", "Stereographic");
        return crate::wkt::parse_crs_from_wkt(&wkt);
    }}"""

if old_build not in content:
    print(f"  ERROR: could not find batch{PREV} build_crs block"); sys.exit(1)
content = content.replace(old_build, new_build, 1)

with open(EPSG_RS, "w") as f:
    f.write(content)
print("  epsg.rs updated")

# ─── Step 4: patch tests ─────────────────────────────────────────────────────

print(f"\n[4] Patching epsg_tests.rs…")
with open(TESTS_RS) as f:
    tc = f.read()

# build-ok test — insert after prev
old_build_test = f"""#[test]
fn epsg_generated_batch{PREV}_codes_build_ok() {{
    assert!(Crs::from_epsg({first_code - 1 if PREV >= 20 else '__'}).is_ok());"""

# Use a simpler approach: just find the prev build_ok test and append after its closing brace
pattern = re.compile(
    r'(#\[test\]\nfn epsg_generated_batch' + str(PREV) + r'_codes_build_ok\(\) \{[^}]+\}\n)',
    re.DOTALL
)
m = pattern.search(tc)
if not m:
    print(f"  ERROR: could not find batch{PREV} build_ok test")
    sys.exit(1)

new_build_test = m.group(1) + f"""
#[test]
fn epsg_generated_batch{N}_codes_build_ok() {{
    assert!(Crs::from_epsg({first_code}).is_ok());
    assert!(Crs::from_epsg({last_code}).is_ok());
}}
"""
tc = tc[:m.start()] + new_build_test + tc[m.end():]

# tail-code test — insert after prev
pattern2 = re.compile(
    r'(#\[test\]\nfn known_epsg_codes_includes_generated_batch' + str(PREV) + r'_tail_code\(\) \{[^}]+\}\n)',
    re.DOTALL
)
m2 = pattern2.search(tc)
if not m2:
    print(f"  ERROR: could not find batch{PREV} tail_code test")
    sys.exit(1)
new_tail_test = m2.group(1) + f"""
#[test]
fn known_epsg_codes_includes_generated_batch{N}_tail_code() {{
    assert!(known_epsg_codes().contains(&{last_code}));
}}
"""
tc = tc[:m2.start()] + new_tail_test + tc[m2.end():]

# metadata spot-check — append before closing brace of epsg_generated_metadata_is_specific
old_meta_end = f"""    let info = epsg_info({first_code - 1 if PREV >= 21 else '__'}).unwrap();
    assert!(!info.name.is_empty());
}}"""
# simpler: just find the last spot-check entry and append
spot_marker = f"epsg_info({first_code - 1 if first_code > 10632 else 10632}).unwrap();"
# Actually use the prev batch's first code spot check
prev_first_map = {22: 10632, 23: None, 24: None, 25: None, 26: None, 27: None}
prev_first = prev_first_map.get(N)
if prev_first is None:
    # find from file: last epsg_info assertion before closing brace of metadata test
    m3 = re.search(r'epsg_info\((\d+)\)\.unwrap\(\);\s*assert!\(!info\.name\.is_empty\(\)\);\s*\}',tc)
    if m3:
        prev_first = int(m3.group(1))

if prev_first:
    old_spot = f"""    let info = epsg_info({prev_first}).unwrap();
    assert!(!info.name.is_empty());
}}"""
    new_spot = f"""    let info = epsg_info({prev_first}).unwrap();
    assert!(!info.name.is_empty());

    let info = epsg_info({first_code}).unwrap();
    assert!(!info.name.is_empty());
}}"""
    if old_spot in tc:
        tc = tc.replace(old_spot, new_spot, 1)
    else:
        print(f"  WARN: could not add spot-check for {first_code}")

with open(TESTS_RS, "w") as f:
    f.write(tc)
print("  epsg_tests.rs updated")

# ─── Step 5: update README ────────────────────────────────────────────────────

print(f"\n[5] Updating README…")
with open(README_MD) as f:
    rm = f.read()

prev_epsg  = len(known) - 3   # subtract the 3 ESRI codes
prev_total = len(known)
new_epsg   = prev_epsg  + len(accepted)
new_total  = prev_total + len(accepted)

for old_str, new_str in [
    (f"**{prev_epsg} EPSG codes** (**{prev_total} total CRS/projection codes**",
     f"**{new_epsg} EPSG codes** (**{new_total} total CRS/projection codes**"),
    (f"**{prev_epsg} EPSG codes** and **{prev_total} total CRS/projection codes**",
     f"**{new_epsg} EPSG codes** and **{new_total} total CRS/projection codes**"),
    (f"({prev_epsg} EPSG codes / {prev_total} total CRS/projection codes",
     f"({new_epsg} EPSG codes / {new_total} total CRS/projection codes"),
]:
    rm = rm.replace(old_str, new_str)

with open(README_MD, "w") as f:
    f.write(rm)
print(f"  README: {prev_epsg}/{prev_total} → {new_epsg}/{new_total}")

# ─── Step 6: insert metadata arms ────────────────────────────────────────────

print(f"\n[6] Inserting EpsgInfo metadata…")
codes = [code for code, _ in accepted]
arms = []
for code in codes:
    data = projinfo_json(code)
    name  = data.get("name", "")
    area  = ""
    a     = data.get("bbox") or data.get("area_of_use", {})
    if isinstance(a, dict): area = a.get("name", "")
    if not area:
        a2 = data.get("area_of_use", {})
        if isinstance(a2, dict): area = a2.get("name", "")
        elif isinstance(a2, str): area = a2
    unit = ""
    cs   = data.get("coordinate_system") or {}
    axes = cs.get("axis") or []
    if axes:
        u = axes[0].get("unit", {})
        unit = u.get("name", "") if isinstance(u, dict) else (u if isinstance(u, str) else "")
    if not unit: unit = "metre"
    def esc(s): return s.replace('\\', '\\\\').replace('"', '\\"')
    arms.append(
        f'        {code} => Some(EpsgInfo {{\n'
        f'            code: {code},\n'
        f'            name: "{esc(name)}",\n'
        f'            area_of_use: "{esc(area)}",\n'
        f'            unit: "{esc(unit)}",\n'
        f'        }}),\n'
    )

with open(INFO_RS) as f:
    ic = f.read()

start_marker = f"        {first_code} => Some(EpsgInfo {{"
if start_marker in ic:
    print("  Already inserted — skipping")
else:
    target = "        _ => None,\n    }\n}"
    ic = ic.replace(target, "".join(arms) + target, 1)
    with open(INFO_RS, "w") as f:
        f.write(ic)
    print(f"  Inserted {len(arms)} arms")

# ─── Step 7: corpus gates ─────────────────────────────────────────────────────

print(f"\n[7] Running corpus gates…")
gate_tests = [
    f"epsg_generated_batch{N}_codes_build_ok",
    f"known_epsg_codes_includes_generated_batch{N}_tail_code",
    "known_epsg_codes_have_wkt",
    "from_wkt_parses_exporter_methods_for_all_known_codes",
]
test_filter = " ".join(gate_tests)
out, err, rc = run(f"cargo test --manifest-path Cargo.toml -- {test_filter} --nocapture 2>&1")
print(out[-3000:] if len(out) > 3000 else out)
if rc != 0 or "FAILED" in out:
    print(f"\n  ERROR: corpus gates failed for batch {N}")
    sys.exit(rc or 1)
print("  All corpus gates PASSED")

# ─── Step 8: full regression ──────────────────────────────────────────────────

print(f"\n[8] Full regression…")
out, err, rc = run("cargo test --manifest-path Cargo.toml -- --nocapture 2>&1")
summary_lines = [l for l in out.splitlines() if "test result" in l or "FAILED" in l]
for l in summary_lines:
    print(f"  {l}")
if rc != 0 or any("FAILED" in l for l in summary_lines):
    print("  ERROR: regression failed"); sys.exit(1)
print("  Full regression PASSED")

# ─── Done ─────────────────────────────────────────────────────────────────────

print(f"\n✓ Batch {N} complete: codes {first_code}–{last_code}, count={len(accepted)}")
print(f"  Registry now: {new_epsg} EPSG / {new_total} total")
