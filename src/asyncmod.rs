
use tokio::net::UdpSocket;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::runtime::Runtime;
use tokio_rustls::webpki::DNSNameRef;

use crate::error;
use crate::error::Result;
use crate::*;

pub struct TcpUdpPipe<T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin + std::marker::Send + 'static> {
    buf: [u8; 2050], // 2048 + 2 for len
    tcp_stream: T,
    udp_socket: UdpSocket,
}

impl<T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin + std::marker::Send + 'static> TcpUdpPipe<T> {

    pub fn new(tcp_stream: T, udp_socket: UdpSocket) -> TcpUdpPipe<T> {
        TcpUdpPipe {
            tcp_stream,
            udp_socket,
            buf: [0u8; 2050],
        }
    }

    pub async fn shuffle_after_first_udp(mut self) -> Result<usize> {
        let (len, src_addr) = self.udp_socket.recv_from(&mut self.buf[2..]).await?;

        println!("first packet from {}, connecting to that", src_addr);
        self.udp_socket.connect(src_addr).await?;

        send_udp(&mut self.buf, &mut self.tcp_stream, len).await?;

        self.shuffle().await
    }

    pub async fn shuffle(self) -> Result<usize> {
        // todo: investigate https://docs.rs/tokio/0.2.22/tokio/net/struct.TcpStream.html#method.into_split
        let (mut tcp_rd, mut tcp_wr) = tokio::io::split(self.tcp_stream);
        let (mut udp_rd, mut udp_wr) = self.udp_socket.split();
        let mut recv_buf = self.buf.clone(); // or zeroed or?

        tokio::spawn(async move {
            loop {
                let len = udp_rd.recv(&mut recv_buf[2..]).await?;
                send_udp(&mut recv_buf, &mut tcp_wr, len).await?;
            }

            // Sometimes, the rust type inferencer needs
            // a little help
            #[allow(unreachable_code)]
                {
                    unsafe { std::hint::unreachable_unchecked(); }
                    Ok::<_, error::Error>(())
                }
        });

        let mut send_buf = self.buf.clone(); // or zeroed or?

        loop {
            tcp_rd.read_exact(&mut send_buf[..2]).await?;
            let len = ((send_buf[0] as usize) << 8) + send_buf[1] as usize;
            #[cfg(feature = "verbose")]
            println!("tcp expecting len: {}", len);
            tcp_rd.read_exact(&mut send_buf[..len]).await?;
            #[cfg(feature = "verbose")]
            println!("tcp got len: {}", len);
            udp_wr.send(&send_buf[..len]).await?;
        }

        #[allow(unreachable_code)]
            {
                unsafe { std::hint::unreachable_unchecked(); }
                Ok(0)
            }
    }
}

async fn send_udp<T: AsyncWriteExt + std::marker::Unpin + 'static>(buf: &mut [u8; 2050], tcp_stream: &mut T, len: usize) -> Result<()> {
    #[cfg(feature = "verbose")]
    println!("udp got len: {}", len);

    buf[0] = ((len >> 8) & 0xFF) as u8;
    buf[1] = (len & 0xFF) as u8;

    // todo: tcp_stream.write_all(&buf[..len + 2]).await
    Ok(tcp_stream.write_all(&buf[..len + 2]).await?)
    // todo: do this? self.tcp_stream.flush()
}

impl ProxyClient {

    pub async fn start_async(&self) -> Result<usize> {
        let tcp_stream = self.tcp_connect()?;

        let udp_socket = self.udp_connect()?;

        TcpUdpPipe::new(tokio::net::TcpStream::from_std(tcp_stream).expect("how could this tokio tcp fail?"), UdpSocket::from_std(udp_socket).expect("how could this tokio udp fail?"))
            .shuffle_after_first_udp().await
    }

    pub fn start(&self) -> Result<usize> {
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            self.start_async().await
        })
    }

    pub async fn start_tls_async(&self, hostname: Option<&str>, pinnedpubkey: Option<&str>) -> Result<usize> {
        let tcp_stream = self.tcp_connect()?;
        let tcp_stream = tokio::net::TcpStream::from_std(tcp_stream).expect("how could this tokio tcp fail?");

        use tokio_rustls::{ TlsConnector, rustls::ClientConfig };

        let mut config = ClientConfig::new();
        config.dangerous().set_certificate_verifier(match pinnedpubkey {
            Some(pinnedpubkey) => Arc::new(PinnedpubkeyCertVerifier { pinnedpubkey: pinnedpubkey.to_owned() }),
            None => Arc::new(DummyCertVerifier{}),
        });

        let hostname = match hostname {
            Some(hostname) => match DNSNameRef::try_from_ascii_str(hostname) {
                Ok(hostname) => hostname,
                Err(_) => {
                    config.enable_sni = false;
                    DNSNameRef::try_from_ascii_str(&"dummy.hostname").unwrap() // why does rustls ABSOLUTELY REQUIRE this ????
                }
            },
            None => {
                config.enable_sni = false;
                DNSNameRef::try_from_ascii_str(&"dummy.hostname").unwrap() // why does rustls ABSOLUTELY REQUIRE this ????
            }
        };
        //println!("hostname: {:?}", hostname);

        let connector = TlsConnector::from(Arc::new(config));

        let tcp_stream= connector.connect(hostname, tcp_stream).await?;

        let udp_socket = self.udp_connect()?;

        // we want to wait for first udp packet from client first, to set the target to respond to
        TcpUdpPipe::new(tcp_stream, UdpSocket::from_std(udp_socket).expect("how could this tokio udp fail?"))
            .shuffle_after_first_udp().await
    }

    pub fn start_tls(&self, hostname: Option<&str>, pinnedpubkey: Option<&str>) -> Result<usize> {
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            self.start_tls_async(hostname, pinnedpubkey).await
        })
    }
}

