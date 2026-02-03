#!/usr/bin/env python3
"""
Dead Code Cleanup Script for BitQuan

Scans for #[allow(dead_code)] and automatically removes unused code.
Run this after removing #[allow(dead_code)] to clean up the leftovers.
"""

import re
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

# ANSI colors
GREEN = "\033[92m"
RED = "\033[91m"
YELLOW = "\033[93m"
BLUE = "\033[94m"
RESET = "\033[0m"


def find_rust_files(root_dir: Path) -> List[Path]:
    """Find all .rs files in the project."""
    rust_files = []
    for path in root_dir.rglob("*.rs"):
        # Skip test files and external repos
        full_path = str(path)
        if ("tests" not in full_path and
            "ψ/" not in full_path and
            "/target/" not in full_path):
            rust_files.append(path)
    return rust_files


def find_dead_code_allowances(file_path: Path) -> List[Tuple[int, str]]:
    """
    Find #[allow(dead_code)] in a file with context.
    Returns list of (line_number, line_content) tuples.
    """
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except Exception as e:
        print(f"{RED}Error reading {file_path}: {e}{RESET}")
        return []

    allowances = []
    for i, line in enumerate(lines, 1):
        if '#[allow(dead_code)]' in line:
            allowances.append((i, line.strip()))
    return allowances


def extract_function_name(line: str, context_lines: List[str]) -> str:
    """Extract function/struct/field name from a line."""
    # Try to extract from the line with allowance
    # or from the next line (if declaration follows)

    # Pattern 1: Function with allowance on same line
    fn_match = re.search(r'fn\s+(\w+)', line)
    if fn_match:
        return fn_match.group(1)

    # Pattern 2: Struct field with allowance
    field_match = re.search(r'(\w+)\s*:', line)
    if field_match:
        return field_match.group(1)

    # Pattern 3: Check next line for declaration
    if context_lines:
        next_line = context_lines[0] if context_lines else ""
        fn_match = re.search(r'fn\s+(\w+)', next_line)
        if fn_match:
            return fn_match.group(1)

        struct_match = re.search(r'struct\s+(\w+)', next_line)
        if struct_match:
            return struct_match.group(1)

        enum_match = re.search(r'enum\s+(\w+)', next_line)
        if enum_match:
            return enum_match.group(1)

    return "unknown"


def is_actually_used(item_name: str, file_path: Path, project_root: Path) -> bool:
    """
    Check if a function/struct/field is actually used elsewhere in the codebase.
    """
    item_name = re.escape(item_name)

    # Skip common patterns that are used externally
    if item_name in ['new', 'default', 'clone', 'from', 'to']:
        return True

    # Search for references in all Rust files
    try:
        result = subprocess.run(
            ['rg', '--type', 'rust', item_name, str(project_root)],
            capture_output=True,
            text=True,
            timeout=30
        )

        if result.returncode == 0:
            matches = result.stdout.strip().split('\n')
            # Filter out the file itself
            matches = [m for m in matches if m and str(file_path) not in m]

            # Also exclude test files
            matches = [m for m in matches if 'tests' not in m]

            return len(matches) > 2  # Allow some false positives
    except Exception:
        pass  # ripgrep may not be installed, skip advanced detection

    return False


def remove_dead_code_block(file_path: Path, line_number: int, dry_run: bool = True) -> bool:
    """
    Remove a dead code block (function/struct/field) starting at line_number.
    Returns True if successful.
    """
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            lines = f.readlines()

        if line_number > len(lines):
            return False

        # Find the block to remove
        start_line = line_number - 1  # Convert to 0-indexed

        # Determine the block type and find its end
        current_line = lines[start_line]

        # Look for the end of the block
        end_line = start_line + 1

        # For functions: find closing brace
        if 'fn ' in current_line or 'async fn ' in current_line:
            brace_count = 0
            found_opening = False
            for i in range(start_line, len(lines)):
                line = lines[i]
                if '{' in line:
                    found_opening = True
                    brace_count += line.count('{')
                if '}' in line:
                    brace_count -= line.count('}')

                if found_opening and brace_count == 0:
                    end_line = i + 1
                    break

        # For single-line fields
        elif ';' in current_line and '{' not in current_line:
            end_line = start_line + 1

        # For structs
        elif 'struct ' in current_line:
            brace_count = 0
            for i in range(start_line, len(lines)):
                line = lines[i]
                if '{' in line:
                    brace_count += line.count('{')
                if '}' in line:
                    brace_count -= line.count('}')

                if brace_count == 0 and '}' in line:
                    end_line = i + 1
                    break

        # Remove the lines (including empty lines before/after)
        # Find the range to remove
        remove_start = start_line
        while remove_start > 0 and not lines[remove_start - 1].strip():
            remove_start -= 1

        remove_end = end_line
        while remove_end < len(lines) and not lines[remove_end].strip():
            remove_end += 1

        # Remove lines
        new_lines = lines[:remove_start] + lines[remove_end:]

        if dry_run:
            print(f"  {YELLOW}Would remove lines {remove_start+1} to {remove_end}:{RESET}")
            print(f"    {lines[start_line].strip()}")
            return False
        else:
            with open(file_path, 'w', encoding='utf-8') as f:
                f.writelines(new_lines)
            return True

    except Exception as e:
        print(f"  {RED}Error processing {file_path}: {e}{RESET}")
        return False


