#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 3 || $# -gt 4 ]]; then
  echo "usage: $0 <version> <target> <binary-path> [output-dir]" >&2
  exit 2
fi

version="$1"
target="$2"
binary_path="$3"
output_dir="${4:-dist}"

if [[ ! -f "$binary_path" ]]; then
  echo "binary not found: $binary_path" >&2
  exit 1
fi

compute_sha256() {
  local path="$1"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$path" | awk '{print $1}'
    return
  fi
  if command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$path" | awk '{print $1}'
    return
  fi
  if command -v sha256 >/dev/null 2>&1; then
    sha256 -q "$path"
    return
  fi
  echo "no sha256 tool available" >&2
  exit 1
}

package_name="deco-v${version}-${target}"
staging_dir="${output_dir}/${package_name}"
archive_path="${output_dir}/${package_name}.tar.gz"
checksum_path="${archive_path}.sha256"
binary_name="$(basename "$binary_path")"

rm -rf "$staging_dir"
mkdir -p "$staging_dir"
cp "$binary_path" "${staging_dir}/${binary_name}"
cp LICENSE "${staging_dir}/LICENSE"
cp README.md "${staging_dir}/README.md"

tar -C "$output_dir" -czf "$archive_path" "$package_name"
checksum="$(compute_sha256 "$archive_path")"
printf '%s  %s\n' "$checksum" "$(basename "$archive_path")" > "$checksum_path"

printf 'archive=%s\nchecksum=%s\n' "$archive_path" "$checksum_path"
