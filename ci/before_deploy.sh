# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd)

    test -f Cargo.lock || cargo generate-lockfile

    # TODO Update this to build the artifacts that matter to you
    cross rustc --bin wireguard-proxy --target $TARGET --release --no-default-features --features $CARGO_FEATURES

    # to check how they are built
    file target/$TARGET/release/wireguard-proxy* || echo 'file failed'

    # TODO Update this to package the right artifacts, this needs to handle .exe too...
    case $TARGET in
        x86_64-pc-windows-gnu)
            strip target/$TARGET/release/wireguard-proxy.exe || echo 'strip failed, ignoring...'
            cp target/$TARGET/release/wireguard-proxy.exe $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.exe
            ;;
        *)
            strip target/$TARGET/release/wireguard-proxy || echo 'strip failed, ignoring...'
            cp target/$TARGET/release/wireguard-proxy $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET
            ;;
    esac
}

main
