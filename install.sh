#!/bin/sh
# Claudex installer - works on bash, zsh, sh, dash
# Supports: macOS (Intel/Apple Silicon), Linux (x86_64/aarch64), Ubuntu, CentOS, Alpine
set -e

REPO="StringKe/claudex"
INSTALL_DIR="${CLAUDEX_INSTALL_DIR:-$HOME/.local/bin}"

# Detect OS and architecture
detect_target() {
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            # Detect musl vs glibc
            libc="gnu"
            if command -v ldd >/dev/null 2>&1; then
                case "$(ldd --version 2>&1 || true)" in
                    *musl*) libc="musl" ;;
                esac
            elif [ -f /etc/alpine-release ]; then
                libc="musl"
            fi

            case "$arch" in
                x86_64|amd64)   echo "x86_64-unknown-linux-${libc}" ;;
                aarch64|arm64)  echo "aarch64-unknown-linux-${libc}" ;;
                *)              echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64)         echo "x86_64-apple-darwin" ;;
                arm64|aarch64)  echo "aarch64-apple-darwin" ;;
                *)              echo "Unsupported architecture: $arch" >&2; exit 1 ;;
            esac
            ;;
        *)
            echo "Unsupported OS: $os" >&2
            echo "For Windows, download from: https://github.com/$REPO/releases" >&2
            exit 1
            ;;
    esac
}

# Check required commands
check_deps() {
    for cmd in curl tar uname; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            echo "Error: '$cmd' is required but not found." >&2
            echo "Install it with your package manager (apt, yum, brew, apk, etc.)" >&2
            exit 1
        fi
    done
}

# Get latest release tag
get_latest_version() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/'
}

main() {
    echo "Claudex Installer"
    echo "================="
    echo

    check_deps

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

    if ! curl -fsSL "$url" -o "$tmpdir/claudex.tar.gz"; then
        echo "" >&2
        echo "Download failed. This target may not have a pre-built binary." >&2
        echo "Try building from source: cargo install --git https://github.com/$REPO" >&2
        exit 1
    fi

    tar xzf "$tmpdir/claudex.tar.gz" -C "$tmpdir"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/claudex" "$INSTALL_DIR/claudex"
    chmod +x "$INSTALL_DIR/claudex"

    echo
    echo "Installed claudex to $INSTALL_DIR/claudex"

    # Check if INSTALL_DIR is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*) ;;
        *)
            echo
            echo "Add to PATH (add to your shell rc file):"
            echo "  export PATH=\"$INSTALL_DIR:\$PATH\""
            ;;
    esac

    echo
    "$INSTALL_DIR/claudex" --version 2>/dev/null || true
}

main "$@"
