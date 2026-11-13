#!/usr/bin/env bash
set -euo pipefail

repository_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repository_root"

archive=${1:-}
if [[ -z "$archive" ]]; then
  cargo package --allow-dirty
  archive=$(find target/package -maxdepth 1 -type f -name 'le-rust-*.crate' -print -quit)
fi
if [[ -z "$archive" || ! -f "$archive" ]]; then
  echo "Le Rust package archive was not found" >&2
  exit 1
fi

temporary_root=$(mktemp -d)
trap 'rm -rf "$temporary_root"' EXIT
mkdir -p "$temporary_root/package"
tar -xzf "$archive" -C "$temporary_root/package"
extracted=$(find "$temporary_root/package" -mindepth 1 -maxdepth 1 -type d -print -quit)
if [[ -z "$extracted" ]]; then
  echo "Le Rust package archive did not contain a crate directory" >&2
  exit 1
fi
mv "$extracted" "$temporary_root/package/le-rust-package"
cp -R consumer-tests "$temporary_root/consumer"
cargo run --quiet --manifest-path "$temporary_root/consumer/Cargo.toml"
