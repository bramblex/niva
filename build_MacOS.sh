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

target/release/niva \
	--resource-dir=packages/devtools/build \
	--project=packages/devtools/build \
	--build=dist/NivaDevTools.app

zip -r dist/NivaDevTools_"$VERSION"_MacOS.zip dist/NivaDevTools.app