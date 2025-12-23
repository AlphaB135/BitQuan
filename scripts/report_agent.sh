#!/bin/bash

# Report Agent - Comprehensive Codebase Analysis Tool
# Usage: ./scripts/report_agent.sh [report_type] [target] [options]
#
# Report Types:
#   - migration: Async migration progress and blockers
#   - performance: Performance bottlenecks and optimizations
#   - security: Security vulnerabilities and recommendations
#   - architecture: Design patterns and structural improvements
#   - comprehensive: All of the above
#
# Target: directory, file, or component to analyze
# Options:
#   --output-dir: Directory to save reports (default: reports/)
#   --format: output format (markdown, json, html)
#   --depth: analysis depth (quick, standard, deep)
#   --include-tests: include test files in analysis

set -euo pipefail

# Default values
REPORT_TYPE=${1:-"comprehensive"}
TARGET=${2:-"."}
OUTPUT_DIR="reports"
FORMAT="markdown"
DEPTH="standard"
INCLUDE_TESTS=false
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Parse arguments
shift 2
while [[ $# -gt 0 ]]; do
  case $1 in
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --format)
      FORMAT="$2"
      shift 2
      ;;
    --depth)
      DEPTH="$2"
      shift 2
      ;;
    --include-tests)
      INCLUDE_TESTS=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Report filename
REPORT_FILE="$OUTPUT_DIR/report_${REPORT_TYPE}_${TIMESTAMP}.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}🔍 Report Agent Starting...${NC}"
echo -e "${BLUE}Report Type:${NC} $REPORT_TYPE"
echo -e "${BLUE}Target:${NC} $TARGET"
echo -e "${BLUE}Output:${NC} $REPORT_FILE"
echo ""

# Function to analyze file structure
analyze_structure() {
  local target="$1"

  echo -e "${YELLOW}Analyzing project structure...${NC}"

  # Count different file types
  local rust_files=$(find "$target" -name "*.rs" -type f | wc -l)
  local toml_files=$(find "$target" -name "Cargo.toml" -type f | wc -l)
  local json_files=$(find "$target" -name "*.json" -type f | wc -l)
  local md_files=$(find "$target" -name "*.md" -type f | wc -l)

  # Get project stats
  local total_lines=$(find "$target" -name "*.rs" -type f -exec wc -l {} + | tail -1 | awk '{print $1}' || echo "0")

  cat << EOF
### Project Structure Analysis
- **Rust Files**: $rust_files
- **Cargo.toml Files**: $toml_files
- **JSON Files**: $json_files
- **Markdown Files**: $md_files
- **Total Lines of Code**: $total_lines

EOF
}

