#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
hook="$repo_root/.githooks/commit-msg"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

good_message="$tmp_dir/good.txt"
bad_message="$tmp_dir/bad.txt"

printf 'fix(cli): handle docker stderr in text mode\n' >"$good_message"
printf 'Improve CLI error detail coverage\n' >"$bad_message"

"$hook" "$good_message"

if "$hook" "$bad_message"; then
  echo "commit-msg hook accepted an invalid subject" >&2
  exit 1
fi
