#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

# Builds PyOxy executables.

set -ex

TARGET_TRIPLE=$1
PYTHON_VERSION=$2
TARGET_DIR="$(pwd)/$3"

source ~/.cargo/env

# Use PyOxidizer to generate embeddable files.
pyoxidizer build \
  --system-rust \
  --release \
  --path pyoxy \
  --target-triple ${TARGET_TRIPLE} \
  --var BUILD_PATH ${TARGET_DIR} \
  --var PYTHON_VERSION ${PYTHON_VERSION}

# Use PyOxidizer's embeddable files to build the pyoxy binary. Its
# build script will hook things up to the pyembed crate.
export PYO3_CONFIG_FILE=${TARGET_DIR}/${TARGET_TRIPLE}/release/resources/pyo3-build-config-file.txt

# Emulate `-Z linker-features=+lld`
GCC_LD="$(rustc --print sysroot)/lib/rustlib/$(rustc --print sysroot | sed -Ee 's/.*\/[^\-]+-//')/bin/gcc-ld"

~/.cargo/bin/cargo build \
  --target-dir ${TARGET_DIR} \
  --bin pyoxy \
  --release \
  --config "build.rustflags = [\"-C\", '''link-arg=-B${GCC_LD}''', \"-C\", \"link-arg=-fuse-ld=lld\"]" \
  --target ${TARGET_TRIPLE}
