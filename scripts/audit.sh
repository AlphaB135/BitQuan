#!/bin/bash
# Audit & Security Check Script
# Runs cargo audit, dependency checking, and coverage summary

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  BitQuan Security & Dependency Audit                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Check if tools are installed
command -v cargo-audit >/dev/null 2>&1 || {
    echo "⚠️  cargo-audit not installed. Installing..."
    cargo install cargo-audit
}

command -v cargo-deny >/dev/null 2>&1 || {
    echo "⚠️  cargo-deny not installed. Installing..."
    cargo install cargo-deny
}

command -v cargo-geiger >/dev/null 2>&1 || {
    echo "⚠️  cargo-geiger not installed. Installing..."
    cargo install cargo-geiger
}

command -v cargo-llvm-cov >/dev/null 2>&1 || {
    echo "⚠️  cargo-llvm-cov not installed. Installing..."
    cargo install cargo-llvm-cov
}

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📋 Running Security Vulnerability Scan"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo audit --color always || {
    echo "❌ Security vulnerabilities found!"
    exit 1
}
echo "✅ No known security vulnerabilities"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📜 Checking Licenses"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo deny check licenses || {
    echo "❌ License compatibility issues found!"
    exit 1
}
echo "✅ All licenses compatible"
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔬 Checking for Unsafe Code"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo geiger --color always || {
    echo "⚠️  Unsafe code detected (review required)"
}
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Code Coverage Summary"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo llvm-cov --summary-only --color always 2>/dev/null || {
    echo "⚠️  Coverage calculation skipped (requires llvm-tools)"
}
echo ""

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ Audit Complete                                       ║"
echo "╚══════════════════════════════════════════════════════════╝"
