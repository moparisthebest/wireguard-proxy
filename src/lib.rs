use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::str::FromStr;

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
