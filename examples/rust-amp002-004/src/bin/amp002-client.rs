use std::io;
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use amp001_example::{
    build_authcrypt_signed, build_plain_signed, demo_agents, make_message_id, now_ms, read_frame,
    receive_and_verify, validate_ack_semantics, write_frame, AckBody, AckSource, AgentKeys,
    DidResolver, HelloBody, MessageMeta, Recipients, TextMessageBody, TYPE_ACK, TYPE_HELLO,
    TYPE_MESSAGE,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        return Err(
            "usage: cargo run --bin amp002-client -- <alice|bob> [server_addr] [--once <alice|bob|did> <text>]"
                .into(),
        );
    }

    let name = args[1].as_str();
    let mut cursor = 2;
    let mut server_addr = "127.0.0.1:7002".to_string();

    if args.get(cursor).is_some() && !args[cursor].starts_with("--") {
        server_addr = args[cursor].clone();
        cursor += 1;
    }

    let once = if let Some(flag) = args.get(cursor) {
        if flag == "--once" {
            if cursor + 2 >= args.len() {
                return Err("--once requires <target> <text>".into());
            }
            let target = args[cursor + 1].clone();
            let text = args[cursor + 2..].join(" ");
            Some((target, text))
        } else {
            return Err(format!("unknown flag: {flag}").into());
        }
    } else {
        None
    };

    let demo = demo_agents();
    let me = demo
        .by_name(name)
        .ok_or("client name must be one of: alice, bob")?;
    let resolver = demo.resolver();

    let stream = TcpStream::connect(&server_addr)?;
    stream.set_nodelay(true)?;

    println!("[client:{name}] connected to {server_addr}");

    let reader = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));
    let counter = Arc::new(AtomicU64::new(1));

    send_hello_registration(&me, &demo.relay.did, &writer, &counter)?;

    let recv_writer = Arc::clone(&writer);
    let recv_counter = Arc::clone(&counter);
    let recv_resolver = resolver.clone();
    let recv_me = me.clone();

    thread::spawn(move || {
        if let Err(err) = receiver_loop(reader, recv_writer, recv_counter, recv_me, recv_resolver) {
            eprintln!("[client] receiver loop stopped: {err}");
        }
    });

    if let Some((target, text)) = once {
        let target_did = resolve_target_did(&demo, &resolver, &target)
            .map_err(|v| io::Error::new(io::ErrorKind::InvalidInput, v))?;
        send_text_message(&me, &resolver, &writer, &counter, &target_did, &text)?;
        thread::sleep(Duration::from_millis(600));
        println!("[client:{name}] one-shot mode done");
        return Ok(());
    }

    let default_target = if me.did.contains(":alice") {
        demo.bob.did.clone()
    } else {
        demo.alice.did.clone()
    };

    println!("commands:");
    println!("  /send <alice|bob|did> <text>");
    println!("  /quit");
    println!("default: type plain text to send to {default_target}");

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
            println!("[client:{name}] quitting");
            break;
        }

        if let Some(rest) = input.strip_prefix("/send ") {
            if let Some((target_token, text)) = split_first(rest) {
                let target_did = match resolve_target_did(&demo, &resolver, target_token) {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("[client:{name}] {err}");
                        continue;
                    }
                };
                if let Err(err) =
                    send_text_message(&me, &resolver, &writer, &counter, &target_did, text)
                {
                    eprintln!("[client:{name}] send failed: {err}");
                }
            } else {
                eprintln!("usage: /send <alice|bob|did> <text>");
            }
            continue;
        }

        if let Err(err) = send_text_message(&me, &resolver, &writer, &counter, &default_target, input)
        {
            eprintln!("[client:{name}] send failed: {err}");
        }
    }

    Ok(())
}

