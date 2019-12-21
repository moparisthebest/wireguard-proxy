use std::net::UdpSocket;
use std::time::Duration;

use std::process::{exit, Command};
use std::{env, thread};
use wireguard_proxy::{Args, ProxyClient, ProxyServer};

const PONG: [u8; 246] = [
    0x6A, 0x2, 0x6B, 0xC, 0x6C, 0x3F, 0x6D, 0xC, 0xA2, 0xEA, 0xDA, 0xB6, 0xDC, 0xD6, 0x6E, 0x0,
    0x22, 0xD4, 0x66, 0x3, 0x68, 0x2, 0x60, 0x60, 0xF0, 0x15, 0xF0, 0x7, 0x30, 0x0, 0x12, 0x1A,
    0xC7, 0x17, 0x77, 0x8, 0x69, 0xFF, 0xA2, 0xF0, 0xD6, 0x71, 0xA2, 0xEA, 0xDA, 0xB6, 0xDC, 0xD6,
    0x60, 0x1, 0xE0, 0xA1, 0x7B, 0xFE, 0x60, 0x4, 0xE0, 0xA1, 0x7B, 0x2, 0x60, 0x1F, 0x8B, 0x2,
    0xDA, 0xB6, 0x60, 0xC, 0xE0, 0xA1, 0x7D, 0xFE, 0x60, 0xD, 0xE0, 0xA1, 0x7D, 0x2, 0x60, 0x1F,
    0x8D, 0x2, 0xDC, 0xD6, 0xA2, 0xF0, 0xD6, 0x71, 0x86, 0x84, 0x87, 0x94, 0x60, 0x3F, 0x86, 0x2,
    0x61, 0x1F, 0x87, 0x12, 0x46, 0x2, 0x12, 0x78, 0x46, 0x3F, 0x12, 0x82, 0x47, 0x1F, 0x69, 0xFF,
    0x47, 0x0, 0x69, 0x1, 0xD6, 0x71, 0x12, 0x2A, 0x68, 0x2, 0x63, 0x1, 0x80, 0x70, 0x80, 0xB5,
    0x12, 0x8A, 0x68, 0xFE, 0x63, 0xA, 0x80, 0x70, 0x80, 0xD5, 0x3F, 0x1, 0x12, 0xA2, 0x61, 0x2,
    0x80, 0x15, 0x3F, 0x1, 0x12, 0xBA, 0x80, 0x15, 0x3F, 0x1, 0x12, 0xC8, 0x80, 0x15, 0x3F, 0x1,
    0x12, 0xC2, 0x60, 0x20, 0xF0, 0x18, 0x22, 0xD4, 0x8E, 0x34, 0x22, 0xD4, 0x66, 0x3E, 0x33, 0x1,
    0x66, 0x3, 0x68, 0xFE, 0x33, 0x1, 0x68, 0x2, 0x12, 0x16, 0x79, 0xFF, 0x49, 0xFE, 0x69, 0xFF,
    0x12, 0xC8, 0x79, 0x1, 0x49, 0x2, 0x69, 0x1, 0x60, 0x4, 0xF0, 0x18, 0x76, 0x1, 0x46, 0x40,
    0x76, 0xFE, 0x12, 0x6C, 0xA2, 0xF2, 0xFE, 0x33, 0xF2, 0x65, 0xF1, 0x29, 0x64, 0x14, 0x65, 0x0,
    0xD4, 0x55, 0x74, 0x15, 0xF2, 0x29, 0xD4, 0x55, 0x0, 0xEE, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0x80, 0x0, 0x0, 0x0, 0x0, 0x0,
];

struct Server {
    udp_host: String,
    udp_target: String,
    socket_timeout: Option<Duration>,
}

impl Server {
    fn new(udp_host: String, udp_target: String, secs: u64) -> Server {
        Server {
            udp_host,
            udp_target,
            socket_timeout: match secs {
                0 => None,
                x => Some(Duration::from_secs(x)),
            },
        }
    }

