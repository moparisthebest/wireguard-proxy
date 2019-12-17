# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    cross build --target $TARGET --release --features $CARGO_FEATURES

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    # first make sure udp-test succeeds running against itself
    cross run --target $TARGET --release --features $CARGO_FEATURES --bin udp-test

    # now run udp-test through proxy/proxyd
    cross run --target $TARGET --release --features $CARGO_FEATURES --bin udp-test -- -is

    if [ $CARGO_FEATURES != "default" ]; then
        # run TLS tests then too
        cross run --target $TARGET --release --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem

        # now pubkey tests

        # one that should fail (wrong pinnedpubkey lowercase e at end instead of uppercase E)
        cross run --target $TARGET --release --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4de= && exit 1 || true

        # and one that should pass
        cross run --target $TARGET --release --features $CARGO_FEATURES --bin udp-test -- -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4dE=
    fi
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
