#!/usr/bin/env bash
set -euo pipefail

# Install toolchain and helpers
pkg install \
  developer/gnu-binutils \
  developer/build/gnu-make \
  developer/pkg-config \
  archiver/gnu-tar \
  runtime/python-313 \
  git

# Build/Install Node.js 24.11.1
cd /tmp
curl -O https://nodejs.org/dist/v24.11.1/node-v24.11.1.tar.gz
gtar xf node-v24.11.1.tar.gz
cd node-v24.11.1

./configure --prefix=/opt/ooce/nodejs-24.11.1 --dest-os=solaris --dest-cpu=x64 --openssl-use-def-ca-store
gmake -j"$(psrinfo -p)"
gmake install

# Verify the Node.js version:
node -v

# Verify npm version:
npm -v