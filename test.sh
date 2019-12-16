#!/bin/sh
set -x

# first run without TLS
cargo clean
cargo build --release --no-default-features

export PATH="$(pwd)/target/release:$PATH"

# first make sure udp-test succeeds running against itself
udp-test || exit 1

# now run udp-test without spawning other processes
udp-test -is || exit 1

# now run proxyd pointing to udp-test
wireguard-proxy -th 127.0.0.1:5555 -ut 127.0.0.1:51822 &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 5
# proxy pointing to proxyd
wireguard-proxy -tt 127.0.0.1:5555 &
proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# and udp-test pointing to proxy, which then hops to proxyd, and finally back to udp-test
udp-test -uh 127.0.0.1:51822
udp_exit=$?

kill $proxyd_pid $proxy_pid

[ $udp_exit -ne 0 ] && exit $udp_exit

# now run udp-test essentially just like the script above, but all in rust
udp-test -s || exit 1

echo "non-tls tests passed!"

echo -e '\n\n\n\n\n\n\n' | openssl req -new -x509 -days 365 -nodes -out cert.pem -keyout cert.key

# first run without TLS
cargo clean
cargo build --release

export PATH="$(pwd)/target/release:$PATH"

# first make sure udp-test succeeds running against itself
udp-test || exit 1

# now run udp-test without spawning other processes
udp-test -is || exit 1

# now run proxyd pointing to udp-test
wireguard-proxy -th 127.0.0.1:5555 -ut 127.0.0.1:51822 --tls-key cert.key --tls-cert cert.pem &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 5
# proxy pointing to proxyd
wireguard-proxy -tt 127.0.0.1:5555 --tls &
proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# and udp-test pointing to proxy, which then hops to proxyd, and finally back to udp-test
udp-test -uh 127.0.0.1:51822
udp_exit=$?

kill $proxyd_pid $proxy_pid

rm -f cert.pem cert.key

[ $udp_exit -ne 0 ] && exit $udp_exit

# now run udp-test essentially just like the script above, but all in rust
udp-test -s

exit $?
