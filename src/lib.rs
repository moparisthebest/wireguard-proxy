use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct Args<'a> {
    args: &'a Vec<String>,
}

impl<'a> Args<'a> {
    pub fn new(args: &'a Vec<String>) -> Args {
        Args { args }
    }
    pub fn get_str(&self, index: usize, def: &'a str) -> &'a str {
        match self.args.get(index) {
            Some(ret) => ret,
            None => def,
        }
    }
    pub fn get<T: FromStr>(&self, index: usize, def: T) -> T {
        match self.args.get(index) {
            Some(ret) => match ret.parse::<T>() {
                Ok(ret) => ret,
                Err(_) => def, // or panic
            },
            None => def,
        }
    }
}

pub struct TcpUdpPipe {
    buf: [u8; 2050], // 2048 + 2 for len
    tcp_stream: TcpStream,
    udp_socket: UdpSocket,
}

impl TcpUdpPipe {
    pub fn new(tcp_stream: TcpStream, udp_socket: UdpSocket) -> TcpUdpPipe {
        TcpUdpPipe {
            tcp_stream,
            udp_socket,
            buf: [0u8; 2050],
        }
    }

    pub fn try_clone(&self) -> std::io::Result<TcpUdpPipe> {
        Ok(TcpUdpPipe::new(
            self.tcp_stream.try_clone()?,
            self.udp_socket.try_clone()?,
        ))
    }

    pub fn udp_to_tcp(&mut self) -> std::io::Result<usize> {
        let len = self.udp_socket.recv(&mut self.buf[2..])?;
        println!("udp got len: {}", len);

        self.buf[0] = ((len >> 8) & 0xFF) as u8;
        self.buf[1] = (len & 0xFF) as u8;

        //let test_len = ((self.buf[0] as usize) << 8) + self.buf[1] as usize;
        //println!("tcp sending test_len: {}", test_len);

        self.tcp_stream.write(&self.buf[..len + 2])
    }

    pub fn tcp_to_udp(&mut self) -> std::io::Result<usize> {
        self.tcp_stream.read_exact(&mut self.buf[..2])?;
        let len = ((self.buf[0] as usize) << 8) + self.buf[1] as usize;
        println!("tcp expecting len: {}", len);
        self.tcp_stream.read_exact(&mut self.buf[..len])?;
        println!("tcp got len: {}", len);
        self.udp_socket.send(&self.buf[..len])

        //let sent = udp_socket.send_to(&buf[..len], &self.udp_target)?;
        //assert_eq!(sent, len);
    }
}

pub struct ProxyClient {
    pub udp_host: String,
    pub udp_target: String,
    pub tcp_target: String,
    pub socket_timeout: Option<Duration>,
}

impl ProxyClient {
    pub fn new(udp_host: String, udp_target: String, tcp_target: String, secs: u64) -> ProxyClient {
        ProxyClient {
            udp_host,
            udp_target,
            tcp_target,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        }
    }

    pub fn start(&self) -> std::io::Result<usize> {
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

pub struct ProxyServer {
    pub tcp_host: String,
    pub client_handler: Arc<ProxyServerClientHandler>,
}

impl ProxyServer {
    pub fn new(
        tcp_host: String,
        udp_target: String,
        udp_host: String,
        udp_low_port: u16,
        udp_high_port: u16,
        secs: u64,
    ) -> ProxyServer {
        let client_handler = Arc::new(ProxyServerClientHandler {
            udp_target,
            udp_host,
            udp_low_port,
            udp_high_port,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        });
        ProxyServer {
            tcp_host,
            client_handler,
        }
    }

    pub fn start(&self) -> std::io::Result<()> {
        let listener = TcpListener::bind(&self.tcp_host)?;
        println!("Listening for connections on {}", &self.tcp_host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client_handler = self.client_handler.clone();
                    thread::spawn(move || {
                        client_handler
                            .handle_client(stream)
                            .expect("error handling connection")
                    });
                }
                Err(e) => {
                    println!("Unable to connect: {}", e);
                }
            }
        }
        Ok(())
    }
}

pub struct ProxyServerClientHandler {
    pub udp_target: String,
    pub udp_host: String,
    pub udp_low_port: u16,
    pub udp_high_port: u16,
    pub socket_timeout: Option<Duration>,
}

impl ProxyServerClientHandler {
    pub fn handle_client(&self, tcp_stream: TcpStream) -> std::io::Result<usize> {
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
        udp_socket.connect(&self.udp_target)?;

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
