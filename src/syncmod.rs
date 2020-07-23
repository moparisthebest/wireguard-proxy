use std::thread;
use crate::error::Result;
use crate::*;

use std::net::TcpListener;
use std::io::{Write, Read};

#[cfg(any(feature = "tls", feature = "openssl_vendored"))]
#[path = ""]
mod tls {
    pub mod openssl;
    pub use super::tls::openssl::{TlsStream, TlsListener};
}

#[cfg(not(any(feature = "tls", feature = "openssl_vendored")))]
#[path = ""]
mod tls {
    pub mod notls;
    pub use super::tls::notls::{TlsStream, TlsListener};
}

use tls::{TlsStream, TlsListener};

pub struct TcpUdpPipe<T: Write + Read + TryClone<T> + Send + 'static> {
    buf: [u8; 2050], // 2048 + 2 for len
    tcp_stream: T,
    udp_socket: UdpSocket,
}

impl<T: Write + Read + TryClone<T> + Send + 'static> TcpUdpPipe<T> {
    pub fn new(tcp_stream: T, udp_socket: UdpSocket) -> TcpUdpPipe<T> {
        TcpUdpPipe {
            tcp_stream,
            udp_socket,
            buf: [0u8; 2050],
        }
    }

    pub fn try_clone(&self) -> Result<TcpUdpPipe<T>> {
        Ok(TcpUdpPipe::new(
            self.tcp_stream.try_clone()?,
            self.udp_socket.try_clone()?,
        ))
    }

    pub fn shuffle_after_first_udp(&mut self) -> Result<usize> {
        let (len, src_addr) = self.udp_socket.recv_from(&mut self.buf[2..])?;

        println!("first packet from {}, connecting to that", src_addr);
        self.udp_socket.connect(src_addr)?;

        self.send_udp(len)?;

        self.shuffle()
    }

    pub fn udp_to_tcp(&mut self) -> Result<()> {
        let len = self.udp_socket.recv(&mut self.buf[2..])?;
        self.send_udp(len)
    }

    fn send_udp(&mut self, len: usize) -> Result<()> {
        #[cfg(feature = "verbose")]
        println!("udp got len: {}", len);

        self.buf[0] = ((len >> 8) & 0xFF) as u8;
        self.buf[1] = (len & 0xFF) as u8;

        Ok(self.tcp_stream.write_all(&self.buf[..len + 2])?)
        // todo: do this? self.tcp_stream.flush()
    }

    pub fn tcp_to_udp(&mut self) -> Result<usize> {
        self.tcp_stream.read_exact(&mut self.buf[..2])?;
        let len = ((self.buf[0] as usize) << 8) + self.buf[1] as usize;
        #[cfg(feature = "verbose")]
        println!("tcp expecting len: {}", len);
        self.tcp_stream.read_exact(&mut self.buf[..len])?;
        #[cfg(feature = "verbose")]
        println!("tcp got len: {}", len);
        Ok(self.udp_socket.send(&self.buf[..len])?)

        //let sent = udp_socket.send_to(&buf[..len], &self.udp_target)?;
        //assert_eq!(sent, len);
    }

    pub fn shuffle(&mut self) -> Result<usize> {
        let mut udp_pipe_clone = self.try_clone()?;
        thread::spawn(move || loop {
            udp_pipe_clone
                .udp_to_tcp()
                .expect("cannot write to tcp_clone");
        });

        loop {
            self.tcp_to_udp()?;
        }
    }
}

pub trait TryClone<T> {
    fn try_clone(&self) -> Result<T>;
}

impl TryClone<UdpSocket> for UdpSocket {
    fn try_clone(&self) -> Result<UdpSocket> {
        Ok(self.try_clone()?)
    }
}

impl TryClone<TcpStream> for TcpStream {
    fn try_clone(&self) -> Result<TcpStream> {
        Ok(self.try_clone()?)
    }
}

impl ProxyClient {

    pub fn start(&self) -> Result<usize> {
        let tcp_stream = self.tcp_connect()?;

        let udp_socket = self.udp_connect()?;

        // we want to wait for first udp packet from client first, to set the target to respond to
        TcpUdpPipe::new(tcp_stream, udp_socket).shuffle_after_first_udp()
    }

    pub fn start_tls(&self, hostname: Option<&str>, pinnedpubkey: Option<&str>) -> Result<usize> {
        let tcp_stream = self.tcp_connect()?;

        let tcp_stream = TlsStream::client(hostname, pinnedpubkey, tcp_stream)?;

        let udp_socket = self.udp_connect()?;

        // we want to wait for first udp packet from client first, to set the target to respond to
        TcpUdpPipe::new(tcp_stream, udp_socket).shuffle_after_first_udp()
    }
}

impl ProxyServer {

    pub fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.tcp_host)?;
        println!("Listening for connections on {}", &self.tcp_host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client_handler = self.client_handler.clone();
                    client_handler.set_tcp_options(&stream).expect("cannot set tcp options");

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

    pub fn start_tls(&self, tls_key: &str, tls_cert: &str) -> Result<()> {
        let tls_listener = Arc::new(TlsListener::new(tls_key, tls_cert)?);

        let listener = TcpListener::bind(&self.tcp_host)?;
        println!("Listening for TLS connections on {}", &self.tcp_host);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let client_handler = self.client_handler.clone();
                    client_handler.set_tcp_options(&stream).expect("cannot set tcp options");

                    let tls_listener = tls_listener.clone();
                    thread::spawn(move || {
                        let stream = tls_listener.wrap(stream).expect("cannot wrap with tls");
                        client_handler
                            .handle_client_tls(stream)
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

impl ProxyServerClientHandler {

    pub fn set_tcp_options(&self, tcp_stream: &TcpStream) -> Result<()> {
        Ok(tcp_stream.set_read_timeout(self.socket_timeout)?)
    }

    pub fn handle_client(&self, tcp_stream: TcpStream) -> Result<usize> {
        TcpUdpPipe::new(tcp_stream, self.udp_bind()?).shuffle()
    }

    pub fn handle_client_tls(&self, tcp_stream: TlsStream) -> Result<usize> {
        TcpUdpPipe::new(tcp_stream, self.udp_bind()?).shuffle()
    }
}