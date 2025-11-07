#!/usr/bin/env python3
"""
Documentation Analyzer - Find duplicates and broken links
Phase B of documentation restructure
"""

import os
import re
import json
from pathlib import Path
from collections import defaultdict
from difflib import SequenceMatcher

def normalize_filename(name):
    """Normalize filename for duplicate detection"""
    # Remove extension
    base = name.replace('.md', '')
    # Convert to lowercase
    base = base.lower()
    # Remove special chars, keep alphanumeric
    base = re.sub(r'[^a-z0-9]+', '', base)
    return base

def get_md_files(docs_dir):
    """Get all markdown files in docs/"""
    md_files = []
    for root, dirs, files in os.walk(docs_dir):
        # Skip hidden and archive directories
        dirs[:] = [d for d in dirs if not d.startswith('.') and d != '_archive']
        for file in files:
            if file.endswith('.md') and not file.startswith('.'):
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, docs_dir)
                md_files.append({
                    'path': full_path,
                    'rel_path': rel_path,
                    'filename': file,
                    'normalized': normalize_filename(file),
                    'size': os.path.getsize(full_path)
                })
    return md_files

def find_duplicates(md_files):
    """Find potential duplicate files by normalized name"""
    by_normalized = defaultdict(list)
    for f in md_files:
        by_normalized[f['normalized']].append(f)
    
    duplicates = {k: v for k, v in by_normalized.items() if len(v) > 1}
    return duplicates

def extract_links(file_path):
    """Extract all markdown links from a file"""
    links = []
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            # Match [text](link)
            pattern = r'\[([^\]]+)\]\(([^)]+)\)'
            matches = re.findall(pattern, content)
            for text, link in matches:
                # Skip external links and anchors
                if not link.startswith('http') and not link.startswith('#'):
                    links.append({
                        'text': text,
                        'link': link,
                        'line': content[:content.find(f'[{text}]({link})')].count('\n') + 1
                    })
    except Exception as e:
        print(f"Error reading {file_path}: {e}")
    return links

def check_link(link, source_file, docs_dir):
    """Check if a link target exists"""
    source_dir = os.path.dirname(source_file)
    
    # Handle relative links
    if link.startswith('../'):
        # Relative to docs root
        target = os.path.normpath(os.path.join(docs_dir, link))
    elif link.startswith('./'):
        # Relative to current file
        target = os.path.normpath(os.path.join(source_dir, link))
    else:
        # Assume relative to current file
        target = os.path.normpath(os.path.join(source_dir, link))
    
    # Check if exists
    if os.path.exists(target):
        return True, target
    
    # If link doesn't have extension, try .md
    if not target.endswith('.md'):
        target_md = target + '.md'
        if os.path.exists(target_md):
            return True, target_md
    
    # Check if it's a directory with README.md
    if os.path.isdir(target):
        readme = os.path.join(target, 'README.md')
        if os.path.exists(readme):
            return True, readme
    
    return False, target

def find_broken_links(md_files, docs_dir):
    """Find all broken internal links"""
    broken = []
    for f in md_files:
        links = extract_links(f['path'])
        for link_info in links:
            link = link_info['link']
            exists, target = check_link(link, f['path'], docs_dir)
            if not exists:
                broken.append({
                    'file': f['rel_path'],
                    'link': link,
                    'text': link_info['text'],
                    'line': link_info['line'],
                    'attempted_target': target
                })
    return broken

def get_file_info(file_path):
    """Get detailed file info for comparison"""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            lines = content.split('\n')
            
            # Find H1
            h1 = None
            for line in lines:
                if line.startswith('# '):
                    h1 = line[2:].strip()
                    break
            
            # Count H2s
            h2_count = sum(1 for line in lines if line.startswith('## '))
            
            # Word count
            words = len(re.findall(r'\b\w+\b', content))
            
            return {
                'h1': h1,
                'h2_count': h2_count,
                'words': words,
                'lines': len(lines)
            }
    except Exception as e:
        return {'error': str(e)}

def main():
    docs_dir = 'docs'
    
    print("🔍 Phase B: Documentation Analysis")
    print("=" * 60)
    
    # Get all markdown files
    md_files = get_md_files(docs_dir)
    print(f"\n📄 Found {len(md_files)} markdown files")
    
    # Find duplicates
    print("\n🔎 Finding potential duplicates...")
    duplicates = find_duplicates(md_files)
    print(f"   Found {len(duplicates)} normalized names with multiple files")
    
    # Detailed duplicate analysis
    duplicate_details = []
    for norm_name, files in duplicates.items():
        files_with_info = []
        for f in files:
            info = get_file_info(f['path'])
            files_with_info.append({
                'path': f['rel_path'],
                'size': f['size'],
                **info
            })
        
        duplicate_details.append({
            'normalized_name': norm_name,
            'count': len(files),
            'files': files_with_info
        })
    
    # Sort by count
    duplicate_details.sort(key=lambda x: x['count'], reverse=True)
    
    # Find broken links
    print("\n🔗 Checking internal links...")
    broken_links = find_broken_links(md_files, docs_dir)
    print(f"   Found {len(broken_links)} broken internal links")
    
    # Save results
    results = {
        'total_files': len(md_files),
        'duplicate_groups': len(duplicates),
        'broken_links': len(broken_links),
        'duplicates': duplicate_details,
        'broken': broken_links
    }
    
    with open('tools/phase_b_analysis.json', 'w') as f:
        json.dump(results, f, indent=2)
    
    print(f"\n💾 Results saved to tools/phase_b_analysis.json")
    
    # Print summary
    print("\n" + "=" * 60)
    print("📊 SUMMARY")
    print("=" * 60)
    print(f"Total markdown files: {len(md_files)}")
    print(f"Duplicate groups: {len(duplicates)}")
    print(f"Broken links: {len(broken_links)}")
    
    # Top duplicates
    if duplicate_details:
        print(f"\n🔝 Top 10 duplicate groups:")
        for i, dup in enumerate(duplicate_details[:10], 1):
            print(f"{i}. '{dup['normalized_name']}' - {dup['count']} files")
            for f in dup['files']:
                print(f"   - {f['path']:<50} {f['words']:>6} words, H1: {f.get('h1', 'None')[:30]}")
    
    # Sample broken links
    if broken_links:
        print(f"\n❌ Sample broken links (showing first 10):")
        for i, link in enumerate(broken_links[:10], 1):
            print(f"{i}. {link['file']}:{link['line']}")
            print(f"   Link: {link['link']}")
            print(f"   Text: '{link['text']}'")

if __name__ == '__main__':
    main()
