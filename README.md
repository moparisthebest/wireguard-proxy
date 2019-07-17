# wireguard-proxy

Proxy wireguard UDP packets over TCP/TLS

`wireguard-proxyd` is a server-side daemon to accept TCP connections from multiple clients and pipe data to and from the specified UDP port  
`wireguard-proxy` is a client-side daemon that accepts UDP packets on a local port from a single client, connects to a single remote TCP port, and pipes data between them

Testing:

`udp-test` is a utility to send a UDP packet and then receive a UDP packet and ensure they are the same, this verifies packets sent through proxy/proxyd are unmolested  
`test.sh` runs udp-test against itself and then through proxyd/proxy  
`udp-test -s` runs udp-test against itself through proxyd/proxy by spawning actual binaries
`udp-test -is` runs udp-test against itself through proxyd/proxy in same executable by using library, so does not test command line parsing etc

Testing with GNU netcat:

- `nc -vulp 51820` listen on udp like wireguard would
- `nc -u -p 51821 127.0.0.1 51820` connect directly to local udp wireguard port to send data to 51820 from port 51821
- `nc -vlp 5555` listen on tcp like wireguard-proxy would
- `nc 127.0.0.1 5555` connect directly to local tcp wireguard-proxy port to send/recieve data
- so to test through wireguard-proxy run first and last command while it's running, type in both places

# License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in die by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
