#!/usr/bin/env bash
# SecuritySmith Build Verification — Produces real installable artifacts.
# This runs `cargo tauri build`, which compiles the Rust binary + bundles.
# Slow: ~3-5 minutes. Run after verify-all.sh passes, before releasing.
#
# Usage:
#   ./scripts/verify-build.sh         Build all bundles
#   ./scripts/verify-build.sh deb     Build .deb package only
#   ./scripts/verify-build.sh linux   Build Linux bundles (.deb + .AppImage)
#   ./scripts/verify-build.sh appimage  Build .AppImage only

set -euo pipefail

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

ERRORS=0

check_ok() { printf "  ${GREEN}✓${NC} %s\n" "$1"; }
check_fail() { printf "  ${RED}✗${NC} %s\n" "$1"; ERRORS=$((ERRORS + 1)); }
check_info() { printf "  ${YELLOW}!${NC} %s\n" "$1"; }

# ─── Parse argument ─────────────────────────────────────────
TARGET="${1:-all}"
BUNDLE_FLAG=""

if [ "$TARGET" = "linux" ]; then
    BUNDLE_FLAG="--bundles deb,appimage"
elif [ "$TARGET" = "deb" ] || [ "$TARGET" = "appimage" ]; then
    BUNDLE_FLAG="--bundles $TARGET"
elif [ "$TARGET" != "all" ]; then
    echo "Unknown target: $TARGET"
    echo "Usage: $0 [deb|linux|appimage]"
    exit 1
fi

echo "═══════════════════════════════════════════════════════════════"
echo "  SecuritySmith Build Verification"
echo "  Target: $TARGET"
echo "═══════════════════════════════════════════════════════════════"

# ─── 1. Build ─────────────────────────────────────────────────
echo ""
echo "🔨 1. Running Tauri build (this may take a few minutes)..."
echo ""
if (npx tauri build $BUNDLE_FLAG > /tmp/tauri-build.log 2>&1); then
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
BINARY_PATH=""
if [ -f "target/release/securitysmith" ]; then
    BINARY_PATH="target/release/securitysmith"
elif [ -f "src-tauri/target/release/securitysmith" ]; then
    BINARY_PATH="src-tauri/target/release/securitysmith"
fi
if [ -n "$BINARY_PATH" ]; then
    SIZE=$(du -sh "$BINARY_PATH" | cut -f1)
    check_ok "$BINARY_PATH   ($SIZE)"
else
    check_fail "Binary missing"
fi

# ─── 4. Verify installable bundles ────────────────────────────
echo ""
echo "📦 4. Installable bundles"
BUNDLES=0

# Prefer workspace-root target/ (Tauri v2 workspace builds)
DEB_PATH=""
APPIMAGE_PATH=""
RPM_PATH=""
if [ -f "target/release/bundle/deb/securitysmith_0.1.0_amd64.deb" ]; then
    DEB_PATH="target/release/bundle/deb/securitysmith_0.1.0_amd64.deb"
elif [ -f "src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb" ]; then
    DEB_PATH="src-tauri/target/release/bundle/deb/securitysmith_0.1.0_amd64.deb"
fi
if [ -f "target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage" ]; then
    APPIMAGE_PATH="target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage"
elif [ -f "src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage" ]; then
    APPIMAGE_PATH="src-tauri/target/release/bundle/appimage/securitysmith_0.1.0_amd64.AppImage"
fi
if [ -f "target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm" ]; then
    RPM_PATH="target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm"
elif [ -f "src-tauri/target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm" ]; then
    RPM_PATH="src-tauri/target/release/bundle/rpm/securitysmith-0.1.0-1.x86_64.rpm"
fi

if [ -n "$DEB_PATH" ]; then
    SIZE=$(du -sh "$DEB_PATH" | cut -f1)
    check_ok ".deb package ($SIZE)"
    BUNDLES=$((BUNDLES + 1))
else
    if [ "$TARGET" = "all" ] || [ "$TARGET" = "linux" ] || [ "$TARGET" = "deb" ]; then
        check_fail ".deb missing"
    fi
fi

if [ -n "$APPIMAGE_PATH" ]; then
    SIZE=$(du -sh "$APPIMAGE_PATH" | cut -f1)
    check_ok ".AppImage ($SIZE)"
    BUNDLES=$((BUNDLES + 1))
else
    if [ "$TARGET" = "all" ] || [ "$TARGET" = "linux" ] || [ "$TARGET" = "appimage" ]; then
        check_fail ".AppImage missing"
    fi
fi

# .rpm is only expected when building all bundles
if [ "$TARGET" = "all" ]; then
    if [ -n "$RPM_PATH" ]; then
        SIZE=$(du -sh "$RPM_PATH" | cut -f1)
        check_ok ".rpm package ($SIZE)"
        BUNDLES=$((BUNDLES + 1))
    else
        check_fail ".rpm missing"
    fi
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
