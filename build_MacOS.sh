#!/bin/bash

VERSION=$(git describe --tags --always | sed 's/\./_/g')
rm -rf dist
mkdir -p dist/x86_64

yarn
cd packages/devtools
rm -rf build
yarn build
rm -rf build/windows
cd ../..

rm -rf target/release

RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release --target=x86_64-apple-darwin
RUSTFLAGS="-l framework=WebKit" MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release --target=aarch64-apple-darwin

target/x86_64-apple-darwin/release/niva \
	--debug-resource=packages/devtools/build \
	--debug-config=packages/devtools/niva.json \
	--project=packages/devtools \
	--build=dist/x86_64/NivaDevtools.app

cp -r dist/x86_64 dist/aarch64
cp -f target/aarch64-apple-darwin/release/niva dist/aarch64/NivaDevtools.app/Contents/MacOS/NivaDevtools

zip -r dist/NivaDevtools_"$VERSION"_MacOS_x86_64.zip dist/x86_64/NivaDevtools.app
zip -r dist/NivaDevtools_"$VERSION"_MacOS_aarch64.zip dist/aarch64/NivaDevtools.app