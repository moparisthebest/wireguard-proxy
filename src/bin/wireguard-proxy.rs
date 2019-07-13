use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::time::Duration;

use std::env;
use std::thread;
use wireguard_proxy::Args;

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
        let mut tcp_stream = TcpStream::connect(&self.tcp_target)?;

        tcp_stream.set_read_timeout(self.socket_timeout)?;

        let udp_socket = UdpSocket::bind(&self.udp_host)?;
        udp_socket.set_read_timeout(self.socket_timeout)?;
        udp_socket.connect(&self.udp_target)?;

        let udp_socket_clone = udp_socket.try_clone().expect("clone udp_socket failed");
        let mut tcp_stream_clone = tcp_stream.try_clone().expect("clone tcp_stream failed");
        thread::spawn(move || {
            let mut buf = [0u8; 2048];
            loop {
                match udp_socket_clone.recv(&mut buf) {
                    Ok(len) => {
                        println!("udp got len: {}", len);
                        tcp_stream_clone
                            .write(&buf[..len])
                            .expect("cannot write to tcp_clone");
                    }
                    Err(e) => {
                        println!("recv function failed: {:?}", e);
                        break;
                    }
                }
            }
        });

        let mut buf = [0u8; 2048];
        loop {
            match tcp_stream.read(&mut buf) {
                Ok(len) => {
                    println!("tcp got len: {}", len);
                    udp_socket.send(&buf[..len])?;
                }
                Err(e) => {
                    println!("Unable to read stream: {}", e);
                    break;
                }
            }
        }

        Ok(0)
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