use tokio_rustls::rustls;
use tokio_rustls::webpki;

struct DummyCertVerifier;

impl rustls::ServerCertVerifier for DummyCertVerifier {
    fn verify_server_cert(&self,
                          _roots: &rustls::RootCertStore,
                          _certs: &[rustls::Certificate],
                          _hostname: webpki::DNSNameRef<'_>,
                          _ocsp: &[u8]) -> core::result::Result<rustls::ServerCertVerified, rustls::TLSError> {
        // verify nothing, subject to MITM
        Ok(rustls::ServerCertVerified::assertion())
    }
}

struct PinnedpubkeyCertVerifier {
    pinnedpubkey: String,
}

impl rustls::ServerCertVerifier for PinnedpubkeyCertVerifier {
    fn verify_server_cert(&self,
                          _roots: &rustls::RootCertStore,
                          certs: &[rustls::Certificate],
                          _hostname: webpki::DNSNameRef<'_>,
                          _ocsp: &[u8]) -> core::result::Result<rustls::ServerCertVerified, rustls::TLSError> {
        if certs.is_empty() {
            return Err(rustls::TLSError::NoCertificatesPresented);
        }
        let cert = webpki::trust_anchor_util::cert_der_as_trust_anchor(&certs[0].0)
            .map_err(rustls::TLSError::WebPKIError)?;

        //println!("spki.len(): {}", cert.spki.len());
        //println!("spki: {:?}", cert.spki);
        // todo: what is wrong with webpki? it returns *almost* the right answer but missing these leading bytes:
        // guess I'll open an issue... (I assume this is some type of algorithm identifying header or something)
        let mut pubkey: Vec<u8> = vec![48, 130, 1, 34];
        pubkey.extend(cert.spki);

        let pubkey = ring::digest::digest(&ring::digest::SHA256, &pubkey);
        let pubkey = base64::encode(pubkey);
        let pubkey = ["sha256//", &pubkey].join("");

        for key in self.pinnedpubkey.split(";") {
            if key == pubkey {
                return Ok(rustls::ServerCertVerified::assertion());
            }
        }

        Err(rustls::TLSError::General(format!("pubkey '{}' not found in allowed list '{}'", pubkey, self.pinnedpubkey)))
    }
}

impl ProxyServer {

    pub async fn start_async(&self) -> Result<()> {
        let mut listener = tokio::net::TcpListener::bind(&self.tcp_host).await?;
        println!("Listening for connections on {}", &self.tcp_host);

        loop {
            let (stream, _) = listener.accept().await?;
            let client_handler = self.client_handler.clone();
            tokio::spawn(async move {
                client_handler
                    .handle_client_async(stream).await
                    .expect("error handling connection");
            });
        }

        #[allow(unreachable_code)]
            {
                unsafe { std::hint::unreachable_unchecked(); }
                Ok(())
            }
    }

    pub fn start(&self) -> Result<()> {
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            self.start_async().await
        })
    }

    pub async fn start_tls_async(&self, tls_key: &str, tls_cert: &str) -> Result<()> {

        use std::fs::File;
        use std::io::BufReader;
        use std::io;
        use tokio_rustls::rustls::internal::pemfile::{ certs, pkcs8_private_keys };

        let mut tls_key = pkcs8_private_keys(&mut BufReader::new(File::open(tls_key)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;
        if tls_key.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))?;
        }
        let tls_key = tls_key.remove(0);

        let tls_cert = certs(&mut BufReader::new(File::open(tls_cert)?))
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;

        let mut config = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        config.set_single_cert(tls_cert, tls_key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));

        let mut listener = tokio::net::TcpListener::bind(&self.tcp_host).await?;
        println!("Listening for TLS connections on {}", &self.tcp_host);

        loop {
            let (stream, _) = listener.accept().await?;
            let client_handler = self.client_handler.clone();
            let acceptor = acceptor.clone();

            tokio::spawn(async move {
                let stream = acceptor.accept(stream).await.expect("failed to wrap with TLS?");

                client_handler
                    .handle_client_async(stream).await
                    .expect("error handling connection");
            });
        }

        #[allow(unreachable_code)]
            {
                unsafe { std::hint::unreachable_unchecked(); }
                Ok(())
            }
    }

    pub fn start_tls(&self, tls_key: &str, tls_cert: &str) -> Result<()> {
        let mut rt = Runtime::new()?;

        rt.block_on(async {
            self.start_tls_async(tls_key, tls_cert).await
        })
    }
}

impl ProxyServerClientHandler {

    pub async fn handle_client_async<T: AsyncReadExt + AsyncWriteExt + std::marker::Unpin + std::marker::Send + 'static>(&self, tcp_stream: T) -> Result<usize> {
        TcpUdpPipe::new(tcp_stream,
                                   UdpSocket::from_std(self.udp_bind()?).expect("how could this tokio udp fail?")
        ).shuffle().await
    }
}