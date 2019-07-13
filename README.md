# wireguard-proxy

Server-side daemon to proxy multiple TCP connections to wireguard, client-side implementation coming here soon

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
