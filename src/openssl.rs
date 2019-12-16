
use openssl::ssl::{SslConnector, SslMethod, SslStream, SslVerifyMode, SslAcceptor, SslFiletype, HandshakeError};
use std::sync::Arc;
use std::cell::UnsafeCell;
use std::net::TcpStream;
use crate::TryClone;
use std::io::{Read, Write};

use crate::error::*;
use std::error::Error as StdError;

impl TryClone<TlsStream> for TlsStream {
    fn try_clone(&self) -> Result<TlsStream> {
        Ok(self.clone())
    }
}

pub struct TlsStream {
    sess: Arc<UnsafeCell<SslStream<TcpStream>>>,
}

impl TlsStream {
    fn new(stream: SslStream<TcpStream>) -> TlsStream {
        TlsStream {
            sess: Arc::new(UnsafeCell::new(stream))
        }
    }
    pub fn client(host_name: &str, tcp_stream: TcpStream) -> Result<TlsStream> {
        let mut connector = SslConnector::builder(SslMethod::tls())?.build().configure()?;
        connector.set_verify_hostname(false);
        connector.set_verify(SslVerifyMode::NONE);
        let tcp_stream = connector.connect(host_name, tcp_stream)?;
        Ok(TlsStream::new(tcp_stream))
    }
}

unsafe impl Sync for TlsStream {}
unsafe impl Send for TlsStream {}

impl Clone for TlsStream {
    fn clone(&self) -> Self {
        TlsStream {
            sess: self.sess.clone(),
        }
    }
}

impl TlsStream {
    pub fn borrow_mut(&self) -> &mut SslStream<TcpStream> {
        unsafe {
            &mut *self.sess.get()
        }
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.borrow_mut().read(buf)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.borrow_mut().write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.borrow_mut().flush()
    }
}

pub struct TlsListener {
    acceptor: SslAcceptor,
}

impl TlsListener {
    pub fn new(tls_key: &str, tls_cert: &str) -> Result<TlsListener> {
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
        acceptor.set_private_key_file(tls_key, SslFiletype::PEM)?;
        acceptor.set_certificate_chain_file(tls_cert)?;
        acceptor.check_private_key()?;
        let acceptor = acceptor.build();
        Ok(TlsListener {
            acceptor
        })
    }
    pub fn wrap(&self, tcp_stream: TcpStream) -> Result<TlsStream> {
        Ok(TlsStream::new(self.acceptor.accept(tcp_stream)?))
    }
}

impl From<openssl::error::ErrorStack> for Error {
    fn from(value: openssl::error::ErrorStack) -> Self {
        Error::new(value.description())
    }
}

impl From<HandshakeError<std::net::TcpStream>> for Error {
    fn from(value: HandshakeError<std::net::TcpStream>) -> Self {
        Error::new(value.description())
    }
}
