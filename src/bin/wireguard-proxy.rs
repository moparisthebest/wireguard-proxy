use std::env;
use wireguard_proxy::{Args, ProxyClient};

fn main() {
    let raw_args = env::args().collect();
    let args = Args::new(&raw_args);
    if args.get_str(1, "").contains("-h") {
        println!(
            "usage: {} [-h] [udp_host, 127.0.0.1:51820] [tcp_target, 127.0.0.1:5555] [socket_timeout, 0]",
            args.get_str(0, "wireguard-proxy")
        );
        return;
    }

    let proxy_client = ProxyClient::new(
        args.get_str(1, "127.0.0.1:51820").to_owned(),
        args.get_str(2, "127.0.0.1:5555").to_owned(),
        args.get(3, 0),
    );

    println!(
        "udp_host: {}, tcp_target: {}, socket_timeout: {:?}",
        proxy_client.udp_host,
        proxy_client.tcp_target,
        proxy_client.socket_timeout,
    );

    proxy_client.start().expect("error running proxy_client");
}
