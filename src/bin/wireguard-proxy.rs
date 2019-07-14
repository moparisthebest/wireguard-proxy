use std::net::{TcpStream, UdpSocket};
use std::time::Duration;

use std::env;
use std::thread;
use wireguard_proxy::{Args, TcpUdpPipe};

struct Server {
    udp_host: String,
    udp_target: String,
    tcp_target: String,
    socket_timeout: Option<Duration>,
}

impl Server {
    fn new(udp_host: String, udp_target: String, tcp_target: String, secs: u64) -> Server {
        Server {
            udp_host,
            udp_target,
            tcp_target,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        }
    }

    fn start(&self) -> std::io::Result<usize> {
        let tcp_stream = TcpStream::connect(&self.tcp_target)?;

        tcp_stream.set_read_timeout(self.socket_timeout)?;

        let udp_socket = UdpSocket::bind(&self.udp_host)?;
        udp_socket.set_read_timeout(self.socket_timeout)?;
        //udp_socket.connect(&self.udp_target)?; // this isn't strictly needed...  just filters who we can receive from

        let mut udp_pipe = TcpUdpPipe::new(tcp_stream, udp_socket);
        let mut udp_pipe_clone = udp_pipe.try_clone()?;
        thread::spawn(move || loop {
            udp_pipe_clone
                .udp_to_tcp()
                .expect("cannot write to tcp_clone");
        });

        loop {
            udp_pipe.tcp_to_udp()?;
        }
    }
}

fn main() {
    let raw_args = env::args().collect();
    let args = Args::new(&raw_args);
    if args.get_str(1, "").contains("-h") {
        println!(
            "usage: {} [-h] [udp_host, 127.0.0.1:51821] [udp_target, 127.0.0.1:51820] [tcp_target, 127.0.0.1:5555] [socket_timeout, 0]",
            args.get_str(0, "wireguard-proxy")
        );
        return;
    }

    let server = Server::new(
        args.get_str(1, "127.0.0.1:51821").to_owned(),
        args.get_str(2, "127.0.0.1:51820").to_owned(),
        args.get_str(3, "127.0.0.1:5555").to_owned(),
        args.get(3, 0),
    );

    println!(
        "udp_host: {}, udp_target: {}, tcp_target: {}, socket_timeout: {:?}",
        server.udp_host, server.udp_target, server.tcp_target, server.socket_timeout,
    );

    server.start().expect("error running server");
}
