#!/usr/bin/env bash
set -euo pipefail

REPO="stolinski/csslint"
VERSION="latest"
INSTALL_DIR=""

usage() {
  cat <<'EOF'
Install csslint from GitHub release binaries.

Usage:
  install.sh [--version <tag>] [--install-dir <dir>] [--repo <owner/name>]

Options:
  --version <tag>      Release tag to install (default: latest stable release)
  --install-dir <dir>  Destination directory for binary
  --repo <owner/name>  GitHub repo (default: stolinski/csslint)
  -h, --help           Show this help

Examples:
  ./scripts/install.sh
  ./scripts/install.sh --version v0.1.0
  ./scripts/install.sh --install-dir "$HOME/.local/bin"
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="${2:-}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:-}"
      shift 2
      ;;
    --repo)
      REPO="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$INSTALL_DIR" ]]; then
  if [[ -w "/usr/local/bin" ]]; then
    INSTALL_DIR="/usr/local/bin"
  else
    INSTALL_DIR="$HOME/.local/bin"
  fi
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) OS_SLUG="linux" ;;
  Darwin) OS_SLUG="macos" ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_SLUG="x86_64" ;;
  arm64|aarch64) ARCH_SLUG="arm64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

if [[ "$OS_SLUG" == "linux" && "$ARCH_SLUG" != "x86_64" ]]; then
  echo "No prebuilt Linux binary for architecture: $ARCH_SLUG" >&2
  exit 1
fi

ASSETS=()
if [[ "$OS_SLUG" == "linux" ]]; then
  ASSETS+=("csslint-linux-x86_64.tar.gz")
elif [[ "$OS_SLUG" == "macos" ]]; then
  if [[ "$ARCH_SLUG" == "arm64" ]]; then
    ASSETS+=("csslint-macos-arm64.tar.gz" "csslint-macos-x86_64.tar.gz")
  else
    ASSETS+=("csslint-macos-x86_64.tar.gz")
  fi
fi

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

download() {
  local url="$1"
  local out="$2"
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$url" -o "$out"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$out" "$url"
  else
    echo "Need curl or wget to download artifacts" >&2
    exit 1
  fi
}

selected_asset=""
for candidate in "${ASSETS[@]}"; do
  if [[ "$VERSION" == "latest" ]]; then
    base_url="https://github.com/${REPO}/releases/latest/download/${candidate}"
  else
    base_url="https://github.com/${REPO}/releases/download/${VERSION}/${candidate}"
  fi

  if download "$base_url" "$TMP_DIR/$candidate"; then
    selected_asset="$candidate"
    if download "${base_url}.sha256" "$TMP_DIR/${candidate}.sha256"; then
      :
    else
      echo "Warning: checksum file not found for $candidate" >&2
      rm -f "$TMP_DIR/$candidate"
      selected_asset=""
      continue
    fi
    break
  fi
done

if [[ -z "$selected_asset" ]]; then
  echo "Could not download a matching release asset for ${OS_SLUG}/${ARCH_SLUG}" >&2
  if [[ "$VERSION" == "latest" ]]; then
    echo "Tip: this uses the latest stable release. Use --version for prerelease tags." >&2
  fi
  exit 1
fi

pushd "$TMP_DIR" >/dev/null
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c "${selected_asset}.sha256"
else
  expected="$(awk '{print $1}' "${selected_asset}.sha256")"
  actual="$(shasum -a 256 "$selected_asset" | awk '{print $1}')"
  if [[ "$expected" != "$actual" ]]; then
    echo "Checksum mismatch for $selected_asset" >&2
    exit 1
  fi
fi

tar -xzf "$selected_asset"
popd >/dev/null

mkdir -p "$INSTALL_DIR"

if command -v install >/dev/null 2>&1; then
  install -m 0755 "$TMP_DIR/csslint" "$INSTALL_DIR/csslint"
else
  cp "$TMP_DIR/csslint" "$INSTALL_DIR/csslint"
  chmod 0755 "$INSTALL_DIR/csslint"
fi

echo "Installed csslint to $INSTALL_DIR/csslint"
echo "Run: csslint --help"
