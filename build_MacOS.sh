#!/usr/bin/env bash
set -euo pipefail

VERSION="$(git describe --tags --always --dirty | sed 's/\./_/g')"
HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  arm64) HOST_TARGET="aarch64-apple-darwin" ;;
  x86_64) HOST_TARGET="x86_64-apple-darwin" ;;
  *) echo "Unsupported macOS build host: $HOST_ARCH" >&2; exit 1 ;;
esac

rm -rf dist
mkdir -p dist/x86_64

npm ci
npm run build --workspace=packages/devtools
# Windows helpers in public/ are copied by Vite but do not belong in macOS apps.
rm -rf packages/devtools/build/windows

RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0 \
  cargo build --release --target=x86_64-apple-darwin -p niva
RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0 \
  cargo build --release --target=aarch64-apple-darwin -p niva

"target/$HOST_TARGET/release/niva" \
  --debug-resource=packages/devtools/build \
  --debug-config=packages/devtools/niva.json \
  --project=packages/devtools \
  --build=dist/x86_64/NivaDevtools.app

cp -f target/x86_64-apple-darwin/release/niva \
  dist/x86_64/NivaDevtools.app/Contents/MacOS/NivaDevtools
cp -R dist/x86_64 dist/aarch64
cp -f target/aarch64-apple-darwin/release/niva \
  dist/aarch64/NivaDevtools.app/Contents/MacOS/NivaDevtools

verify_release_binary() {
  local binary="$1" arch="$2" description bytes
  description="$(file -b "$binary")"
  if [[ "$description" != *"$arch"* ]]; then
    echo "Wrong Mach-O architecture for $binary: $description" >&2
    exit 1
  fi
  bytes="$(stat -f%z "$binary")"
  echo "$arch release binary: $bytes bytes"
  if (( bytes >= 3300000 )); then
    echo "$arch exceeds the 3,300,000 byte release ceiling" >&2
    exit 1
  fi
  if (( bytes >= 3000000 )); then
    echo "$arch exceeds the 3,000,000 byte reference target but remains within the accepted release ceiling" >&2
  fi
}

verify_release_binary dist/x86_64/NivaDevtools.app/Contents/MacOS/NivaDevtools x86_64
verify_release_binary dist/aarch64/NivaDevtools.app/Contents/MacOS/NivaDevtools arm64

(cd dist/x86_64 && zip -qr "../NivaDevtools_${VERSION}_MacOS_x86_64.zip" NivaDevtools.app)
(cd dist/aarch64 && zip -qr "../NivaDevtools_${VERSION}_MacOS_aarch64.zip" NivaDevtools.app)
