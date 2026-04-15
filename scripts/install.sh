#!/usr/bin/env bash
set -euo pipefail

REPO="stolinski/csslint"
VERSION="latest"
INSTALL_DIR=""

usage() {
  cat <<'EOF'
Install clint from GitHub release binaries.

Usage:
  install.sh [--version <tag>] [--install-dir <dir>] [--repo <owner/name>]

Options:
  --version <tag>      Release tag to install (default: latest stable release, with prerelease fallback)
  --install-dir <dir>  Destination directory for binary
  --repo <owner/name>  GitHub repo (default: stolinski/csslint)
  -h, --help           Show this help

Examples:
  ./scripts/install.sh
  ./scripts/install.sh --version v0.1.0
  ./scripts/install.sh --install-dir "$HOME/.local/bin"
EOF
}

path_contains_dir() {
  local dir="$1"
  case ":${PATH:-}:" in
    *":${dir}:"*|*":${dir}/:"*) return 0 ;;
    *) return 1 ;;
  esac
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
  ASSETS+=("clint-linux-x86_64.tar.gz")
elif [[ "$OS_SLUG" == "macos" ]]; then
  if [[ "$ARCH_SLUG" == "arm64" ]]; then
    ASSETS+=("clint-macos-arm64.tar.gz" "clint-macos-x86_64.tar.gz")
  else
    ASSETS+=("clint-macos-x86_64.tar.gz")
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
selected_version="$VERSION"
fallback_tag=""

resolve_newest_release_tag() {
  local releases_url="https://api.github.com/repos/${REPO}/releases?per_page=1"
  local releases_file="$TMP_DIR/releases.json"
  if ! download "$releases_url" "$releases_file" 2>/dev/null; then
    return 1
  fi

  awk -F '"' '/"tag_name"[[:space:]]*:/ { print $4; exit }' "$releases_file"
}

attempt_download_for_version() {
  local version="$1"
  selected_asset=""

  for candidate in "${ASSETS[@]}"; do
    local base_url
    if [[ "$version" == "latest" ]]; then
      base_url="https://github.com/${REPO}/releases/latest/download/${candidate}"
    else
      base_url="https://github.com/${REPO}/releases/download/${version}/${candidate}"
    fi

    if download "$base_url" "$TMP_DIR/$candidate" 2>/dev/null; then
      selected_asset="$candidate"
      if download "${base_url}.sha256" "$TMP_DIR/${candidate}.sha256" 2>/dev/null; then
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

  [[ -n "$selected_asset" ]]
}

if ! attempt_download_for_version "$selected_version"; then
  if [[ "$VERSION" == "latest" ]]; then
    fallback_tag="$(resolve_newest_release_tag || true)"
    if [[ -n "$fallback_tag" ]]; then
      echo "Latest stable release does not include ${OS_SLUG}/${ARCH_SLUG}; trying newest tag ${fallback_tag}" >&2
      selected_version="$fallback_tag"
      attempt_download_for_version "$selected_version" || true
    fi
  fi
fi

if [[ -z "$selected_asset" ]]; then
  echo "Could not download a matching release asset for ${OS_SLUG}/${ARCH_SLUG}" >&2
  echo "Tip: pass --version <tag> to install an explicit release." >&2
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
  install -m 0755 "$TMP_DIR/clint" "$INSTALL_DIR/clint"
else
  cp "$TMP_DIR/clint" "$INSTALL_DIR/clint"
  chmod 0755 "$INSTALL_DIR/clint"
fi

echo "Installed clint to $INSTALL_DIR/clint"

if path_contains_dir "$INSTALL_DIR"; then
  echo "Run: clint --help"
else
  echo "Note: $INSTALL_DIR is not on your PATH."
  echo "Add this to your shell profile and restart your shell:"
  if [[ "$INSTALL_DIR" == "$HOME/.local/bin" ]]; then
    echo '  export PATH="$HOME/.local/bin:$PATH"'
  else
    printf '  export PATH="%s:$PATH"\n' "$INSTALL_DIR"
  fi
  echo "Then run: clint --help"
  echo "For now, run directly: $INSTALL_DIR/clint --help"
fi
