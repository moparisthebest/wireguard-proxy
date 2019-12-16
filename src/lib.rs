use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::cell::{UnsafeCell};
use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode};

pub struct Args<'a> {
    args: &'a Vec<String>,
}

impl<'a> Args<'a> {
    pub fn new(args: &'a Vec<String>) -> Args {
        Args { args }
    }
    pub fn flag(&self, flag: &'a str) -> bool {
        self.args.contains(&flag.to_owned())
    }
    pub fn get_option(&self, flags: &[&'a str]) -> Option<&'a str> {
        for flag in flags.iter() {
            let mut found = false;
            for arg in self.args.iter() {
                if found {
                    return Some(arg);
                }
                if arg == flag {
                    found = true;
                }
            }
        }
        return None;
    }
    pub fn get_str(&self, flags: &[&'a str], def: &'a str) -> &'a str {
        match self.get_option(flags) {
            Some(ret) => ret,
            None => def,
        }
    }
    pub fn get<T: FromStr>(&self, flags: &[&'a str], def: T) -> T {
        match self.get_option(flags) {
            Some(ret) => match ret.parse::<T>() {
                Ok(ret) => ret,
                Err(_) => def, // or panic
            },
            None => def,
        }
    }
    pub fn get_str_idx(&self, index: usize, def: &'a str) -> &'a str {
        match self.args.get(index) {
            Some(ret) => ret,
            None => def,
        }
    }
}

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

    pub fn try_clone(&self) -> std::io::Result<TcpUdpPipe<T>> {
        Ok(TcpUdpPipe::new(
            self.tcp_stream.try_clone()?,
            self.udp_socket.try_clone()?,
        ))
    }

    pub fn shuffle_after_first_udp(&mut self) -> std::io::Result<usize> {
        let (len, src_addr) = self.udp_socket.recv_from(&mut self.buf[2..])?;

        println!("first packet from {}, connecting to that", src_addr);
        self.udp_socket.connect(src_addr)?;

        self.send_udp(len)?;

        self.shuffle()
    }

    pub fn udp_to_tcp(&mut self) -> std::io::Result<()> {
        let len = self.udp_socket.recv(&mut self.buf[2..])?;
        self.send_udp(len)
    }

    fn send_udp(&mut self, len: usize) -> std::io::Result<()> {
        println!("udp got len: {}", len);

        self.buf[0] = ((len >> 8) & 0xFF) as u8;
        self.buf[1] = (len & 0xFF) as u8;

        //let test_len = ((self.buf[0] as usize) << 8) + self.buf[1] as usize;
        //println!("tcp sending test_len: {}", test_len);

        self.tcp_stream.write_all(&self.buf[..len + 2])
        // todo: do this? self.tcp_stream.flush()
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

    pub fn shuffle(&mut self) -> std::io::Result<usize> {
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

pub struct ProxyClient {
    pub udp_host: String,
    pub tcp_target: String,
    pub socket_timeout: Option<Duration>,
}

impl ProxyClient {
    pub fn new(udp_host: String, tcp_target: String, secs: u64) -> ProxyClient {
        ProxyClient {
            udp_host,
            tcp_target,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        }
    }

    fn tcp_connect(&self) -> std::io::Result<TcpStream> {
        let tcp_stream = TcpStream::connect(&self.tcp_target)?;
        tcp_stream.set_read_timeout(self.socket_timeout)?;
        Ok(tcp_stream)
    }

    fn udp_connect(&self) -> std::io::Result<UdpSocket> {
        let udp_socket = UdpSocket::bind(&self.udp_host)?;
        udp_socket.set_read_timeout(self.socket_timeout)?;
        Ok(udp_socket)
    }

    pub fn start(&self) -> std::io::Result<usize> {
        let tcp_stream = self.tcp_connect()?;

        let udp_socket = self.udp_connect()?;

        // we want to wait for first udp packet from client first, to set the target to respond to
        TcpUdpPipe::new(tcp_stream, udp_socket).shuffle_after_first_udp()
    }

    pub fn start_tls(&self) -> std::io::Result<usize> {
        let tcp_stream = self.tcp_connect()?;

        let mut connector = SslConnector::builder(SslMethod::tls()).unwrap().build().configure().unwrap();
        connector.set_verify_hostname(false);
        connector.set_verify(SslVerifyMode::NONE);
        let tcp_stream = connector.connect(self.tcp_target.split(":").next().unwrap(), tcp_stream).unwrap();
        let tcp_stream = OpensslCell { sess: Arc::new(UnsafeCell::new(tcp_stream)) };

        let udp_socket = self.udp_connect()?;

        // we want to wait for first udp packet from client first, to set the target to respond to
        TcpUdpPipe::new(tcp_stream, udp_socket).shuffle_after_first_udp()
    }
}


pub trait TryClone<T> {
    fn try_clone(&self) -> std::io::Result<T>;
}

impl TryClone<UdpSocket> for UdpSocket {
    fn try_clone(&self) -> std::io::Result<UdpSocket> {
        self.try_clone()
    }
}

impl TryClone<TcpStream> for TcpStream {
    fn try_clone(&self) -> std::io::Result<TcpStream> {
        self.try_clone()
    }
}

impl TryClone<OpensslCell> for OpensslCell {
    fn try_clone(&self) -> std::io::Result<OpensslCell> {
        Ok(self.clone())
    }
}

pub struct OpensslCell {
    sess: Arc<UnsafeCell<SslStream<TcpStream>>>,
}

unsafe impl Sync for OpensslCell {}
unsafe impl Send for OpensslCell {}

impl Clone for OpensslCell {
    fn clone(&self) -> Self {
        OpensslCell {
            sess: self.sess.clone(),
        }
    }
}

impl OpensslCell {
    pub fn borrow(&self) -> &SslStream<TcpStream> {
        unsafe {
            &*self.sess.get()
        }
    }
    pub fn borrow_mut(&self) -> &mut SslStream<TcpStream> {
        unsafe {
            &mut *self.sess.get()
        }
    }
}

impl Read for OpensslCell {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.borrow_mut().read(buf)
    }
}

impl Write for OpensslCell {
    fn write(&mut self, buf: &[u8]) -> Result<usize, std::io::Error> {
        self.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> Result<(), std::io::Error> {
        self.borrow_mut().flush()
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

        TcpUdpPipe::new(tcp_stream, udp_socket).shuffle()
    }
}
