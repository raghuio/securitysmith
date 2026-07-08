#!/bin/sh
# build.sh — Build, install, and package the SecuritySmith CLI (sm)
#
# Usage:
#   ./scripts/build.sh              Build release binary and install to ~/.cargo/bin
#   ./scripts/build.sh --install     Same as above (explicit)
#   ./scripts/build.sh --system      Install to /usr/local/bin (needs sudo)
#   ./scripts/build.sh --deb         Create a .deb package (uses cargo-deb)
#   ./scripts/build.sh --rpm         Create a .rpm package (uses cargo-generate-rpm)
#   ./scripts/build.sh --tar          Create a .tar.gz archive
#   ./scripts/build.sh --man          Generate man pages and install
#   ./scripts/build.sh --all          Build all available package formats
#   ./scripts/build.sh --build        Build only, do not install or package
#
# This script wraps cargo's own tooling — it does not reimplement packaging.
#   install  → cargo install --path .
#   deb      → cargo deb            (auto-installs cargo-deb if missing)
#   rpm      → cargo generate-rpm  (auto-installs cargo-generate-rpm if missing)
#   tar      → tar czf             (no cargo equivalent exists)
#
set -eu

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
PROJECT_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
DIST_DIR="$PROJECT_DIR/dist"
BINARY_NAME="sm"

# ─── Helpers ─────────────────────────────────────────────────
log()  { printf "  -> %s\n" "$1"; }
ok()   { printf "  ✓  %s\n" "$1"; }
fail() { printf "  ✗  %s\n" "$1"; exit 1; }

ensure_subcmd() {
    # $1 = cargo subcommand name (e.g. cargo-deb), $2 = install crate name
    if ! cargo "$1" --help >/dev/null 2>&1; then
        log "Installing $1 (cargo install $2)..."
        cargo install "$2"
    fi
}

# ─── Build ───────────────────────────────────────────────────
build_binary() {
    log "Building release binary..."
    (cd "$PROJECT_DIR" && cargo build --release)
    BINARY="$PROJECT_DIR/target/release/$BINARY_NAME"
    [ -f "$BINARY" ] || fail "Binary not found at $BINARY"
    ok "Binary built: $BINARY"
}

# ─── Install ────────────────────────────────────────────────
install_user() {
    log "Installing to ~/.cargo/bin (cargo install --path crates/cli)..."
    (cd "$PROJECT_DIR" && cargo install --path crates/cli --force)
    ok "Installed: sm is in ~/.cargo/bin/"
    log "Verifying..."
    sm --version && ok "sm is runnable" || fail "sm not in PATH (add ~/.cargo/bin to PATH)"
}

install_system() {
    DEST="/usr/local/bin/$BINARY_NAME"
    log "Installing to $DEST..."
    SUDO_CMD="${SUDO:-sudo}"
    build_binary
    $SUDO_CMD cp "$BINARY" "$DEST"
    $SUDO_CMD chmod 755 "$DEST"
    ok "Installed: $DEST"
    "$DEST" --version && ok "sm is runnable" || fail "sm did not run"
}

# ─── .deb (cargo-deb) ───────────────────────────────────────
build_deb() {
    ensure_subcmd deb cargo-deb
    log "Creating .deb (cargo deb)..."
    (cd "$PROJECT_DIR" && cargo deb --output "$DIST_DIR/")
    ok ".deb package created in $DIST_DIR"
}

# ─── .rpm (cargo-generate-rpm) ──────────────────────────────
build_rpm() {
    ensure_subcmd generate-rpm cargo-generate-rpm
    log "Creating .rpm (cargo generate-rpm)..."
    (cd "$PROJECT_DIR" && cargo generate-rpm -o "$DIST_DIR/")
    ok ".rpm package created in $DIST_DIR"
}

# ─── .tar.gz (no cargo equivalent) ──────────────────────────
build_tar() {
    log "Creating .tar.gz archive..."
    MACHINE=$(uname -m)
    VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' "$PROJECT_DIR/Cargo.toml" | head -1)
    TAR_FILE="$DIST_DIR/securitysmith_${VERSION}_${MACHINE}.tar.gz"
    mkdir -p "$DIST_DIR/tar-build/usr/local/bin"
    cp "$BINARY" "$DIST_DIR/tar-build/usr/local/bin/$BINARY_NAME"
    chmod 755 "$DIST_DIR/tar-build/usr/local/bin/$BINARY_NAME"
    tar czf "$TAR_FILE" -C "$DIST_DIR/tar-build" .
    rm -rf "$DIST_DIR/tar-build"
    ok ".tar.gz archive: $TAR_FILE"
}

# --- Man pages (clap_mangen) ---
build_man() {
    log "Generating man pages (cargo run --bin gen-man)..."
    (cd "$PROJECT_DIR" && cargo run --bin gen-man --release -- --output "$DIST_DIR/sm.1")
    ok "Man page: $DIST_DIR/sm.1"
    log "Installing man page to $PREFIX/share/man/man1/sm.1..."
    MAN_DIR="$PREFIX/share/man/man1"
    if mkdir -p "$MAN_DIR" 2>/dev/null; then
        cp "$DIST_DIR/sm.1" "$MAN_DIR/sm.1"
    else
        SUDO_CMD="${SUDO:-sudo}"
        $SUDO_CMD mkdir -p "$MAN_DIR"
        $SUDO_CMD cp "$DIST_DIR/sm.1" "$MAN_DIR/sm.1"
    fi
    ok "Installed: $MAN_DIR/sm.1"
}

# ─── Main ───────────────────────────────────────────────────
mkdir -p "$DIST_DIR"

TARGET="${1:---install}"

case "$TARGET" in
    --install|"")   build_binary; install_user ;;
    --system)        install_system ;;
    --build)        build_binary; ok "Build complete (no install)" ;;
    --deb)          build_binary; build_deb ;;
    --rpm)          build_binary; build_rpm ;;
    --tar)          build_binary; build_tar ;;
    --man)          build_man ;;
    --all)
        build_binary
        build_man
        build_deb 2>/dev/null || log "skipped .deb (cargo-deb failed)"
        build_rpm 2>/dev/null || log "skipped .rpm (cargo-generate-rpm failed)"
        build_tar
        ;;
    *)
        printf "Usage: %s [--install|--system|--build|--deb|--rpm|--tar|--man|--all]\n" "$0"
        exit 1
        ;;
esac

ok "Done."