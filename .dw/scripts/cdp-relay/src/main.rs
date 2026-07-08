//! cdp-relay — a tiny TCP forwarder.
//!
//! Default mode listens on 0.0.0.0:<listen> and forwards every connection to
//! 127.0.0.1:<target>. Reverse mode connects outbound to a WSL broker, then forwards
//! each broker-approved session to 127.0.0.1:<target>. dev-workflow uses reverse mode
//! on WSL NAT so no Windows admin/firewall rule is required. std-only: no crates, no
//! runtime. dev-workflow ships a prebuilt Windows x64 .exe; this source is kept for
//! maintainers who need to rebuild it.

use std::env;
use std::io::{self, Read};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

const DEFAULT_REVERSE_POOL: usize = 8;
const RETRY_DELAY: Duration = Duration::from_millis(200);

fn pump(mut from: TcpStream, mut to: TcpStream) {
    let _ = io::copy(&mut from, &mut to);
    // Signal EOF to the peer so the other direction can finish cleanly.
    let _ = to.shutdown(Shutdown::Write);
}

fn bridge(left: TcpStream, right: TcpStream) -> io::Result<()> {
    left.set_nodelay(true).ok();
    right.set_nodelay(true).ok();

    let left_rx = left.try_clone()?;
    let right_rx = right.try_clone()?;
    thread::spawn(move || pump(left, right_rx));
    thread::spawn(move || pump(right, left_rx));
    Ok(())
}

fn handle(client: TcpStream, target_port: u16) -> io::Result<()> {
    let upstream = TcpStream::connect(("127.0.0.1", target_port))?;
    bridge(client, upstream)
}

fn wait_for_go(stream: &mut TcpStream) -> io::Result<()> {
    let mut seen = Vec::with_capacity(3);
    let mut byte = [0u8; 1];
    while seen.len() < 3 {
        stream.read_exact(&mut byte)?;
        seen.push(byte[0]);
        if seen == b"GO\n" {
            return Ok(());
        }
    }
    Err(io::Error::new(io::ErrorKind::InvalidData, "expected GO"))
}

fn run_listen(listen_port: u16, target_port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", listen_port)).expect("bind failed");
    for stream in listener.incoming() {
        if let Ok(client) = stream {
            // One connection failing must not take down the relay.
            let _ = handle(client, target_port);
        }
    }
    Ok(())
}

fn handle_reverse_session(mut broker: TcpStream, target_port: u16) -> io::Result<()> {
    broker.set_nodelay(true).ok();
    wait_for_go(&mut broker)?;
    let upstream = TcpStream::connect(("127.0.0.1", target_port))?;
    bridge(broker, upstream)
}

fn reverse_worker(broker_host: String, broker_port: u16, target_port: u16) {
    loop {
        match TcpStream::connect((broker_host.as_str(), broker_port)) {
            Ok(broker) => {
                if handle_reverse_session(broker, target_port).is_err() {
                    thread::sleep(RETRY_DELAY);
                }
            }
            Err(_) => thread::sleep(RETRY_DELAY),
        }
    }
}

fn run_reverse(broker_host: String, broker_port: u16, target_port: u16, pool_size: usize) {
    let pool_size = pool_size.max(1);
    for _ in 0..pool_size {
        let host = broker_host.clone();
        thread::spawn(move || reverse_worker(host, broker_port, target_port));
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

fn usage() -> ! {
    eprintln!("usage:");
    eprintln!("  cdp-relay <listen_port> <target_port>");
    eprintln!("  cdp-relay --reverse <broker_host> <broker_port> <target_port> [pool_size]");
    std::process::exit(2);
}

fn parse_port(value: &str, name: &str) -> u16 {
    value.parse().unwrap_or_else(|_| {
        eprintln!("invalid {name}");
        std::process::exit(2);
    })
}

fn parse_pool(value: Option<&String>) -> usize {
    value
        .map(|s| {
            s.parse().unwrap_or_else(|_| {
                eprintln!("invalid pool_size");
                std::process::exit(2);
            })
        })
        .unwrap_or(DEFAULT_REVERSE_POOL)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.get(1).map(String::as_str) == Some("--reverse") {
        if args.len() < 5 {
            usage();
        }
        let broker_host = args[2].clone();
        let broker_port = parse_port(&args[3], "broker_port");
        let target_port = parse_port(&args[4], "target_port");
        run_reverse(
            broker_host,
            broker_port,
            target_port,
            parse_pool(args.get(5)),
        );
    }

    if args.len() < 3 {
        usage();
    }
    let listen_port = parse_port(&args[1], "listen_port");
    let target_port = parse_port(&args[2], "target_port");
    if let Err(err) = run_listen(listen_port, target_port) {
        eprintln!("cdp-relay: {err}");
        std::process::exit(1);
    }
}
