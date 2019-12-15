# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    # TODO Update this to build the artifacts that matter to you
    cross rustc --bin wireguard-proxy --target $TARGET --release -- -C lto

    # TODO Update this to package the right artifacts, this needs to handle .exe too...
    case $TARGET in
        x86_64-pc-windows-gnu)
            strip target/$TARGET/release/wireguard-proxy.exe || echo 'strip failed, ignoring...'
            cp target/$TARGET/release/wireguard-proxy.exe $stage/
            ;;
        *)
            strip target/$TARGET/release/wireguard-proxy || echo 'strip failed, ignoring...'
            cp target/$TARGET/release/wireguard-proxy $stage/
            ;;
    esac

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
    cd $src

    rm -rf $stage
}

main
