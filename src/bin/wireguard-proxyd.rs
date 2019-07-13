use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::time::Duration;

use std::env;
use std::sync::Arc;
use std::thread;
use wireguard_proxy::Args;

struct Server {
    udp_target: String,
    udp_host: String,
    udp_low_port: u16,
    udp_high_port: u16,
    socket_timeout: Option<Duration>,
}

impl Server {
    fn new(
        udp_target: String,
        udp_host: String,
        udp_low_port: u16,
        udp_high_port: u16,
        secs: u64,
    ) -> Server {
        Server {
            udp_target,
            udp_host,
            udp_low_port,
            udp_high_port,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        }
    }

    fn handle_client(&self, mut tcp_stream: TcpStream) -> std::io::Result<usize> {
        tcp_stream.set_read_timeout(self.socket_timeout)?;

        let mut port = self.udp_low_port;
        let udp_socket = loop {
            match UdpSocket::bind((&self.udp_host[..], port)) {
                Ok(sock) => break sock,
                Err(_) => {
                    port += 1;
                    if port > self.udp_high_port {
                        panic!("cannot find free port, increase range?");
                    }
                }
            }
        };
        udp_socket.set_read_timeout(self.socket_timeout)?;
        //udp_socket.connect(&self.udp_target)?;

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
                    //udp_socket.send(&buf[..len])?;
                    let sent = udp_socket.send_to(&buf[..len], &self.udp_target)?;
                    println!("udp sent len: {}", sent);
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
            "usage: {} [-h] [tcp_host, 127.0.0.1:5555] [udp_target, 127.0.0.1:51820] [udp_bind_host_range, 127.0.0.1:30000-40000] [socket_timeout, 0]",
            args.get_str(0, "wireguard-proxyd")
        );
        return;
    }
    let host = args.get_str(1, "127.0.0.1:5555");

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

    let server = Arc::new(Server::new(
        args.get_str(2, "127.0.0.1:51820").to_owned(),
        udp_host.to_string(),
        udp_low_port,
        udp_high_port,
        args.get(4, 0),
    ));

    println!(
        "udp_target: {}, udp_bind_host_range: {}, socket_timeout: {:?}",
        server.udp_target, udp_bind_host_range_str, server.socket_timeout,
    );

    let listener = TcpListener::bind(&host).unwrap();
    println!("Listening for connections on {}", &host);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let server = server.clone();
                thread::spawn(move || {
                    server
                        .handle_client(stream)
                        .expect("error handling connection")
                });
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}
