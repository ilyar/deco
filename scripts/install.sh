#!/usr/bin/env bash
set -euo pipefail

REPO="${DECO_INSTALL_REPO:-ilyar/deco}"
VERSION="${DECO_VERSION:-v1.0.0-alpha.1}"
INSTALL_DIR="${DECO_INSTALL_DIR:-$HOME/.local/bin}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

need_cmd curl
need_cmd tar
need_cmd uname
need_cmd install
need_cmd mktemp

usage() {
  cat <<'EOF'
Usage: install.sh [--version <tag>] [--install-dir <path>] [--repo <owner/name>]

Environment overrides:
  DECO_VERSION
  DECO_INSTALL_DIR
  DECO_INSTALL_REPO
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="$2"
      shift 2
      ;;
    --repo)
      REPO="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

verify_checksum() {
  local checksum_file="$1"
  local archive_file="$2"
  if command -v sha256sum >/dev/null 2>&1; then
    (
      cd "$TMP_DIR"
      sha256sum -c "$(basename "$checksum_file")"
    )
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    local expected actual
    expected="$(awk '{print $1}' "$checksum_file")"
    actual="$(shasum -a 256 "$archive_file" | awk '{print $1}')"
    if [[ "$expected" != "$actual" ]]; then
      echo "checksum verification failed for $(basename "$archive_file")" >&2
      exit 1
    fi
    return
  fi
  if command -v sha256 >/dev/null 2>&1; then
    local expected actual
    expected="$(awk '{print $1}' "$checksum_file")"
    actual="$(sha256 -q "$archive_file")"
    if [[ "$expected" != "$actual" ]]; then
      echo "checksum verification failed for $(basename "$archive_file")" >&2
      exit 1
    fi
    return
  fi
  echo "missing checksum tool: need sha256sum, shasum, or sha256" >&2
  exit 1
}

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux) os_target="unknown-linux-gnu" ;;
  Darwin) os_target="apple-darwin" ;;
  FreeBSD) os_target="unknown-freebsd" ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "Windows is not supported by the bash installer. Download the Windows zip from GitHub Releases." >&2
    exit 1
    ;;
  *)
    echo "unsupported operating system: $os" >&2
    exit 1
    ;;
esac

case "$arch" in
  x86_64|amd64) arch_target="x86_64" ;;
  arm64|aarch64) arch_target="aarch64" ;;
  *)
    echo "unsupported architecture: $arch" >&2
    exit 1
    ;;
esac

target="${arch_target}-${os_target}"
case "$target" in
  aarch64-unknown-linux-gnu)
    target="aarch64-unknown-linux-musl"
    ;;
  aarch64-apple-darwin|aarch64-unknown-linux-musl|x86_64-apple-darwin|x86_64-unknown-freebsd|x86_64-unknown-linux-gnu)
    ;;
  *)
    echo "no published binary for target: $target" >&2
    exit 1
    ;;
esac

archive="deco-${VERSION}-${target}.tar.gz"
checksum="${archive}.sha256"
base_url="https://github.com/${REPO}/releases/download/${VERSION}"

echo "Installing deco ${VERSION} for ${target} into ${INSTALL_DIR}" >&2

curl -fsSL "${base_url}/${archive}" -o "${TMP_DIR}/${archive}"
curl -fsSL "${base_url}/${checksum}" -o "${TMP_DIR}/${checksum}"

verify_checksum "${TMP_DIR}/${checksum}" "${TMP_DIR}/${archive}"

tar -xzf "${TMP_DIR}/${archive}" -C "$TMP_DIR"
mkdir -p "$INSTALL_DIR"
install "${TMP_DIR}/deco-${VERSION}-${target}/deco" "${INSTALL_DIR}/deco"

echo "Installed ${INSTALL_DIR}/deco" >&2
echo "Add ${INSTALL_DIR} to PATH if needed." >&2