# Function to analyze async migration
analyze_migration() {
  local target="$1"

  echo -e "${YELLOW}Analyzing async migration status...${NC}"

  # Search for async/await patterns
  local async_count=$(rg -c "async fn" "$target" --type rust 2>/dev/null || echo "0")
  local await_count=$(rg -c "\.await" "$target" --type rust 2>/dev/null || echo "0")
  local tokio_usage=$(rg -c "tokio::" "$target" --type rust 2>/dev/null || echo "0")

  # Find blocking patterns
  local blocking_patterns=$(rg -n "std::thread::|thread::sleep|block_on" "$target" --type rust 2>/dev/null || echo "No blocking patterns found")

  # Check migration audit
  local audit_file="async_migration_audit_report.md"
  local audit_status="Not found"

  if [[ -f "$audit_file" ]]; then
    audit_status="Found ($(stat -f%z "$audit_file" 2>/dev/null || stat -c%s "$audit_file" 2>/dev/null) bytes)"
  fi

  cat << EOF
### Async Migration Status
- **Async Functions**: $async_count
- **Await Usage**: $await_count
- **Tokio Dependencies**: $tokio_usage
- **Migration Audit**: $audit_status

#### Blocking Patterns Found
\`\`\`
$blocking_patterns
\`\`\`

EOF
}

# Function to analyze performance
analyze_performance() {
  local target="$1"

  echo -e "${YELLOW}Analyzing performance characteristics...${NC}"

  # Find potential bottlenecks
  local clones=$(rg -n "\.clone\(\)" "$target" --type rust 2>/dev/null | head -10 || echo "No excessive clones found")
  local loops=$(rg -n "for .* in|loop |while " "$target" --type rust 2>/dev/null | head -10 || echo "No loops found")
  local allocations=$(rg -n "Box::new|Vec::new|HashMap::new" "$target" --type rust 2>/dev/null | head -10 || echo "No allocations found")

  cat << EOF
### Performance Analysis

#### Potential Clones (Top 10)
\`\`\`
$clones
\`\`\`

#### Loop Patterns (Top 10)
\`\`\`
$loops
\`\`\`

#### Memory Allocations (Top 10)
\`\`\`
$allocations
\`\`\`

EOF
}

# Function to analyze security
analyze_security() {
  local target="$1"

  echo -e "${YELLOW}Analyzing security patterns...${NC}"

  # Search for security-sensitive patterns
  local unsafe_blocks=$(rg -n "unsafe " "$target" --type rust 2>/dev/null || echo "No unsafe blocks found")
  local panics=$(rg -n "panic!|unwrap\(\)|expect\(" "$target" --type rust 2>/dev/null || echo "No panics found")
  local crypto_usage=$(rg -n "hash|sha|crypto|encrypt|decrypt" "$target" --type rust 2>/dev/null || echo "No crypto usage found")

  cat << EOF
### Security Analysis

#### Unsafe Blocks
\`\`\`
$unsafe_blocks
\`\`\`

#### Potential Panics
\`\`\`
$panics
\`\`\`

#### Cryptographic Usage
\`\`\`
$crypto_usage
\`\`\`

EOF
}

# Function to analyze architecture
analyze_architecture() {
  local target="$1"

  echo -e "${YELLOW}Analyzing architecture patterns...${NC}"

  # Find architectural elements
  local modules=$(rg -n "^mod " "$target" --type rust 2>/dev/null | head -20 || echo "No modules found")
  local structs=$(rg -n "^pub struct " "$target" --type rust 2>/dev/null | head -20 || echo "No public structs found")
  local traits=$(rg -n "^pub trait " "$target" --type rust 2>/dev/null | head -10 || echo "No public traits found")

  # Dependencies
  if [[ -f "Cargo.toml" ]]; then
    local dependencies=$(grep -E "^\[dependencies\]" -A 50 Cargo.toml | grep -v "^\[" | grep -v "^$" || echo "No dependencies found")
  fi

  cat << EOF
### Architecture Analysis

#### Module Structure
\`\`\`
$modules
\`\`\`

#### Public Structs (Top 20)
\`\`\`
$structs
\`\`\`

#### Public Traits (Top 10)
\`\`\`
$traits
\`\`\`

#### Dependencies
\`\`\toml
$dependencies
\`\`\`

EOF
}

# Function to generate recommendations
generate_recommendations() {
  local report_type="$1"

  cat << EOF
## Recommendations

### Priority 0 (Critical)
- [ ] Complete async migration in network and consensus modules
- [ ] Remove all unwrap() calls that could cause panics
- [ ] Add proper error handling for all external calls

### Priority 1 (High)
- [ ] Optimize excessive .clone() calls identified in performance analysis
- [ ] Add comprehensive unit tests for core consensus logic
- [ ] Implement proper logging for debugging async operations

### Priority 2 (Medium)
- [ ] Document architectural patterns and module responsibilities
- [ ] Add integration tests for cross-module interactions
- [ ] Consider adding benchmarks for performance-critical paths

### Priority 3 (Low)
- [ ] Standardize code formatting across all modules
- [ ] Add inline documentation for public APIs
- [ ] Consider extracting common patterns into utility modules

EOF
}

# Function to create file references
create_file_references() {
  local target="$1"

  echo -e "${YELLOW}Creating file references...${NC}"

  # Find important files
  local main_lib=$(find "$target" -name "lib.rs" -type f | head -5)
  local main_files=$(find "$target" -name "main.rs" -type f | head -5)
  local test_files=$(find "$target" -name "*test*.rs" -type f | head -5)
  local config_files=$(find "$target" -name "*.toml" -o -name "*.json" -o -name "*.yaml" -o -name "*.yml" | head -10)

  cat << EOF
## File References

### Core Library Files
EOF

  for file in $main_lib; do
    echo "- \`$file\` - Main library entry point"
  done

  cat << EOF

### Binary Entry Points
EOF

  for file in $main_files; do
    echo "- \`$file\` - Application entry point"
  done

  cat << EOF

### Configuration Files
EOF

  for file in $config_files; do
    echo "- \`$file\` - Configuration file"
  done

  if [[ "$INCLUDE_TESTS" == "true" ]]; then
    cat << EOF

### Test Files
EOF
    for file in $test_files; do
      echo "- \`$file\` - Test implementation"
    done
  fi

  echo ""
}

# Main report generation
generate_report() {
  local report_type="$1"
  local target="$2"

  echo -e "${GREEN}Generating $report_type report...${NC}"

  # Start building the report
  cat > "$REPORT_FILE" << EOF
# Report: ${report_type^} Analysis

**Generated**: $(date)
**Target**: $target
**Analysis Depth**: $DEPTH
**Agent Version**: 1.0.0

## Executive Summary

EOF

  # Add sections based on report type
  case $report_type in
    "migration")
      analyze_migration "$target" >> "$REPORT_FILE"
      analyze_structure "$target" >> "$REPORT_FILE"
      ;;
    "performance")
      analyze_performance "$target" >> "$REPORT_FILE"
      analyze_structure "$target" >> "$REPORT_FILE"
      ;;
    "security")
      analyze_security "$target" >> "$REPORT_FILE"
      analyze_structure "$target" >> "$REPORT_FILE"
      ;;
    "architecture")
      analyze_architecture "$target" >> "$REPORT_FILE"
      analyze_structure "$target" >> "$REPORT_FILE"
      ;;
    "comprehensive")
      analyze_structure "$target" >> "$REPORT_FILE"
      analyze_migration "$target" >> "$REPORT_FILE"
      analyze_performance "$target" >> "$REPORT_FILE"
      analyze_security "$target" >> "$REPORT_FILE"
      analyze_architecture "$target" >> "$REPORT_FILE"
      ;;
  esac

  # Add common sections
  generate_recommendations "$report_type" >> "$REPORT_FILE"
  create_file_references "$target" >> "$REPORT_FILE"

  # Add footer
  cat >> "$REPORT_FILE" << EOF

---

## Report Metadata

- **Analysis Duration**: $(SECONDS)s
- **Files Analyzed**: $(find "$target" -type f | wc -l)
- **Report Format**: $FORMAT
- **Next Review**: Recommended within 7 days

## Generated By

Report Agent v1.0.0 - Automated Codebase Analysis Tool
EOF
}

# Execute the analysis
SECONDS=0
generate_report "$REPORT_TYPE" "$TARGET"

# Success message
echo -e "${GREEN}✅ Report generated successfully!${NC}"
echo -e "${GREEN}📄 Report saved to:${NC} $REPORT_FILE"
echo ""
echo -e "${BLUE}Quick stats:${NC}"
echo "- Files analyzed: $(find "$TARGET" -type f | wc -l)"
echo "- Analysis time: ${SECONDS}s"
echo "- Report size: $(stat -f%z "$REPORT_FILE" 2>/dev/null || stat -c%s "$REPORT_FILE" 2>/dev/null) bytes"

# Offer to open the report
if command -v code >/dev/null 2>&1; then
  echo ""
  read -p "Open report in VS Code? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    code "$REPORT_FILE"
  fi
fi

# Offer to show preview
echo ""
read -p "Show report preview? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
  echo ""
  echo -e "${BLUE}=== Report Preview ===${NC}"
  head -50 "$REPORT_FILE"
  echo ""
  echo -e "${YELLOW}... (truncated)${NC}"
fi
