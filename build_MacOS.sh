#!/bin/bash

VERSION=$(git describe --tags --always | sed 's/\./_/g')
rm -rf dist
mkdir -p dist

yarn
cd packages/devtools
rm -rf build
yarn build
rm -rf build/windows
cd ../..

rm -rf target/release
RUSTFLAGS="-l framework=WebKit" cargo build --release

target/release/tauri_lite \
	--resource-dir=packages/devtools/build \
	--project=packages/devtools/build \
	--build=dist/TauriLiteDevTools.app

zip -r dist/TauriLiteDevTools_"$VERSION"_MacOS.zip dist/TauriLiteDevTools.app