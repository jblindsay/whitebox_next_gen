import re
import subprocess

# Reuse files produced by batch_runner.
with open("/tmp/wbprojection_known_codes.txt", "r") as f:
    known = {int(line.strip()) for line in f if line.strip().isdigit()}

with open("/tmp/proj_projected_epsg_codes.txt", "r") as f:
    projected = {int(line.strip()) for line in f if line.strip().isdigit()}

candidates = sorted(projected - known)

methods = {}
for code in sorted(set(candidates)):
    wkt = subprocess.run(
        ["projinfo", f"EPSG:{code}", "-o", "WKT1:ESRI", "--single-line", "-q"],
        capture_output=True,
        text=True,
    ).stdout
    if "DerivingConversion" in wkt:
        continue
    m = re.search(r'PROJECTION\["([^"]+)"\]', wkt)
    if not m:
        continue
    key = re.sub(r"[^a-z]+", "", m.group(1).lower())
    methods.setdefault(key, []).append(code)

print("Remaining projected/compound unknown with parseable projection methods:", sum(len(v) for v in methods.values()))
for key, codes in sorted(methods.items(), key=lambda kv: len(kv[1]), reverse=True):
    print(f"{key}: {len(codes)} -> {codes}")
