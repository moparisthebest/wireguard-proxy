use std::net::{TcpStream, UdpSocket};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

mod error;
use error::Result;

fn arg_to_env(arg: &str) -> Option<String> {
    if !arg.starts_with("--") {
        return None;
    }
    let env = "WGP_".to_owned();
    let mut env = env + &arg.trim_matches('-').replace("-", "_");
    env.make_ascii_uppercase();
    Some(env)
}

fn env_for_arg(arg: &str) -> Option<String> {
    arg_to_env(arg).and_then(|key| std::env::var(key).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_arg_to_env() {
        assert_eq!(arg_to_env("--tcp-host"), Some("WGP_TCP_HOST".to_owned()));
        assert_eq!(arg_to_env("--tls"), Some("WGP_TLS".to_owned()));
        assert_eq!(arg_to_env("-h"), None);
        assert_eq!(arg_to_env("-th"), None);
    }
}

pub struct Args<'a> {
    args: &'a Vec<String>,
}

impl<'a> Args<'a> {
    pub fn new(args: &'a Vec<String>) -> Args {
        Args { args }
    }
    pub fn flag(&self, flag: &'a str) -> bool {
        if self.args.contains(&flag.to_owned()) {
            return true;
        }
        // because env we want slightly special handling of empty/0/false
        match env_for_arg(flag) {
            Some(env) => &env != "" && &env != "0" && &env != "false",
            None => false,
        }
    }
    pub fn get_option(&self, flags: &[&'a str]) -> Option<String> {
        for flag in flags.iter() {
            let mut found = false;
            for arg in self.args.iter() {
                if found {
                    return Some(arg.to_owned());
                }
                if arg == flag {
                    found = true;
                }
            }
        }
        // no matching arguments are found, so check env variables as a fallback
        for flag in flags.iter() {
            let env = env_for_arg(flag);
            if env.is_some() {
                return env;
            }
        }
        return None;
    }
    pub fn get_str(&self, flags: &[&'a str], def: &'a str) -> String {
        match self.get_option(flags) {
            Some(ret) => ret,
            None => def.to_owned(),
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
}

pub struct ProxyClient {
    pub udp_host: String,
    pub tcp_target: String,
    pub socket_timeout: Option<Duration>,
}

pub struct ProxyServer {
    pub tcp_host: String,
    pub client_handler: Arc<ProxyServerClientHandler>,
}

pub struct ProxyServerClientHandler {
    pub udp_target: String,
    pub udp_host: String,
    pub udp_low_port: u16,
    pub udp_high_port: u16,
    pub socket_timeout: Option<Duration>,
}

#[cfg(any(feature = "async"))]
#[path = ""]
mod net {
    mod asyncmod;
}

#[cfg(not(any(feature = "async")))]
#[path = ""]
mod net {



    mod syncmod;
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

    fn tcp_connect(&self) -> Result<TcpStream> {
        let tcp_stream = TcpStream::connect(&self.tcp_target)?;
        tcp_stream.set_read_timeout(self.socket_timeout)?;
        Ok(tcp_stream)
    }

    fn udp_connect(&self) -> Result<UdpSocket> {
        let udp_socket = UdpSocket::bind(&self.udp_host)?;
        udp_socket.set_read_timeout(self.socket_timeout)?;
        Ok(udp_socket)
    }
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
}

impl ProxyServerClientHandler {
    fn udp_bind(&self) -> Result<UdpSocket> {
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
        Ok(udp_socket)
    }
}
