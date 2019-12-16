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
                                 WARNING: currently verifies nothing!

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

 Common Options:
 -h, --help                      print this usage text
 -st, --socket-timeout <seconds> Socket timeout (time to wait for data)
                                 before terminating, default: {}
        "#, default_udp_host_target, default_udp_host_target, default_socket_timeout);
        return;
    }

    let socket_timeout = args.get(&["-st", "--socket-timeout"], default_socket_timeout);

    if tcp_target.is_some() {
        client(tcp_target.unwrap(), socket_timeout, args);
    } else {
        server(tcp_host.unwrap(), socket_timeout, args);
    }
}

fn client(tcp_target: &str, socket_timeout: u64, args: Args) {
    let proxy_client = ProxyClient::new(
        args.get_str(&["-uh", "--udp-host"], "127.0.0.1:51820").to_owned(),
        tcp_target.to_owned(),
        socket_timeout,
    );

    println!(
        "udp_host: {}, tcp_target: {}, socket_timeout: {:?}",
        proxy_client.udp_host,
        proxy_client.tcp_target,
        proxy_client.socket_timeout,
    );

    if args.flag("--tls") {
        proxy_client.start_tls().expect("error running tls proxy_client");
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

    println!(
        "udp_target: {}, udp_bind_host_range: {}, socket_timeout: {:?}",
        proxy_server.client_handler.udp_target,
        udp_bind_host_range_str,
        proxy_server.client_handler.socket_timeout,
    );

    proxy_server.start().expect("error running proxy_server");
}
