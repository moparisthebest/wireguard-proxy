#!/bin/sh
set -x

# cert created with:
# cd ci && echo -e '\n\n\n\n\n\n\n' | openssl req -new -x509 -days 3650 -nodes -out cert.pem -keyout cert.key

export PATH="$(pwd)/target/release:$PATH"

run_tests() {
client_arg="$1"
shift

# first make sure udp-test succeeds running against itself
udp-test || exit 1

# now run udp-test without spawning other processes
udp-test -is "$@" || exit 1

# now run proxyd pointing to udp-test
wireguard-proxy -th 127.0.0.1:5555 -ut 127.0.0.1:51822 "$@" &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 5
# proxy pointing to proxyd
wireguard-proxy -tt 127.0.0.1:5555 "$client_arg" &
proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# and udp-test pointing to proxy, which then hops to proxyd, and finally back to udp-test
udp-test -uh 127.0.0.1:51822
udp_exit=$?

kill $proxyd_pid $proxy_pid

[ $udp_exit -ne 0 ] && exit $udp_exit

# now run udp-test essentially just like the script above, but all in rust
udp-test -s "$@" || exit 1

}

# first run without TLS
cargo clean
cargo build --release --no-default-features || exit 1
run_tests || exit 1

# first run with non-vendored tls
cargo clean
cargo build --release --no-default-features --features tls || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

# second run with vendored tls
cargo clean
cargo build --release --no-default-features --features openssl_vendored || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

# third run with async+rustls
cargo clean
cargo build --release --no-default-features --features async || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

# now pubkey tests

# these should fail
udp-test -s --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4de= && exit 1
udp-test -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4de= && exit 1

# these should pass
udp-test -s --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4dE= || exit 1
udp-test -is --tls-key ci/cert.key --tls-cert ci/cert.pem --pinnedpubkey sha256//BEyQeSjwwUBLXXNuCILHRWyV1gLmY31CdMHNA4VH4dE= || exit 1

exit 0
