# wireguard-proxy

[![Travis-CI Build Status](https://api.travis-ci.org/moparisthebest/wireguard-proxy.svg?branch=master)](https://travis-ci.org/moparisthebest/wireguard-proxy)
[![Build status](https://ci.appveyor.com/api/projects/status/vl8c9xdhvgn997d2/branch/master?svg=true)](https://ci.appveyor.com/project/moparisthebest/wireguard-proxy)
[![crates.io](https://img.shields.io/crates/v/wireguard-proxy.svg)](https://crates.io/crates/wireguard-proxy)

Proxy wireguard UDP packets over TCP/TLS

`wireguard-proxy` has 2 modes:
- server-side daemon to accept TCP/TLS connections from multiple clients and pipe data to and from the specified UDP port
- client-side daemon that accepts UDP packets on a local port from a single client, connects to a single remote TCP/TLS port, and pipes data between them

```
$ wireguard-proxy -h
usage: wireguard-proxy [options...]
 Client Mode (requires --tcp-target):
 -tt, --tcp-target <ip:port>     TCP target to send packets to, where
                                 wireguard-proxy server is running
 -uh, --udp-host <ip:port>       UDP host to listen on, point wireguard
                                 client here, default: 127.0.0.1:51820
 --tls                           use TLS when connecting to tcp-target
                                 WARNING: authenticates/verifies nothing
                                 without --pinnedpubkey below!!
 --pinnedpubkey <sha256_hashes>  Public key to verify peer against,
                                 format is any number of base64 encoded
                                 sha256 hashes preceded by "sha256//"
                                 and separated by ";". Identical to curl's
                                 --pinnedpubkey and CURLOPT_PINNEDPUBLICKEY
 --tls-hostname                  send this in SNI instead of host
                                 from --tcp-target, useful for avoiding
                                 DNS lookup on connect

 Server Mode (requires --tcp-host):
 -th, --tcp-host <ip:port>                TCP host to listen on
 -ut, --udp-target <ip:port>              UDP target to send packets to, where
                                          wireguard server is running,
                                          default: 127.0.0.1:51820
 -ur, --udp-bind-host-range <ip:low-high> UDP host and port range to bind to,
                                          one port per TCP connection, to
                                          listen on for UDP packets to send
                                          back over the TCP connection,
                                          default: 127.0.0.1:30000-40000
 -tk, --tls-key <ip:port>                 TLS key to listen with,
                                          requires --tls-cert also
 -tc, --tls-cert <ip:port>                TLS cert to listen with,
                                          requires --tls-key also
 Note: with both --tls-key and --tls-cert,
       - means stdin,
       also the same file can work for both if you combine them into
       one pem file

 Common Options:
 -h, --help                      print this usage text
 -V, --version                   Show version number and TLS support then quit
 -st, --socket-timeout <seconds> Socket timeout (time to wait for data)
                                 before terminating, default: 0

 Environment variable support:
 For every long command line option (starting with --), if you replace the
 leading -- with WGP_, and replace all remaining - with _, and uppercase
 the whole thing, if you don't specify that command line option we will
 read that environment variable for the argument. boolean arguments are
 true if anything but unset, empty, 0, or false.
 Examples:
   --tcp-target ARG is WGP_TCP_TARGET=ARG
   --socket-timeout 5 is WGP_SOCKET_TIMEOUT=5
   --tls is WGP_TLS=1 or WGP_TLS=true
   WGP_TLS=0 or WGP_TLS=false would be like not sending --tls
```

Binaries:

- [releases](https://github.com/moparisthebest/wireguard-proxy/releases) has static builds for most platforms performed by travis-ci and appveyor courtesy of [trust](https://github.com/japaric/trust)
- Arch Linux AUR [wireguard-proxy](https://aur.archlinux.org/packages/wireguard-proxy/) and [wireguard-proxy-git](https://aur.archlinux.org/packages/wireguard-proxy-git/)

Building:

- `cargo build --release` - minimal build without TLS support, no dependencies
- `cargo build --release --feature tls` - links to system openssl
- `cargo build --release --feature openssl_vendored` - compiles vendored openssl and link to it

Testing:

- `udp-test` is a utility to send a UDP packet and then receive a UDP packet and ensure they are the same, this verifies packets sent through proxy server/client are unmolested  
- `udp-test -s` runs udp-test against itself through proxy server/client by spawning actual binaries
- `udp-test -is` runs udp-test against itself through proxy server/client in same executable by using library, so does not test command line parsing etc
- `test.sh` runs udp-test against itself, the udp-test self tests above, and through proxy server/client in the shell script

Testing with GNU netcat:

- `nc -vulp 51820` listen on udp like wireguard would
- `nc -u -p 51821 127.0.0.1 51820` connect directly to local udp wireguard port to send data to 51820 from port 51821
- `nc -vlp 5555` listen on tcp like wireguard-proxy would
- `nc 127.0.0.1 5555` connect directly to local tcp wireguard-proxy port to send/recieve data
- so to test through wireguard-proxy run first and last command while it's running, type in both places

# OpenSSL cert generation

Quick commands to generate your own certificate to use with wireguard-proxy, note if you are actually only sending
wireguard packets over this, the TLS layer doesn't really need to provide any security or authentication, only obfuscation

Currently the only authentication performed is optional and via --pinnedpubkey only if supplied

```sh
# single command self signed RSA cert
openssl req -new -x509 -sha256 -days 3650 -nodes -subj "/C=US/CN=example.org" -newkey rsa:2048 -out cert.pem -keyout key.pem

# customize key type
# more info: https://github.com/openssl/openssl/blob/master/doc/man1/openssl-genpkey.pod
# ordered roughly starting from oldest/worst/most supported (rsa) to newest/best/least supported (ed448) order
# run one of these only to generate the preferred key type
openssl genpkey -algorithm RSA -out key.pem -pkeyopt rsa_keygen_bits:1024
openssl genpkey -algorithm RSA -out key.pem -pkeyopt rsa_keygen_bits:2048
openssl genpkey -algorithm RSA -out key.pem -pkeyopt rsa_keygen_bits:4096
openssl genpkey -algorithm EC -out key.pem -pkeyopt ec_paramgen_curve:P-256 -pkeyopt ec_param_enc:named_curve
openssl genpkey -algorithm EC -out key.pem -pkeyopt ec_paramgen_curve:P-384 -pkeyopt ec_param_enc:named_curve
openssl genpkey -algorithm EC -out key.pem -pkeyopt ec_paramgen_curve:P-521 -pkeyopt ec_param_enc:named_curve
openssl genpkey -algorithm ED25519 -out key.pem
openssl genpkey -algorithm ED448 -out key.pem

# then run this to generate and self-sign a cert with the above key
openssl req -new -x509 -sha256 -days 3650 -nodes -subj "/C=US/CN=example.org" -out cert.pem -key key.pem

# optionally (but recommended) extract pinnedpubkey hash from the above generated cert like so:
# openssl x509 -in cert.pem -pubkey -noout | openssl pkey -pubin -outform der | openssl dgst -sha256 -binary | openssl enc -base64

# optionally run this to see human readable info about the cert
openssl x509 -in cert.pem -noout -text
```

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
