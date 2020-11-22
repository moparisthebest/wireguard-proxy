#!/bin/sh
#set -x

# cert created with:
# cd ci && echo -e '\n\n\n\n\n\n\n' | openssl req -new -x509 -days 3650 -nodes -out cert.pem -keyout cert.key

export PATH="$(pwd)/target/release:$PATH"

run_tests() {
client_arg="$1"
shift

# now run proxyd pointing to nc
wireguard-proxy -th 127.0.0.1:5555 -ut 127.0.0.1:51822 "$@" &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 5
# proxy pointing to proxyd
wireguard-proxy -tt 127.0.0.1:5555 "$client_arg" &
proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1

# nc running through wireguard-proxy's above
nc -lup 51822 >/dev/null &
nc_listen_pid=$!

wireguard-proxy -V

dd if=/dev/zero bs=128M count=10 | nc -u 127.0.0.1 51820 &
nc_connect_pid=$!

sleep 5

kill $nc_listen_pid $nc_connect_pid $proxyd_pid $proxy_pid

}


# first no-network baseline
dd if=/dev/zero bs=128M count=10 | cat >/dev/null

# now openbsd netcat for network baseline
nc -lup 51822 >/dev/null &
nc_listen_pid=$!

dd if=/dev/zero bs=128M count=10 | nc -u 127.0.0.1 51822 &
nc_connect_pid=$!

sleep 5

kill $nc_listen_pid $nc_connect_pid

# first run without TLS
#cargo clean
cargo build --release --no-default-features 2>/dev/null || exit 1
run_tests || exit 1

# third run with async+rustls
#cargo clean
cargo build --release --no-default-features --features async 2>/dev/null || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

exit 0

# first run with non-vendored tls
#cargo clean
cargo build --release --no-default-features --features tls 2>/dev/null || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

# second run with vendored tls
#cargo clean
cargo build --release --no-default-features --features openssl_vendored 2>/dev/null || exit 1
# first plaintext tests
run_tests || exit 1
# then TLS tests
run_tests --tls --tls-key ci/cert.key --tls-cert ci/cert.pem || exit 1

exit 0
