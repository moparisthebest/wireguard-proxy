#!/bin/sh
set -x

# first make sure udp-test succeeds running against itself
cargo run --release --bin udp-test || exit 1

# now run udp-test without spawning other processes
cargo run --release --bin udp-test -- -is || exit 1

# now run udp-test essentially just like the script below, but all in rust
cargo run --release --bin udp-test -- -s || exit 1

# now run proxyd pointing to udp-test
cargo run --release --bin wireguard-proxyd -- 127.0.0.1:5555 127.0.0.1:51822 &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# proxy pointing to proxyd
cargo run --release --bin wireguard-proxy &
proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# and udp-test pointing to proxy, which then hops to proxyd, and finally back to udp-test
cargo run --release --bin udp-test -- 127.0.0.1:51822
udp_exit=$?

kill $proxyd_pid $proxy_pid

exit $udp_exit