fn receiver_loop(
    mut reader: TcpStream,
    writer: Arc<Mutex<TcpStream>>,
    counter: Arc<AtomicU64>,
    me: AgentKeys,
    resolver: DidResolver,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        let frame = read_frame(&mut reader)?;
        let message = match receive_and_verify(&me, &frame, &resolver, now_ms()) {
            Ok(msg) => msg,
            Err(err) => {
                eprintln!("[recv:{}] rejected frame: {}", me.did, err);
                continue;
            }
        };

        match message.meta.typ {
            TYPE_MESSAGE => {
                let body: TextMessageBody = match message.decode_body() {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("[recv:{}] decode message body failed: {}", me.did, err);
                        continue;
                    }
                };
                println!("[recv:{}] from {}: {}", me.did, message.meta.from, body.msg);

                if let Err(err) =
                    send_ack(&me, &writer, &counter, &message.meta.from, message.meta.id)
                {
                    eprintln!("[recv:{}] send ACK failed: {}", me.did, err);
                }
            }
            TYPE_ACK => {
                let ack: AckBody = match message.decode_body() {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("[recv:{}] decode ACK failed: {}", me.did, err);
                        continue;
                    }
                };

                let expected_sender = vec![message.meta.from.clone()];
                if let Err(err) = validate_ack_semantics(&message, &expected_sender, &resolver) {
                    eprintln!("[recv:{}] ACK semantic check failed: {}", me.did, err);
                } else {
                    let reply_to = message
                        .meta
                        .reply_to
                        .map(|id| hex16(&id))
                        .unwrap_or_else(|| "none".to_string());
                    println!(
                        "[recv:{}] ACK from {} source={:?} reply_to={}",
                        me.did, message.meta.from, ack.ack_source, reply_to
                    );
                }
            }
            TYPE_HELLO => {
                let hello: HelloBody = match message.decode_body() {
                    Ok(v) => v,
                    Err(err) => {
                        eprintln!("[recv:{}] decode HELLO failed: {}", me.did, err);
                        continue;
                    }
                };
                println!(
                    "[recv:{}] HELLO from {} {:?}",
                    me.did, message.meta.from, hello.versions
                );
            }
            other => {
                println!("[recv:{}] typ=0x{other:02x} from={}", me.did, message.meta.from);
            }
        }
    }
}

fn send_hello_registration(
    me: &AgentKeys,
    relay_did: &str,
    writer: &Arc<Mutex<TcpStream>>,
    counter: &AtomicU64,
) -> Result<(), Box<dyn std::error::Error>> {
    let ts = now_ms();
    let id = make_message_id(ts, counter.fetch_add(1, Ordering::Relaxed));
    let body = HelloBody {
        versions: vec!["0.30.0".to_string()],
    };

    let meta = MessageMeta {
        v: 1,
        id,
        typ: TYPE_HELLO,
        ts_ms: ts,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(relay_did.to_string()),
        reply_to: None,
        thread_id: None,
    };

    let wire = build_plain_signed(me, meta, &body)?;
    let mut guard = writer.lock().expect("writer poisoned");
    write_frame(&mut *guard, &wire)?;
    println!("[client:{}] registration HELLO sent", me.did);

    Ok(())
}

fn send_text_message(
    me: &AgentKeys,
    resolver: &DidResolver,
    writer: &Arc<Mutex<TcpStream>>,
    counter: &AtomicU64,
    target_did: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if resolver.key_agreement_for(target_did).is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown recipient DID or missing keyAgreement key: {target_did}",
            ),
        )
        .into());
    }

    let ts = now_ms();
    let id = make_message_id(ts, counter.fetch_add(1, Ordering::Relaxed));

    let meta = MessageMeta {
        v: 1,
        id,
        typ: TYPE_MESSAGE,
        ts_ms: ts,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(target_did.to_string()),
        reply_to: None,
        thread_id: None,
    };

    let body = TextMessageBody {
        msg: text.to_string(),
    };

    let wire = build_authcrypt_signed(me, target_did, meta, &body, resolver)?;
    let mut guard = writer.lock().expect("writer poisoned");
    write_frame(&mut *guard, &wire)?;

    println!("[client:{}] sent encrypted MESSAGE to {target_did}", me.did);
    Ok(())
}

fn send_ack(
    me: &AgentKeys,
    writer: &Arc<Mutex<TcpStream>>,
    counter: &AtomicU64,
    target_did: &str,
    reply_to: [u8; 16],
) -> Result<(), Box<dyn std::error::Error>> {
    let ts = now_ms();
    let id = make_message_id(ts, counter.fetch_add(1, Ordering::Relaxed));
    let body = AckBody {
        ack_source: AckSource::Recipient,
        received_at: ts,
        ack_target: None,
    };

    let meta = MessageMeta {
        v: 1,
        id,
        typ: TYPE_ACK,
        ts_ms: ts,
        ttl_ms: 60_000,
        from: String::new(),
        to: Recipients::One(target_did.to_string()),
        reply_to: Some(reply_to),
        thread_id: None,
    };

    let wire = build_plain_signed(me, meta, &body)?;
    let mut guard = writer.lock().expect("writer poisoned");
    write_frame(&mut *guard, &wire)?;
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

fn resolve_target_did(
    demo: &amp001_example::DemoAgents,
    resolver: &DidResolver,
    target_token: &str,
) -> Result<String, String> {
    let token = target_token.trim();
    let mapped = demo.did_for_alias(token);

    if mapped.starts_with("did:") {
        if resolver.key_agreement_for(&mapped).is_some() {
            return Ok(mapped);
        }
        return Err(format!(
            "target DID not available in local resolver: {mapped} (use alice/bob or a known DID)",
        ));
    }

    Err(format!(
        "invalid target '{token}': use alice, bob, relay, or a full DID",
    ))
}

fn hex16(id: &[u8; 16]) -> String {
    let mut out = String::with_capacity(32);
    for b in id {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
