#!/usr/bin/env python3
import re
import sys
from pathlib import Path
from collections import defaultdict

# Read scan results
scan_file = Path("tools/p0_unwrap_scan.txt")
if not scan_file.exists():
    print("tools/p0_unwrap_scan.txt not found")
    sys.exit(1)

lines = scan_file.read_text().splitlines()

# Categorize by file
by_file = defaultdict(list)
for line in lines:
    match = re.match(r'^([^:]+):(\d+):(.+)$', line)
    if match:
        file_path, line_no, content = match.groups()
        by_file[file_path].append((int(line_no), content.strip()))

# Check if line is in test module
def is_test_line(file_path, line_no):
    try:
        with open(file_path) as f:
            lines = f.readlines()
        
        # Check if in #[cfg(test)] or mod tests block
        in_test_mod = False
        for i in range(max(0, line_no - 100), line_no):
            if i < len(lines):
                l = lines[i].strip()
                if 'mod tests' in l or '#[cfg(test)]' in l:
                    in_test_mod = True
                if in_test_mod and l.startswith('mod ') and 'tests' in l:
                    return True
                if '#[test]' in l:
                    return True
        
        # Also check the line itself
        if line_no <= len(lines):
            line_text = lines[line_no-1]
            if'#[test]' in line_text or 'mod tests' in line_text:
                return True
    except:
        pass
    return False

# Categorize
prod_unwraps = {}
test_unwraps = {}

for file_path, occurrences in by_file.items():
    if '.tmp:' in file_path:
        continue  # Skip temp files
    
    prod = []
    test = []
    
    for line_no, content in occurrences:
        if is_test_line(file_path, line_no):
            test.append((line_no, content))
        else:
            prod.append((line_no, content))
    
    if prod:
        prod_unwraps[file_path] = prod
    if test:
        test_unwraps[file_path] = test

# Report
print("=" * 80)
print("P0 PRODUCTION unwrap/expect/panic INVENTORY")
print("=" * 80)
print()

if not prod_unwraps:
    print("✅ NO PRODUCTION UNWRAPS FOUND!")
else:
    print(f"Found {sum(len(v) for v in prod_unwraps.values())} production unwraps in {len(prod_unwraps)} files:")
    print()
    
    for file_path in sorted(prod_unwraps.keys()):
        occurrences = prod_unwraps[file_path]
        print(f"\n📁 {file_path} ({len(occurrences)} occurrences)")
        for line_no, content in occurrences[:5]:  # Show first 5
            print(f"   L{line_no}: {content[:70]}")
        if len(occurrences) > 5:
            print(f"   ... and {len(occurrences) - 5} more")

print()
print("=" * 80)
print(f"Test unwraps: {sum(len(v) for v in test_unwraps.values())} (acceptable)")
print("=" * 80)
