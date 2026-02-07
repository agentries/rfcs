use std::io::{self, BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(
            "usage: cargo run --bin amp005-client -- <alice|bob|did> [server_addr] [--once <to> <text>]"
                .into(),
        );
    }

    let me = resolve_alias(&args[1]);
    let mut cursor = 2;
    let mut server_addr = "127.0.0.1:7103".to_string();
    if args.get(cursor).is_some() && !args[cursor].starts_with("--") {
        server_addr = args[cursor].clone();
        cursor += 1;
    }

    let once = if let Some(flag) = args.get(cursor) {
        if flag == "--once" {
            if cursor + 2 >= args.len() {
                return Err("--once requires <to> <text>".into());
            }
            let to = resolve_alias(&args[cursor + 1]);
            let text = args[cursor + 2..].join(" ");
            Some((to, text))
        } else {
            return Err(format!("unknown option: {flag}").into());
        }
    } else {
        None
    };

    let stream = TcpStream::connect(&server_addr)?;
    stream.set_nodelay(true)?;
    println!("[client:{me}] connected to {server_addr}");

    let writer = Arc::new(Mutex::new(stream.try_clone()?));
    send_line(&writer, &format!("HELLO {me}"))?;

    let recv_writer = Arc::clone(&writer);
    let recv_me = me.clone();
    thread::spawn(move || {
        if let Err(err) = receiver_loop(stream, recv_writer, recv_me) {
            eprintln!("[client] receiver loop stopped: {err}");
        }
    });

    let counter = AtomicU64::new(1);

    if let Some((to, text)) = once {
        let msg_id = next_msg_id(&counter);
        send_line(&writer, &format!("SEND {msg_id} 60000 {to} {text}"))?;
        send_line(&writer, "POLL")?;
        thread::sleep(Duration::from_millis(700));
        send_line(&writer, "QUIT")?;
        return Ok(());
    }

    let default_target = if me.contains(":alice") {
        "did:web:example.com:agent:bob".to_string()
    } else {
        "did:web:example.com:agent:alice".to_string()
    };

    println!("commands:");
    println!("  /send <alice|bob|did> <text>");
    println!("  /send0 <alice|bob|did> <text>   (ttl=0)");
    println!("  /poll");
    println!("  /quit");
    println!("default: plain text sends to {default_target}");

    let mut line = String::new();
    loop {
        line.clear();
        let n = io::stdin().read_line(&mut line)?;
        if n == 0 {
            break;
        }
        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/quit" {
            send_line(&writer, "QUIT")?;
            println!("[client:{me}] quitting");
            break;
        }

        if input == "/poll" {
            send_line(&writer, "POLL")?;
            continue;
        }

        if let Some(rest) = input.strip_prefix("/send0 ") {
            if let Some((target, text)) = split_first(rest) {
                let to = resolve_alias(target);
                let msg_id = next_msg_id(&counter);
                send_line(&writer, &format!("SEND {msg_id} 0 {to} {text}"))?;
            } else {
                eprintln!("usage: /send0 <alice|bob|did> <text>");
            }
            continue;
        }

        if let Some(rest) = input.strip_prefix("/send ") {
            if let Some((target, text)) = split_first(rest) {
                let to = resolve_alias(target);
                let msg_id = next_msg_id(&counter);
                send_line(&writer, &format!("SEND {msg_id} 60000 {to} {text}"))?;
            } else {
                eprintln!("usage: /send <alice|bob|did> <text>");
            }
            continue;
        }

        let msg_id = next_msg_id(&counter);
        send_line(
            &writer,
            &format!("SEND {msg_id} 60000 {default_target} {input}"),
        )?;
    }

    Ok(())
}

fn receiver_loop(
    stream: TcpStream,
    writer: Arc<Mutex<TcpStream>>,
    me: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }

        let msg = line.trim();
        if msg.is_empty() {
            continue;
        }

        if let Some(rest) = msg.strip_prefix("MSG ") {
            let mut parts = rest.splitn(3, ' ');
            let from = parts.next().unwrap_or_default();
            let msg_id = parts.next().unwrap_or_default();
            let text = parts.next().unwrap_or_default();
            println!("[recv:{me}] from {from} msg_id={msg_id}: {text}");
            if !from.is_empty() && !msg_id.is_empty() {
                send_line(&writer, &format!("ACK {from} {msg_id}"))?;
            }
            continue;
        }

        if let Some(rest) = msg.strip_prefix("DELIVERED ") {
            let mut parts = rest.splitn(2, ' ');
            let msg_id = parts.next().unwrap_or_default();
            let recipient = parts.next().unwrap_or_default();
            println!("[recv:{me}] delivery confirmed msg_id={msg_id} recipient={recipient}");
            continue;
        }

        println!("[recv:{me}] {msg}");
    }

    Ok(())
}

fn send_line(writer: &Arc<Mutex<TcpStream>>, line: &str) -> io::Result<()> {
    let mut guard = writer.lock().expect("writer poisoned");
    guard.write_all(line.as_bytes())?;
    guard.write_all(b"\n")?;
    guard.flush()?;
    Ok(())
}

fn split_first(input: &str) -> Option<(&str, &str)> {
    let mut parts = input.splitn(2, ' ');
    let first = parts.next()?.trim();
    let rest = parts.next()?.trim();
    if first.is_empty() || rest.is_empty() {
        return None;
    }
    Some((first, rest))
}

fn resolve_alias(token: &str) -> String {
    match token {
        "alice" => "did:web:example.com:agent:alice".to_string(),
        "bob" => "did:web:example.com:agent:bob".to_string(),
        _ => token.to_string(),
    }
}

fn next_msg_id(counter: &AtomicU64) -> String {
    let n = counter.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}{:016x}", now_ms(), n)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis() as u64)
        .unwrap_or(0)
}
