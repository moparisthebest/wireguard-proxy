use std::env;
use wireguard_proxy::{Args, ProxyClient, ProxyServer};

fn main() {
    let raw_args = env::args().collect();
    let args = Args::new(&raw_args);

    let default_udp_host_target = "127.0.0.1:51820";
    let default_socket_timeout = 0;

    let tcp_target = args.get_option(&["-tt", "--tcp-target"]);
    let tcp_host = args.get_option(&["-th", "--tcp-host"]);

    if args.flag("-h") || args.flag("--help") ||
        // one of them must be set
        (tcp_target.is_none() && tcp_host.is_none()) ||
        // but both cannot be set
        (tcp_target.is_some() && tcp_host.is_some())
        {
        println!(r#"usage: wireguard-proxy [options...]
 Client Mode (requires --tcp-target):
 -tt, --tcp-target <ip:port>     TCP target to send packets to, where
                                 wireguard-proxy server is running
 -uh, --udp-host <ip:port>       UDP host to listen on, point wireguard
                                 client here, default: {}
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
                                          default: {}
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
 -st, --socket-timeout <seconds> Socket timeout (time to wait for data)
                                 before terminating, default: {}

 Environment variable support:
 For every command line option, short and long, if you replace all
 leading - with WGP_, and replace all remaining - with _, and uppercase
 the whole thing, if you don't specify that command line option we will
 read that environment variable for the argument. boolean arguments are
 true if anything but unset, empty, 0, or false.
 Examples:
   --tcp-target ARG is WGP_TCP_TARGET=ARG
   --socket-timeout 5 is WGP_SOCKET_TIMEOUT=5
   --tls is WGP_TLS=1 or WGP_TLS=true
   WGP_TLS=0 or WGP_TLS=false would be like not sending --tls
        "#, default_udp_host_target, default_udp_host_target, default_socket_timeout);
        return;
    }

    let socket_timeout = args.get(&["-st", "--socket-timeout"], default_socket_timeout);

    if tcp_target.is_some() {
        client(&tcp_target.unwrap(), socket_timeout, args);
    } else {
        server(&tcp_host.unwrap(), socket_timeout, args);
    }
}

fn client(tcp_target: &str, socket_timeout: u64, args: Args) {
    let proxy_client = ProxyClient::new(
        args.get_str(&["-uh", "--udp-host"], "127.0.0.1:51820").to_owned(),
        tcp_target.to_owned(),
        socket_timeout,
    );

    let tls = args.flag("--tls");

    println!(
        "udp_host: {}, tcp_target: {}, socket_timeout: {:?}, tls: {}",
        proxy_client.udp_host,
        proxy_client.tcp_target,
        proxy_client.socket_timeout,
        tls,
    );

    if tls {
        let hostname = args.get_option(&["--tls-hostname"]).or_else(|| tcp_target.split(":").next().map(&str::to_owned));
        let pinnedpubkey = args.get_option(&["--pinnedpubkey"]);
        proxy_client.start_tls(hostname.as_ref().map(String::as_str), pinnedpubkey.as_ref().map(String::as_str)).expect("error running tls proxy_client");
    } else {
        proxy_client.start().expect("error running proxy_client");
    }
}

fn server(tcp_host: &str, socket_timeout: u64, args: Args) {
    let udp_bind_host_range_str = args.get_str(&["-ur", "--udp-bind-host-range"], "127.0.0.1:30000-40000");
    let mut udp_bind_host_range = udp_bind_host_range_str.split(":");
    let udp_host = udp_bind_host_range
        .next()
        .expect("udp_bind_host_range host invalid");
    let mut udp_ports = udp_bind_host_range
        .next()
        .expect("udp_bind_host_range port range invalid")
        .split("-");
    let udp_low_port = udp_ports
        .next()
        .expect("udp_bind_host_range low port invalid")
        .trim()
        .parse::<u16>()
        .expect("udp_bind_host_range low port invalid");
    let udp_high_port = udp_ports
        .next()
        .expect("udp_bind_host_range low port invalid")
        .trim()
        .parse::<u16>()
        .expect("udp_bind_host_range low port invalid");

    let proxy_server = ProxyServer::new(
        tcp_host.to_owned(),
        args.get_str(&["-ut", "--udp-target"], "127.0.0.1:51820").to_owned(),
        udp_host.to_string(),
        udp_low_port,
        udp_high_port,
        socket_timeout,
    );

    let tls_key = args.get_option(&["-tk", "--tls-key"]);
    let tls_cert = args.get_option(&["-tc", "--tls-cert"]);

    println!(
        "udp_target: {}, udp_bind_host_range: {}, socket_timeout: {:?}, tls_key: {:?}, tls_cert: {:?}",
        proxy_server.client_handler.udp_target,
        udp_bind_host_range_str,
        proxy_server.client_handler.socket_timeout,
        tls_key,
        tls_cert,
    );

    if tls_key.is_some() && tls_cert.is_some() {
        proxy_server.start_tls(&tls_key.unwrap(), &tls_cert.unwrap()).expect("error running TLS proxy_server");
    } else if tls_key.is_none() && tls_cert.is_none() {
        proxy_server.start().expect("error running proxy_server");
    } else {
        println!("Error: if one of --tls-key or --tls-cert is specified both must be!");
    }
}
