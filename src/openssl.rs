
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
    pub fn client(hostname: Option<&str>, pinnedpubkey: Option<&str>, tcp_stream: TcpStream) -> Result<TlsStream> {
        let mut connector = SslConnector::builder(SslMethod::tls())?.build().configure()?;
        connector.set_use_server_name_indication(hostname.is_some());
        connector.set_verify_hostname(false);
        connector.set_verify(SslVerifyMode::NONE);
        if pinnedpubkey.is_some() {
            let pinnedpubkey = pinnedpubkey.unwrap().to_owned();
            connector.set_verify_callback(SslVerifyMode::PEER, move|_preverify_ok, x509_store_ctx| {
                //println!("preverify_ok: {}", preverify_ok);
                let cert = x509_store_ctx.current_cert().expect("could not get TLS cert");
                let pubkey = cert.public_key().expect("could not get public key from TLS cert");
                let pubkey = pubkey.public_key_to_der().expect("could not get TLS public key bytes");
                //println!("pubkey.len(): {}", pubkey.len());

                let mut sha256 = openssl::sha::Sha256::new();
                sha256.update(&pubkey);
                let pubkey = sha256.finish();

                let pubkey = ["sha256//", &openssl::base64::encode_block(&pubkey)].join("");
                println!("pubkey from cert: {}", pubkey);

                for key in pinnedpubkey.split(";") {
                    if key == pubkey {
                        println!("SUCCESS: pubkey match found!",);
                        return true;
                    }
                }
                println!("ERROR: pubkey match not found!");
                false
            });
        }
        let tcp_stream = connector.connect(hostname.unwrap_or(""), tcp_stream)?;
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

        if tls_key == "-" || tls_cert == "-" {
            let mut key_and_or_cert = Vec::new();
            println!("fully reading stdin...");
            std::io::stdin().read_to_end(&mut key_and_or_cert)?;
            println!("finished reading stdin");

            if tls_key == "-" {
                let tls_key = openssl::pkey::PKey::private_key_from_pem(&key_and_or_cert)?;
                acceptor.set_private_key(&tls_key)?;
            } else {
                acceptor.set_private_key_file(tls_key, SslFiletype::PEM)?;
            }
            if tls_cert == "-" {
                // todo: read whole chain here or???
                let tls_cert = openssl::x509::X509::from_pem(&key_and_or_cert)?;
                acceptor.set_certificate(&tls_cert)?;
            } else {
                acceptor.set_certificate_chain_file(tls_cert)?;
            }

        } else {
            // set from files
            acceptor.set_private_key_file(tls_key, SslFiletype::PEM)?;
            acceptor.set_certificate_chain_file(tls_cert)?;
        }
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
        Error::new_owned(format!("{}", value))
    }
}

impl From<HandshakeError<std::net::TcpStream>> for Error {
    fn from(value: HandshakeError<std::net::TcpStream>) -> Self {
        Error::new(value.description())
    }
}
