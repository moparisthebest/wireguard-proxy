# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    cross build --target $TARGET
    cross build --target $TARGET --release

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    # first make sure udp-test succeeds running against itself
    cross run --target $TARGET --release --bin udp-test

    # now run udp-test through proxy/proxyd
    cross run --target $TARGET --release --bin udp-test -- -is
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
