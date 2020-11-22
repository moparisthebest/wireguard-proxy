#!/bin/bash
set -exo pipefail

echo "starting build for TARGET $TARGET"

export CRATE_NAME=wireguard-proxy
export OPENSSL_STATIC=1
export CARGO_FEATURES=async

DISABLE_TESTS=${DISABLE_TESTS:-0}

SUFFIX=""

# wine blows up in testing with async build
echo "$TARGET" | grep -E '^x86_64-pc-windows-gnu$' >/dev/null && DISABLE_TESTS=1 && SUFFIX=".exe"

# these only support openssl_vendored, not async
if echo "$TARGET" | grep -E '^(s390x|powerpc|mips)' >/dev/null
then
    CARGO_FEATURES=openssl_vendored
fi

# these don't support any TLS at all
if echo "$TARGET" | grep -E '(^riscv64gc|solaris$)' >/dev/null
then
    CARGO_FEATURES=verbose
fi

cross rustc --bin wireguard-proxy --target $TARGET --release --no-default-features --features $CARGO_FEATURES
cross rustc --bin udp-test --target $TARGET --release --no-default-features --features $CARGO_FEATURES

# to check how they are built
file "target/$TARGET/release/wireguard-proxy$SUFFIX" "target/$TARGET/release/udp-test$SUFFIX"

if [ $DISABLE_TESTS -ne 1 ]
then

    # first make sure udp-test succeeds running against itself
    cross run --target $TARGET --release --no-default-features --features $CARGO_FEATURES --bin udp-test

    # now run udp-test through proxy/proxyd
    cross run --target $TARGET --release --no-default-features --features $CARGO_FEATURES --bin udp-test -- -is

    if [ $CARGO_FEATURES != "verbose" ]; then
        # run TLS tests then too
        cross run --target $TARGET --release --no-default-features --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem

        # now pubkey tests

        # one that should fail (wrong pinnedpubkey lowercase e at end instead of uppercase E)
        cross run --target $TARGET --release --no-default-features --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4de= && exit 1 || true

        # and one that should pass
        cross run --target $TARGET --release --no-default-features --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4dE=
    fi
fi

# if this commit has a tag, upload artifact to release
strip "target/$TARGET/release/wireguard-proxy$SUFFIX" || true # if strip fails, it's fine
mkdir -p release
mv "target/$TARGET/release/wireguard-proxy$SUFFIX" "release/wireguard-proxy-$TARGET$SUFFIX"

echo 'build success!'
exit 0