    fn start(&self) -> std::io::Result<usize> {
        let udp_socket = UdpSocket::bind(&self.udp_host)?;
        udp_socket.set_read_timeout(self.socket_timeout)?;

        let sent = udp_socket.send_to(&PONG, &self.udp_target)?;
        assert_eq!(sent, PONG.len());

        let mut buf = [0u8; 2048];
        match udp_socket.recv_from(&mut buf) {
            Ok((len, src_addr)) => {
                println!("udp got len: {} from src_addr: {}", len, src_addr);
                assert_eq!(len, PONG.len());
                assert_eq!(&buf[..len], &PONG[..]);

                // now reply back to src_addr to make sure other direction works
                let sent = udp_socket.send_to(&PONG, &src_addr)?;
                assert_eq!(sent, PONG.len());
            }
            Err(e) => {
                panic!("recv function failed first round: {:?}", e);
            }
        }

        // if we are here, we successfully sent the reply, did we get it back?
        match udp_socket.recv(&mut buf) {
            Ok(len) => {
                println!("udp got len: {}", len);
                assert_eq!(len, PONG.len());
                assert_eq!(&buf[..len], &PONG[..]);
            }
            Err(e) => {
                panic!("recv function failed second round: {:?}", e);
            }
        }

        println!("success! received back exactly what we sent both ways!");

        Ok(0)
    }
}

