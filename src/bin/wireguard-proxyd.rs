use std::env;
use wireguard_proxy::{Args, ProxyServer};

fn main() {
    let raw_args = env::args().collect();
    let args = Args::new(&raw_args);
    if args.get_str(1, "").contains("-h") {
        println!(
            "usage: {} [-h] [tcp_host, 127.0.0.1:5555] [udp_target, 127.0.0.1:51820] [udp_bind_host_range, 127.0.0.1:30000-40000] [socket_timeout, 0]",
            args.get_str(0, "wireguard-proxyd")
        );
        return;
    }

    let udp_bind_host_range_str = args.get_str(3, "127.0.0.1:30000-40000");
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
        args.get_str(1, "127.0.0.1:5555").to_owned(),
        args.get_str(2, "127.0.0.1:51820").to_owned(),
        udp_host.to_string(),
        udp_low_port,
        udp_high_port,
        args.get(4, 0),
    );

    println!(
        "udp_target: {}, udp_bind_host_range: {}, socket_timeout: {:?}",
        proxy_server.client_handler.udp_target,
        udp_bind_host_range_str,
        proxy_server.client_handler.socket_timeout,
    );

    proxy_server.start().expect("error running proxy_server");
}
