#!/bin/bash

set -eux

export PATH=$PATH:/builds/worker/clang/bin
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_NSS_STATIC=1
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_NSS_DIR=/builds/worker/checkouts/vcs/libs/desktop/darwin/nss
# x86_64 Darwin
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_CC=/builds/worker/clang/bin/clang
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_TOOLCHAIN_PREFIX=/builds/worker/cctools/bin
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_AR=/builds/worker/cctools/bin/x86_64-apple-darwin-ar
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_RANLIB=/builds/worker/cctools/bin/x86_64-apple-darwin-ranlib
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_LD_LIBRARY_PATH=/builds/worker/clang/lib
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_RUSTFLAGS="-C linker=/builds/worker/clang/bin/clang -C link-arg=-fuse-ld=/builds/worker/cctools/bin/x86_64-apple-darwin-ld -C link-arg=-B -C link-arg=/builds/worker/cctools/bin -C link-arg=-target -C link-arg=x86_64-apple-darwin -C link-arg=-isysroot -C link-arg=/tmp/MacOSX10.12.sdk -C link-arg=-Wl,-syslibroot,/tmp/MacOSX10.12.sdk -C link-arg=-Wl,-dead_strip"
# For ring's use of `cc`.
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_APPLE_DARWIN_CFLAGS_x86_64_apple_darwin="-B /builds/worker/cctools/bin -target x86_64-apple-darwin -isysroot /tmp/MacOSX10.12.sdk -Wl,-syslibroot,/tmp/MacOSX10.12.sdk -Wl,-dead_strip"

# x86_64 Windows
# The wrong linker gets used otherwise: https://github.com/rust-lang/rust/issues/33465.
export ORG_GRADLE_PROJECT_RUST_ANDROID_GRADLE_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc"

# Ensure we're compiling dependencies in non-debug mode.
# This is required for rkv/lmdb to work correctly on Android targets and not link to unavailable symbols.
export TARGET_CFLAGS="-DNDEBUG"

# Install clang, a port of cctools, and the macOS SDK into.
# cribbed from glean https://github.com/mozilla/glean/blob/b5faa56305a3a500e49fc02509001c95fde32ee6/taskcluster/scripts/cross-compile-setup.sh
pushd /builds/worker
curl -sfSL --retry 5 --retry-delay 10 \
    https://firefox-ci-tc.services.mozilla.com/api/index/v1/task/gecko.cache.level-3.toolchains.v3.linux64-cctools-port.hash.bec815dd1e64cd75ba803c1f002c646e151c784d804791bffbc23b2ae41f545f/artifacts/public/build/cctools.tar.zst > cctools.tar.zst
tar -I zstd -xf cctools.tar.zst
rm cctools.tar.zst
curl -sfSL --retry 5 --retry-delay 10 \
    https://firefox-ci-tc.services.mozilla.com/api/index/v1/task/gecko.cache.level-3.toolchains.v3.clang-dist-toolchain.hash.9e5dd392eefd9eb1ef3ff7aa40d2823f60b280cb54d716e0f85f0b50573e0d69/artifacts/public/build/clang-dist-toolchain.tar.xz > clang-dist-toolchain.tar.xz
tar -xf clang-dist-toolchain.tar.xz
mv builds/worker/toolchains/clang clang
rm clang-dist-toolchain.tar.xz
popd


pushd /tmp || exit

tooltool.py \
  --url=http://taskcluster/tooltool.mozilla-releng.net/ \
  --manifest="/builds/worker/checkouts/vcs/libs/macos-cc-tools.manifest" \
  fetch

popd || exit
