#!/usr/bin/env python3
"""
Smart link fixer - Phase B completion
Fixes broken links by searching for actual file locations
"""

import os
import re
import json
from pathlib import Path

def find_file_in_docs(filename):
    """Find a file anywhere in docs/"""
    if not filename:
        return None

    for root, dirs, files in os.walk('docs'):
        # Skip hidden and archive
        dirs[:] = [d for d in dirs if not d.startswith('.') and d != '_archive']

        if filename in files:
            return os.path.relpath(os.path.join(root, filename), 'docs')
    return None

def compute_relative_path(from_file, to_file):
    """Compute relative path from one file to another"""
    from_dir = os.path.dirname(from_file)
    rel_path = os.path.relpath(to_file, from_dir)
    return rel_path

def fix_link_in_file(file_path, old_link, new_link):
    """Replace a link in a file"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # Escape special regex characters
        old_escaped = re.escape(old_link)

        # Replace
        new_content = re.sub(
            rf'\[([^\]]+)\]\({old_escaped}\)',
            rf'[\1]({new_link})',
            content
        )

        if new_content != content:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.write(new_content)
            return True
        return False
    except Exception as e:
        print(f"Error: {e}")
        return False

def main():
    # Load analysis
    with open('tools/phase_b_analysis.json', 'r') as f:
        analysis = json.load(f)

    print("🔧 Smart Link Fixer - Phase B")
    print("=" * 70)

    fixed = 0
    skipped = 0
    not_found = 0

    for broken in analysis['broken']:
        file_path = os.path.join('docs', broken['file'])
        old_link = broken['link']

        # Extract filename from link
        if '/' in old_link:
            filename = old_link.split('/')[-1]
        else:
            filename = old_link

        # Remove anchor
        if '#' in filename:
            filename = filename.split('#')[0]

        # Skip empty or directory links
        if not filename or filename.endswith('/'):
            skipped += 1
            continue

        # Find actual file location
        actual_location = find_file_in_docs(filename)

        if actual_location:
            # Compute correct relative path
            from_file = os.path.join('docs', broken['file'])
            to_file = os.path.join('docs', actual_location)

            new_link = compute_relative_path(from_file, to_file)

            # Keep anchor if present
            if '#' in old_link:
                anchor = '#' + old_link.split('#')[1]
                new_link += anchor

            # Fix the link
            if fix_link_in_file(file_path, old_link, new_link):
                print(f"✓ {broken['file']}:{broken['line']}")
                print(f"  {old_link} → {new_link}")
                fixed += 1
            else:
                skipped += 1
        else:
            print(f"⚠ Not found: {filename} (referenced in {broken['file']})")
            not_found += 1

    print("\n" + "=" * 70)
    print(f"✅ Fixed: {fixed}")
    print(f"⏭  Skipped: {skipped}")
    print(f"❌ Not found: {not_found}")

    # Re-analyze
    print("\n🔄 Re-running analysis...")
    os.system('python3 tools/analyze_docs.py > /tmp/reanalysis.txt 2>&1')

    with open('/tmp/reanalysis.txt', 'r') as f:
        lines = f.readlines()
        for line in lines:
            if 'Broken links:' in line or 'SUMMARY' in line:
                print(line.strip())

if __name__ == '__main__':
    main()
