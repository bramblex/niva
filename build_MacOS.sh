#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

VERSION="$(git describe --tags --always --dirty | sed 's/\./_/g')"
HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  arm64)
    HOST_TARGET="aarch64-apple-darwin"
    OTHER_TARGET="x86_64-apple-darwin"
    ;;
  x86_64)
    HOST_TARGET="x86_64-apple-darwin"
    OTHER_TARGET="aarch64-apple-darwin"
    ;;
  *) echo "Unsupported macOS build host: $HOST_ARCH" >&2; exit 1 ;;
esac

rm -rf dist
mkdir -p dist/runtime-artifacts

npm ci
npm run build --workspace=packages/runtime
npm run build --workspace=packages/devtools
# Windows helper assets are copied by Vite but do not belong in macOS apps.
rm -rf packages/devtools/build/windows

build_target() {
  local target="$1"
  shift
  RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0 \
    cargo build --locked --release --target="$target" -p niva "$@"
}

# Keep the packager native to this build host so it can assemble both app
# bundles without Rosetta or a cross-compiled helper executable.
build_target "$HOST_TARGET" -p niva-packager
build_target "$OTHER_TARGET"

stage_runtime() {
  local target="$1" name expected_arch binary description bytes
  case "$target" in
    aarch64-apple-darwin)
      name="niva-macos-aarch64"
      expected_arch="arm64"
      ;;
    x86_64-apple-darwin)
      name="niva-macos-x86_64"
      expected_arch="x86_64"
      ;;
    *) echo "Unsupported macOS target: $target" >&2; exit 1 ;;
  esac

  binary="target/$target/release/niva"
  strip -N "$binary"
  description="$(file -b "$binary")"
  if [[ "$description" != *"$expected_arch"* ]]; then
    echo "Wrong Mach-O architecture for $binary: $description" >&2
    exit 1
  fi
  bytes="$(stat -f%z "$binary")"
  echo "$target release runtime: $bytes bytes"
  if (( bytes >= 3300000 )); then
    echo "$target exceeds the 3,300,000 byte release ceiling" >&2
    exit 1
  fi
  cp -f "$binary" "dist/runtime-artifacts/$name"
}

stage_runtime "$HOST_TARGET"
stage_runtime "$OTHER_TARGET"

python3 scripts/create-packager-kit.py \
  --runtime-dir dist/runtime-artifacts \
  --packager "target/$HOST_TARGET/release/niva-packager" \
  --target macos-aarch64 \
  --target macos-x86_64 \
  --output dist/macos-packager-kit.zip

mkdir -p dist/kit-extracted dist/packages
unzip -q dist/macos-packager-kit.zip -d dist/kit-extracted
KIT_DIR="dist/kit-extracted/niva-build-kit"
"$KIT_DIR/niva-packager" build \
  --manifest "$KIT_DIR/manifest.json" \
  --config packages/devtools/niva.json \
  --resource-dir packages/devtools/build \
  --output-dir dist/packages \
  --target macos-aarch64 \
  --target macos-x86_64 \
  --resource-layout embedded

mv "dist/packages/NivaDevtools-macos-x86_64.zip" \
  "dist/NivaDevtools_${VERSION}_MacOS_x86_64.zip"
mv "dist/packages/NivaDevtools-macos-aarch64.zip" \
  "dist/NivaDevtools_${VERSION}_MacOS_aarch64.zip"
rm -rf dist/runtime-artifacts dist/macos-packager-kit.zip dist/kit-extracted dist/packages
