#!/usr/bin/env python3
"""
Fix broken links in documentation
"""

import json
import re
import os

# Load analysis
with open('tools/phase_b_analysis.json', 'r') as f:
    analysis = json.load(f)

# Create mapping of moved files
MOVED_FILES = {
    './TESTNET_README.md': '../testnet/README.md',
    './OBSERVABILITY.md': '../ops/OBSERVABILITY.md',
    './PRELAUNCH_CHECKLIST.md': '../ops/PRELAUNCH_CHECKLIST.md',
    './MAINNET_ANNOUNCEMENT.md': '../releases/MAINNET_ANNOUNCEMENT.md',
    './STRATUM.md': '../guides/STRATUM.md',  # If exists
    'API_REFERENCE.md': '../rpc/API_REFERENCE.md',  # If exists
    './security/SECURITY.md': '../../SECURITY.md',
}

def fix_link_in_file(file_path, old_link, new_link):
    """Replace a link in a file"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
        
        # Escape special regex characters in old_link
        old_escaped = re.escape(old_link)
        
        # Replace the link
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
        print(f"Error fixing {file_path}: {e}")
        return False

def main():
    print("🔧 Fixing broken links...")
    print("=" * 60)
    
    fixed = 0
    not_fixed = 0
    
    for broken in analysis['broken']:
        file_path = os.path.join('docs', broken['file'])
        old_link = broken['link']
        
        # Try to find replacement
        new_link = None
        
        # Check known moved files
        if old_link in MOVED_FILES:
            new_link = MOVED_FILES[old_link]
        # Try common patterns
        elif old_link.startswith('./') and not '/' in old_link[2:]:
            # Might be in same directory or moved to section
            basename = old_link[2:]
            # Check if it's in ops/
            if os.path.exists(f'docs/ops/{basename}'):
                new_link = f'../ops/{basename}'
            elif os.path.exists(f'docs/security/{basename}'):
                new_link = f'../security/{basename}'
            elif os.path.exists(f'docs/testnet/{basename}'):
                new_link = f'../testnet/{basename}'
            elif os.path.exists(f'docs/releases/{basename}'):
                new_link = f'../releases/{basename}'
        
        if new_link:
            if fix_link_in_file(file_path, old_link, new_link):
                print(f"✓ Fixed in {broken['file']}")
                print(f"  {old_link} → {new_link}")
                fixed += 1
            else:
                not_fixed += 1
        else:
            print(f"⚠ Could not auto-fix: {broken['file']}:{broken['line']}")
            print(f"  Link: {old_link}")
            not_fixed += 1
    
    print("\n" + "=" * 60)
    print(f"✅ Fixed: {fixed}")
    print(f"⚠  Still broken: {not_fixed}")

if __name__ == '__main__':
    main()
