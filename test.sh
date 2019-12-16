#!/bin/sh
set -x

# always run this clean
cargo clean

# first make sure udp-test succeeds running against itself
cargo run --release --bin udp-test || exit 1

# now run udp-test without spawning other processes
cargo run --release --bin udp-test -- -is || exit 1

# now run proxyd pointing to udp-test
cargo run --release --bin wireguard-proxy -- -th 127.0.0.1:5555 -ut 127.0.0.1:51822 &
proxyd_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# proxy pointing to proxyd
#cargo run --release --bin wireguard-proxy -- -tt 127.0.0.1:5555 &

echo -e '\n\n\n\n\n\n\n' | openssl req -new -x509 -days 365 -nodes -out cert.pem -keyout cert.key
socat OPENSSL-LISTEN:5554,bind=127.0.0.1,cert=./cert.pem,key=./cert.key,verify=0 tcp4-connect:127.0.0.1:5555 &

cargo run --release --bin wireguard-proxy -- -tt 127.0.0.1:5554 --tls &

proxy_pid=$!
# wait for ports to be set up, this is fragile...
sleep 1
# and udp-test pointing to proxy, which then hops to proxyd, and finally back to udp-test
cargo run --release --bin udp-test -- -uh 127.0.0.1:51822
udp_exit=$?

kill $proxyd_pid $proxy_pid

rm -f cert.pem cert.key

[ $udp_exit -ne 0 ] && exit $udp_exit

# now run udp-test essentially just like the script above, but all in rust
cargo run --release --bin udp-test -- -s
