#!/usr/bin/env bash
set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

check_file() {
    local label="$1"
    local path="$2"
    if [ -f "$path" ]; then
        local size
        size=$(du -sh "$path" | cut -f1)
        printf "  ${GREEN}✓${NC} %-40s (%s)\n" "$label" "$size"
    else
        printf "  ${RED}✗${NC} %-40s ${RED}MISSING${NC}\n" "$label"
        ERRORS=$((ERRORS + 1))
    fi
}

echo "═══════════════════════════════════════════════════════"
echo "  SecuritySmith Build Verification"
echo "═══════════════════════════════════════════════════════"

echo ""
echo "🔧 Frontend artifacts"
check_file "dist/index.html" "dist/index.html"

echo ""
echo "⚙️  Rust binary"
check_file "src-tauri/target/release/securitysmith" "src-tauri/target/release/securitysmith"

echo ""
echo "📦 Installable bundles"
check_file "deb package (.deb)" "src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb"
check_file "rpm package (.rpm)" "src-tauri/target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm"
check_file "AppImage" "src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage"

echo ""
echo "🧪 Running Rust unit tests..."
if (cd src-tauri && cargo test --quiet 2>&1); then
    echo "  ${GREEN}✓${NC} All Rust tests passed"
else
    echo "  ${RED}✗${NC} Rust tests failed"
    ERRORS=$((ERRORS + 1))
fi

echo ""
echo "═══════════════════════════════════════════════════════"
if [ $ERRORS -eq 0 ]; then
    echo "  ${GREEN}All verifications passed.${NC}"
else
    echo "  ${RED}$ERRORS verification(s) failed.${NC}"
    echo "  Run: ${YELLOW}npm run tauri build${NC} to rebuild artifacts."
    exit 1
fi
echo "═══════════════════════════════════════════════════════"
