#!/usr/bin/env bash
set -euo pipefail

REPO="StringKe/claudex"
INSTALL_DIR="${CLAUDEX_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture
detect_target() {
    local os arch target

    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64)  target="x86_64-unknown-linux-gnu" ;;
                aarch64) target="aarch64-unknown-linux-gnu" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)  target="x86_64-apple-darwin" ;;
                arm64)   target="aarch64-apple-darwin" ;;
                *)       echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            echo "For Windows, download from: https://github.com/$REPO/releases" >&2
            exit 1
            ;;
    esac

    echo "$target"
}

# Get latest release tag
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

main() {
    local target version url tmpdir

    echo "Claudex Installer"
    echo "================="
    echo

    target="$(detect_target)"
    echo "Detected target: $target"

    version="$(get_latest_version)"
    if [ -z "$version" ]; then
        echo "Failed to determine latest version" >&2
        exit 1
    fi
    echo "Latest version: $version"

    url="https://github.com/$REPO/releases/download/$version/claudex-${version}-${target}.tar.gz"
    echo "Downloading: $url"

    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    curl -fsSL "$url" -o "$tmpdir/claudex.tar.gz"
    tar xzf "$tmpdir/claudex.tar.gz" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/claudex" "$INSTALL_DIR/claudex"
    chmod +x "$INSTALL_DIR/claudex"

    echo
    echo "Installed claudex to $INSTALL_DIR/claudex"

    if ! echo "$PATH" | tr ':' '\n' | grep -q "^$INSTALL_DIR$"; then
        echo
        echo "Add to PATH:"
        echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
    fi

    echo
    "$INSTALL_DIR/claudex" --version 2>/dev/null || true
}

main "$@"