fn main() {
    let raw_args = env::args().collect();
    let args = Args::new(&raw_args);
    let default_udp_host_target = "127.0.0.1:51820";
    let default_socket_timeout = 10;

    let tls_key = args.get_option(&["-tk", "--tls-key"]);
    let tls_cert = args.get_option(&["-tc", "--tls-cert"]);
    let pinnedpubkey = args.get_option(&["--pinnedpubkey"]);

    let tls = if tls_key.is_some() && tls_cert.is_some() {
        true
    } else if tls_key.is_none() && tls_cert.is_none() {
        false
    } else {
        println!("Error: if one of --tls-key or --tls-cert is specified both must be!");
        exit(1);
    };

    let mut first_arg = args.get_str(&["-uh", "--udp-host"], default_udp_host_target);
    if args.flag("-h") || args.flag("--help") {
        println!(r#"usage: udp-test [options...]
 -h, --help                      print this usage text
 -s, --self-test                 run a self test through proxy
 -is, --internal-self-test       run a self test through proxy without
                                 spawning other processes
 -uh, --udp-host <ip:port>       UDP host to listen on, default: {}
 -ut, --udp-target <ip:port>     UDP target to send packets to, default: {}
 -st, --socket-timeout <seconds> Socket timeout (time to wait for data)
                                 before terminating, default: {}

 TLS option for self tests only, otherwise self tests are plaintext only:
 -tk, --tls-key <ip:port>        TLS key to listen with,
                                 requires --tls-cert also
 -tc, --tls-cert <ip:port>       TLS cert to listen with,
                                 requires --tls-key also
 --pinnedpubkey <sha256_hashes>  Public key to verify peer against,
                                 format is any number of base64 encoded
                                 sha256 hashes preceded by "sha256//"
                                 and separated by ";". Identical to curl's
                                 --pinnedpubkey and CURLOPT_PINNEDPUBLICKEY

 Environment variable support:
 For every command line option, short and long, if you replace all
 leading - with WGP_, and replace all remaining - with _, and uppercase
 the whole thing, if you don't specify that command line option we will
 read that environment variable for the argument. boolean arguments are
 true if anything but unset, empty, 0, or false.
 Examples:
   --tcp-target ARG is WGP_TCP_TARGET=ARG
   --socket-timeout 5 is WGP_SOCKET_TIMEOUT=5
   --tls is WGP_TLS=1 or WGP_TLS=true
   WGP_TLS=0 or WGP_TLS=false would be like not sending --tls
        "#, default_udp_host_target, default_udp_host_target, default_socket_timeout);
        return;
    } else if args.flag("-s") || args.flag("--self-test") {
        // here is the hard work, we need to spawn proxyd and proxy from the same dir as udp-test...
        let host = "127.0.0.1:51822";
        let tcp_host = "127.0.0.1:5555";
        let sleep = Duration::from_secs(5);

        let udp_test = args.get_str_idx(0, "udp-test");
        let proxy = udp_test.clone().replace("udp-test", "wireguard-proxy");

        let mut proxyd_args = vec!["-th", tcp_host, "-ut", host];

        let tls_key = tls_key.as_ref().map(String::as_str);
        let tls_cert = tls_cert.as_ref().map(String::as_str);
        if tls {
            let tls_key = tls_key.unwrap();
            let tls_cert = tls_cert.unwrap();
            proxyd_args.extend(["-tk", &tls_key, "-tc", &tls_cert].iter().cloned());
        }

        println!("executing: {} {}", proxy, proxyd_args.join(" "));
        let mut proxyd = Command::new(proxy.clone())
                .args(&proxyd_args)
                .spawn()
                .expect("wireguard-proxy server failed to launch");
        println!("waiting: {:?} for wireguard-proxy server to come up.....", sleep);
        thread::sleep(sleep);

        let mut proxy_args = vec!["-tt", tcp_host];

        let pinnedpubkey = pinnedpubkey.as_ref().map(String::as_str);
        if tls {
            proxy_args.push("--tls");
            if pinnedpubkey.is_some() {
                proxy_args.push("--pinnedpubkey");
                proxy_args.push(&pinnedpubkey.unwrap());
            }
        }

        println!("executing: {} {}", proxy, proxy_args.join(" "));
        let mut proxy = Command::new(proxy)
                .args(proxy_args)
                .spawn()
                .expect("wireguard-proxy TLS client failed to launch");
        println!("waiting: {:?} for wireguard-proxy client to come up.....", sleep);
        thread::sleep(sleep);

        println!("executing: {} -uh '{}'", udp_test, host);
        let mut udp_test = Command::new(udp_test)
            .arg("-uh")
            .arg(host)
            .spawn()
            .expect("udp-test failed to launch");
        println!("waiting: {:?} for udp-test to come up.....", sleep);
        thread::sleep(sleep);

        // ignore all these, what could we do anyway?
        proxy.kill().ok();
        proxyd.kill().ok();
        udp_test.kill().ok();

        exit(
            udp_test
                .wait()
                .expect("could not get udp-test exit code")
                .code()
                .expect("could not get udp-test exit code"),
        );
    } else if args.flag("-is") || args.flag("--internal-self-test") {
        let host = "127.0.0.1:51822";
        let tcp_host = "127.0.0.1:5555";
        let sleep = Duration::from_secs(5);

        let proxy_server = ProxyServer::new(
            tcp_host.to_owned(),
            host.to_owned(),
            "127.0.0.1".to_owned(),
            30000,
            30100,
            0,
        );

        println!(
            "udp_target: {}, udp_bind_host_range: 127.0.0.1:30000-30100, socket_timeout: {:?}",
            proxy_server.client_handler.udp_target, proxy_server.client_handler.socket_timeout,
        );

        if tls {
            let tls_key = tls_key.unwrap().to_owned();
            let tls_cert = tls_cert.unwrap().to_owned();
            println!("executing: wireguard-proxy -th '{}' -ut '{}' -tk '{}' -tc '{}'", tcp_host, host, tls_key, tls_cert);
            thread::spawn(move || proxy_server.start_tls(&tls_key, &tls_cert).expect("error running TLS proxy_server"));
        } else {
            println!("executing: wireguard-proxy -th '{}' -ut '{}'", tcp_host, host);
            thread::spawn(move || proxy_server.start().expect("error running proxy_server"));
        }
        println!("waiting: {:?} for wireguard-proxy server to come up.....", sleep);
        thread::sleep(sleep);

        let proxy_client = ProxyClient::new(
            "127.0.0.1:51820".to_owned(),
            tcp_host.to_owned(),
            15,
        );

        println!(
            "udp_host: {}, tcp_target: {}, socket_timeout: {:?}",
            proxy_client.udp_host,
            proxy_client.tcp_target,
            proxy_client.socket_timeout,
        );

        if tls {
            let hostname = tcp_host.split(":").next();
            let pinnedpubkey = pinnedpubkey.as_ref().map(String::as_str);
            match pinnedpubkey {
                Some(pinnedpubkey) =>
                    println!("executing: wireguard-proxy -tt {} --tls --pinnedpubkey {}", tcp_host, pinnedpubkey),
                None =>
                    println!("executing: wireguard-proxy -tt {} --tls", tcp_host),
            }
            // this is a little funky, is this the only way to do it?
            let pinnedpubkey = pinnedpubkey.map(&str::to_owned);
            // can use pinnedpubkey.as_deref() below when it's stabilized
            thread::spawn(move || proxy_client.start_tls(hostname, pinnedpubkey.as_ref().map(String::as_str)).expect("error running proxy_client"));
        } else {
            println!("executing: wireguard-proxy -tt {}", tcp_host);
            thread::spawn(move || proxy_client.start().expect("error running proxy_client"));
        }
        println!("waiting: {:?} for wireguard-proxy client to come up.....", sleep);
        thread::sleep(sleep);

        first_arg = host.to_owned();
    }

    let server = Server::new(
        first_arg.to_owned(),
        args.get_str(&["-ut", "--udp-target"], default_udp_host_target).to_owned(),
        args.get(&["-st", "--socket-timeout"], default_socket_timeout),
    );

    println!(
        "udp_host: {}, udp_target: {}, socket_timeout: {:?}",
        server.udp_host, server.udp_target, server.socket_timeout,
    );

    server.start().expect("error running server");
}
