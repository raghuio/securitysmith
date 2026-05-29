#!/usr/bin/env bash
# SecuritySmith Build Verification — Produces real installable artifacts.
# This runs `npm run tauri build`, which compiles the Rust binary + bundles.
# Slow: ~3-5 minutes. Run after verify-all.sh passes, before releasing.
#
# Usage: ./scripts/verify-build.sh

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
echo "  SecuritySmith Build Verification"
echo "  Produces real installable artifacts (slow)."
echo "═══════════════════════════════════════════════════════════════"

# ─── 1. Clean and build ───────────────────────────────────────
echo ""
echo "🔨 1. Running npm run tauri build (this may take a few minutes)..."
echo ""
if (npm run tauri build > /tmp/tauri-build.log 2>&1); then
    check_ok "Tauri build completed"
else
    check_fail "Tauri build failed"
    check_info "See /tmp/tauri-build.log for details"
    ERRORS=$((ERRORS + 1))
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  ${RED}BUILD FAILED${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    exit 1
fi

# ─── 2. Verify frontend artifacts ─────────────────────────────
echo ""
echo "🔧 2. Frontend artifacts"
if [ -f "dist/index.html" ]; then
    check_ok "dist/index.html"
else
    check_fail "dist/index.html missing"
fi

# ─── 3. Verify Rust binary ────────────────────────────────────
echo ""
echo "⚙️  3. Rust binary"
if [ -f "src-tauri/target/release/securitysmith" ]; then
    SIZE=$(du -sh src-tauri/target/release/securitysmith | cut -f1)
    check_ok "src-tauri/target/release/securitysmith   ($SIZE)"
else
    check_fail "Binary missing"
fi

# ─── 4. Verify installable bundles ────────────────────────────
echo ""
echo "📦 4. Installable bundles"
BUNDLES=0

if [ -f "src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb" ]; then
    SIZE=$(du -sh src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb | cut -f1)
    check_ok "deb package ($SIZE)"
    BUNDLES=$((BUNDLES + 1))
else
    check_fail ".deb missing"
fi

if [ -f "src-tauri/target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm" ]; then
    SIZE=$(du -sh src-tauri/target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm | cut -f1)
    check_ok "rpm package ($SIZE)"
    BUNDLES=$((BUNDLES + 1))
else
    check_fail ".rpm missing"
fi

if [ -f "src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage" ]; then
    SIZE=$(du -sh src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage | cut -f1)
    check_ok "AppImage ($SIZE)"
    BUNDLES=$((BUNDLES + 1))
else
    check_fail ".AppImage missing"
fi

# ─── 5. Rust tests ────────────────────────────────────────────
echo ""
echo "🧪 5. Rust unit tests"
if (cd src-tauri && cargo test --quiet > /dev/null 2>&1); then
    check_ok "All tests passed"
else
    check_fail "Tests failed"
    ERRORS=$((ERRORS + 1))
fi

# ─── Summary ──────────────────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════════════════════════"
if [ $ERRORS -eq 0 ] && [ $BUNDLES -gt 0 ]; then
    echo "  ${GREEN}All verifications passed.${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    exit 0
else
    echo "  ${RED}$ERRORS CHECK(S) FAILED${NC}"
    echo "═══════════════════════════════════════════════════════════════"
    exit 1
fi
