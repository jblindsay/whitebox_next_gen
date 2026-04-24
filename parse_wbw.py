import re
import os
from collections import defaultdict

pyi_path = 'crates/wbw_python/whitebox_workflows/whitebox_workflows.pyi'
rs_path = 'crates/wbw_python/src/wb_environment.rs'

# 1) Parse .pyi for unresolved methods
unresolved_methods = []
current_class = None
if os.path.exists(pyi_path):
    with open(pyi_path, 'r') as f:
        for line in f:
            class_match = re.match(r'^class\s+([a-zA-Z0-9_]+)', line)
            if class_match:
                current_class = class_match.group(1)
            method_match = re.search(r'def\s+([a-zA-Z0-9_]+)\(self, \*args: Any, \*\*kwargs: Any\) -> Any', line)
            if method_match:
                unresolved_methods.append((current_class, method_match.group(1)))

# 2) Parse .rs for signatures
signatures = defaultdict(list)
if os.path.exists(rs_path):
    with open(rs_path, 'r') as f:
        content = f.read()
        # Adjusted regex to handle more variations and "fn" instead of "pub fn"
        matches = re.finditer(r'#\[pyo3\(signature\s*=\s*\((.*?)\)\)\]\s+(?:pub\s+)?fn\s+([a-zA-Z0-9_]+)', content, re.DOTALL)
        for m in matches:
            sig = m.group(1).strip()
            name = m.group(2).strip()
            signatures[name].append(sig)

# 3) Process results
unique_names = set(m[1] for m in unresolved_methods)
counts = {'exactly_one': 0, 'multiple': 0, 'none': 0}
matches_dict = {}

for name in unique_names:
    match_count = len(signatures.get(name, []))
    if match_count == 1:
        counts['exactly_one'] += 1
        matches_dict[name] = True
    elif match_count > 1:
        counts['multiple'] += 1
        matches_dict[name] = True
    else:
        counts['none'] += 1
        matches_dict[name] = False

print(f"Total unresolved methods: {len(unresolved_methods)}")
print(f"Unique method names: {len(unique_names)}")
print(f"Unique names with exactly one Rust signature: {counts['exactly_one']}")
print(f"Unique names with multiple Rust signatures: {counts['multiple']}")
print(f"Unique names with no Rust signatures: {counts['none']}")
print("\nFirst 40 unresolved methods:")
for cls, method in unresolved_methods[:40]:
    matched = "Matched" if matches_dict.get(method) else "Unmatched"
    print(f"{cls}.{method}: {matched}")
