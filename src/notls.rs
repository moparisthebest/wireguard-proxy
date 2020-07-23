use std::net::TcpStream;
use super::super::TryClone;
use std::io::{Read, Write};
use crate::error::*;

fn err() -> Error {
    Error::new("Error: compiled without TLS support")
}

pub struct TlsStream;

impl TlsStream {
    pub fn client(_hostname: Option<&str>, _pinnedpubkey: Option<&str>, _tcp_stream: TcpStream) -> Result<TlsStream> {
        Err(err())
    }
}

impl TryClone<TlsStream> for TlsStream {
    fn try_clone(&self) -> Result<TlsStream> {
        Err(err())
    }
}

impl Read for TlsStream {
    fn read(&mut self, _buf: &mut [u8]) -> IoResult<usize> {
        unimplemented!()
    }
}

impl Write for TlsStream {
    fn write(&mut self, _buf: &[u8]) -> IoResult<usize> {
        unimplemented!()
    }

    fn flush(&mut self) -> IoResult<()> {
        unimplemented!()
    }
}

pub struct TlsListener;

impl TlsListener {
    pub fn new(_tls_key: &str, _tls_cert: &str) -> Result<TlsListener> {
        Err(err())
    }
    pub fn wrap(&self, _tcp_stream: TcpStream) -> Result<TlsStream> {
        Err(err())
    }
}