def scan_and_clean(project_root: Path, dry_run: bool = True):
    """Scan for dead code and clean it up."""

    print(f"{BLUE}Scanning for #[allow(dead_code)]...{RESET}\n")

    rust_files = find_rust_files(project_root)
    print(f"Found {len(rust_files)} Rust files (excluding tests/)\n")

    total_allowances = 0
    files_with_allowances = 0
    removed_count = 0

    for file_path in rust_files:
        allowances = find_dead_code_allowances(file_path)

        if allowances:
            files_with_allowances += 1
            print(f"\n{file_path.relative_to(project_root)}:")
            print(f"  {GREEN}{len(allowances)} allowance(s) found{RESET}")

            for line_num, line in allowances:
                total_allowances += 1
                print(f"  Line {line_num}: {line[:80]}...")

                # Try to extract what this is for
                # Get context around the allowance
                try:
                    with open(file_path, 'r') as f:
                        content_lines = f.readlines()

                    # Get a few lines of context
                    start = max(0, line_num - 2)
                    end = min(len(content_lines), line_num + 3)
                    context = content_lines[start:end]

                    # Try to extract function/struct name
                    item_name = extract_function_name(line, context[1:] if len(context) > 1 else [])

                    # Check if it's actually used
                    if item_name != "unknown":
                        is_used = is_actually_used(item_name, file_path, project_root)

                        if is_used:
                            print(f"    {BLUE}→ {item_name}: ACTUALLY USED - keeping{RESET}")
                        else:
                            print(f"    {YELLOW}→ {item_name}: NOT USED - candidate for removal{RESET}")

                            if not dry_run:
                                if remove_dead_code_block(file_path, line_num, dry_run=False):
                                    removed_count += 1
                                    print(f"    {GREEN}✓ Removed{RESET}")
                            else:
                                removed_count += 1
                except Exception as e:
                    print(f"    {RED}Error checking {file_path}:{line_num}: {e}{RESET}")

    print(f"\n{BLUE}{'='*60}{RESET}")
    print(f"{BLUE}Summary:{RESET}")
    print(f"  Files scanned: {len(rust_files)}")
    print(f"  Files with allowances: {files_with_allowances}")
    print(f"  Total allowances: {total_allowances}")
    print(f"  Candidate for removal: {removed_count}")

    if dry_run:
        print(f"\n{YELLOW}DRY RUN MODE - No files were modified{RESET}")
        print(f"Run with: python scripts/cleanup_dead_code.py --apply")
    else:
        print(f"\n{GREEN}APPLIED - Removed {removed_count} dead code blocks{RESET}")
        print(f"Next steps:")
        print(f"  1. Run: cargo clippy --all-targets --all-features -- -D warnings")
        print(f"  2. Run: cargo test --all")
        print(f"  3. Commit changes")


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Clean up dead code in Rust project')
    parser.add_argument('--apply', action='store_true', help='Actually remove dead code (not dry-run)')
    parser.add_argument('--project-root', type=Path, default=None, help='Project root directory')

    args = parser.parse_args()

    # Determine project root
    if args.project_root:
        project_root = args.project_root
    else:
        # Assume script is in scripts/ directory
        script_path = Path(__file__).resolve()
        project_root = script_path.parent.parent
        if not (project_root / 'Cargo.toml').exists():
            print(f"{RED}Error: Cargo.toml not found in {project_root}{RESET}")
            sys.exit(1)

    dry_run = not args.apply

    print(f"{BLUE}BitQuan Dead Code Cleanup Script{RESET}")
    print(f"Project root: {project_root}")
    print(f"Mode: {'DRY RUN' if dry_run else 'APPLY'}")
    print()

    scan_and_clean(project_root, dry_run=dry_run)


if __name__ == '__main__':
    main()
