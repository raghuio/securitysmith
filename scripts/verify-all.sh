#!/usr/bin/env bash
# Post-Code Verification Script for SecuritySmith
# Run all checks: formatting, compilation, tests, linting, security scan, artifact check.
# This is NOT meant to run after every edit — run once before committing or at end of day.
#
# Usage: ./scripts/verify-all.sh

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

check_ok() { printf "  ${GREEN}✓${NC} %s\n" "$1"; }
check_fail() { printf "  ${RED}✗${NC} %s\n" "$1"; ERRORS=$((ERRORS + 1)); }
check_info() { printf "  ${YELLOW}!${NC} %s\n" "$1"; }

echo "═══════════════════════════════════════════════════════════════"
echo "  SecuritySmith Full Verification"
echo "  Run before committing or at end of day."
echo "═══════════════════════════════════════════════════════════════"

# ─── 1. Rust Format Check ─────────────────────────────────────
echo ""
echo "📐 1. Rust formatting (cargo fmt --check)"
if (cd src-tauri && cargo fmt -- --check > /dev/null 2>&1); then
    check_ok "Formatting clean"
else
    check_fail "Formatting violations detected"
    check_info "Fix: cd src-tauri && cargo fmt"
fi

# ─── 2. Rust Dependency Audit ─────────────────────────────────
echo ""
echo "📦 2. Rust dependency audit (cargo audit)"
if (cd src-tauri && cargo audit --quiet > /dev/null 2>&1); then
    check_ok "No known Rust dependency vulnerabilities"
else
    check_fail "Dependency vulnerabilities detected"
    check_info "Fix: cd src-tauri && cargo audit"
fi

# ─── 3. Rust Linting (Clippy) ─────────────────────────────────
echo ""
echo "🔍 3. Rust linting (cargo clippy)"
if (cd src-tauri && cargo clippy -- -D warnings > /dev/null 2>&1); then
    check_ok "No clippy warnings"
else
    check_fail "Clippy warnings or errors detected"
    check_info "Fix: cd src-tauri && cargo clippy"
fi

# ─── 4. Rust Compilation ──────────────────────────────────────
echo ""
echo "🔨 4. Rust compilation (cargo check)"
if (cd src-tauri && cargo check > /dev/null 2>&1); then
    check_ok "Compiles successfully"
else
    check_fail "Compilation failed"
fi

# ─── 5. Rust Unit Tests ───────────────────────────────────────
echo ""
echo "🧪 5. Rust unit tests (cargo test)"
if (cd src-tauri && cargo test --quiet > /dev/null 2>&1); then
    check_ok "All tests passed"
else
    check_fail "Tests failed"
fi

# ─── 6. Frontend Formatting ─────────────────────────────────
echo ""
echo "📐 6. Frontend formatting (prettier --check)"
if (npx prettier --check "src/**/*.{ts,tsx}" > /dev/null 2>&1); then
    check_ok "Prettier formatting clean"
else
    check_fail "Prettier formatting violations detected"
    check_info "Fix: npx prettier --write 'src/**/*.{ts,tsx}'"
fi

# ─── 7. Frontend Build ────────────────────────────────────────
echo ""
echo "🌐 7. Frontend build (npm run build)"
if (npm run build > /dev/null 2>&1); then
    check_ok "Frontend builds successfully"
else
    check_fail "Frontend build failed"
fi

# ─── 8. Frontend Dependency Audit ─────────────────────────────
echo ""
echo "📦 8. Frontend dependency audit (npm audit)"
if (npm audit --audit-level=high > /dev/null 2>&1); then
    check_ok "No high/critical frontend dependency vulnerabilities"
else
    check_fail "Frontend dependency vulnerabilities detected"
    check_info "Fix: npm audit fix"
fi

# ─── 9. Security Scan (Semgrep — local rules, updated daily) ──────
echo ""
echo "🔒 9. Security static analysis (semgrep — local cached rules, updated daily)"

# Auto-download on first run if rules are missing
if [ ! -d ".semgrep/rules" ]; then
    check_info "First run: downloading Semgrep community rules..."
    if (git clone --depth 1 https://github.com/semgrep/semgrep-rules.git .semgrep/rules > /dev/null 2>&1); then
        check_ok "Rules cached locally"
    else
        check_fail "Failed to download Semgrep rules"
        ERRORS=$((ERRORS + 1))
    fi
fi

# Update rules daily if older than 24 hours
if [ -d ".semgrep/rules/.git" ]; then
    if [ ! -f ".semgrep/rules/.last_update" ] || [ "$(find .semgrep/rules/.last_update -mtime +0 2>/dev/null)" ]; then
        check_info "Updating Semgrep rules (daily check)..."
        if (cd .semgrep/rules && git pull --ff-only > /dev/null 2>&1); then
            touch .semgrep/rules/.last_update
            check_ok "Rules updated"
        else
            check_info "Rule update failed (proceeding with cached rules)"
            touch .semgrep/rules/.last_update
        fi
    fi
fi

echo "     Check rules:"
echo "               .semgrep/rules/rust/"
echo "               .semgrep/rules/typescript/"
echo "               .semgrep/rules/javascript/"
echo "               .semgrep/rules/generic/secrets/"

if [ -d ".semgrep/rules" ]; then
    SEMGREP_OUTPUT=$(semgrep \
        --config .semgrep/rules/rust/ \
        --config .semgrep/rules/typescript/ \
        --config .semgrep/rules/javascript/ \
        --config .semgrep/rules/generic/secrets/ \
        --metrics=off \
        --quiet \
        . 2>&1) || true

    SEMGREP_FINDINGS=$(echo "$SEMGREP_OUTPUT" | grep -oP '\d+(?= Findings)' || echo "0")

    if [ "$SEMGREP_FINDINGS" = "0" ] || [ -z "$SEMGREP_FINDINGS" ]; then
        check_ok "No security findings"
    else
        check_fail "$SEMGREP_FINDINGS security finding(s) detected"
        check_info "Run manually for details:"
        check_info "  semgrep --config .semgrep/rules/rust/ --metrics=off ."
        echo "$SEMGREP_OUTPUT" | tail -n +1 | head -n 40
    fi
fi

# ─── 10. Build Artifacts ──────────────────────────────────────
echo ""
echo "📦 10. Build artifact verification"
if [ -f "src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb" ]; then
    check_ok "deb package exists"
else
    check_info "deb package not built (run: npm run tauri build)"
fi

if [ -f "src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage" ]; then
    check_ok "AppImage exists"
else
    check_info "AppImage not built (run: npm run tauri build)"
fi

# ─── Summary ──────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
if [ $ERRORS -eq 0 ]; then
    echo "  ${GREEN}ALL CHECKS PASSED${NC} — safe to proceed."
    echo "═══════════════════════════════════════════════════════════════"
    exit 0
else
    echo "  ${RED}$ERRORS CHECK(S) FAILED${NC} — fix before proceeding."
    echo "═══════════════════════════════════════════════════════════════"
    exit 1
fi
