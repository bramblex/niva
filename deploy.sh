#!/bin/bash

GIT_HASH=$(git rev-parse HEAD)

yarn build
rm -rf /tmp/niva-gh-pages
git clone --depth 1 --branch gh-pages https://github.com/bramblex/niva.git /tmp/niva-gh-pages

find /tmp/niva-gh-pages ! -path "*/.git/*" -type f -delete
cp -r build/* build/.* /tmp/niva-gh-pages

cd /tmp/niva-gh-pages
git add -A
git commit -a -m "Deploy website - based on ${GIT_HASH}"

git push -f
rm -rf /tmp/niva-ght-pages