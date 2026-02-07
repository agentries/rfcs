use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use amp005_rfc003_tests::{Message, Relay, RelayError};

#[derive(Debug)]
struct Delivery {
    from_did: String,
    msg_id: String,
    recipient_did: String,
    text: String,
}

#[derive(Debug)]
struct SenderNotice {
    sender_did: String,
    msg_id: String,
    recipient_did: String,
}

#[derive(Debug)]
struct RelayState {
    relay: Relay,
    payloads: HashMap<(String, String, String), String>,
    writers: HashMap<String, Arc<Mutex<TcpStream>>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7103".to_string());

    let listener = TcpListener::bind(&addr)?;
    let state = Arc::new(Mutex::new(RelayState {
        relay: Relay::new("did:web:example.com:relay:store", now_ms()),
        payloads: HashMap::new(),
        writers: HashMap::new(),
    }));

    println!("AMP RFC003 relay server listening on {addr}");
    println!("protocol: HELLO/SEND/POLL/ACK/QUIT");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|v| v.to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                println!("[server] accepted {peer}");

                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, state) {
                        eprintln!("[server] connection error: {err}");
                    }
                });
            }
            Err(err) => eprintln!("[server] accept failed: {err}"),
        }
    }

    Ok(())
}

fn handle_connection(
    stream: TcpStream,
    state: Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let writer = Arc::new(Mutex::new(stream));
    let mut registered_did: Option<String> = None;

    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match dispatch_command(input, &writer, &state, &mut registered_did) {
            Ok(keep_running) => {
                if !keep_running {
                    break;
                }
            }
            Err(err) => {
                send_line(&writer, &format!("ERR 5001 INTERNAL {err}"))?;
            }
        }
    }

    if let Some(did) = registered_did {
        let mut guard = state.lock().expect("relay state poisoned");
        guard.writers.remove(&did);
        println!("[server] unregistered {did}");
    }

    Ok(())
}

fn dispatch_command(
    input: &str,
    writer: &Arc<Mutex<TcpStream>>,
    state: &Arc<Mutex<RelayState>>,
    registered_did: &mut Option<String>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if let Some(rest) = input.strip_prefix("HELLO ") {
        let did = rest.trim();
        if did.is_empty() {
            send_line(writer, "ERR 1001 INVALID_MESSAGE missing did")?;
            return Ok(true);
        }

        if let Some(current) = registered_did {
            if current != did {
                send_line(writer, "ERR 3001 UNAUTHORIZED sender DID switch denied")?;
                return Ok(true);
            }
        } else {
            let mut guard = state.lock().expect("relay state poisoned");
            guard.writers.insert(did.to_string(), Arc::clone(writer));
            *registered_did = Some(did.to_string());
            println!("[server] registered {did}");
        }

        send_line(writer, &format!("OK HELLO {did}"))?;
        return Ok(true);
    }

    let Some(me) = registered_did.as_ref() else {
        send_line(writer, "ERR 3001 UNAUTHORIZED send HELLO first")?;
        return Ok(true);
    };

    if let Some(rest) = input.strip_prefix("SEND ") {
        handle_send(me, rest, writer, state)?;
        return Ok(true);
    }

    if input == "POLL" {
        handle_poll(me, writer, state)?;
        return Ok(true);
    }

    if let Some(rest) = input.strip_prefix("ACK ") {
        handle_ack(me, rest, writer, state)?;
        return Ok(true);
    }

    if input == "QUIT" {
        send_line(writer, "OK BYE")?;
        return Ok(false);
    }

    send_line(writer, "ERR 1001 INVALID_MESSAGE unknown command")?;
    Ok(true)
}

fn handle_send(
    me: &str,
    rest: &str,
    writer: &Arc<Mutex<TcpStream>>,
    state: &Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = rest.splitn(4, ' ');
    let msg_id = parts.next().unwrap_or_default().trim();
    let ttl_ms = parts.next().unwrap_or_default().trim();
    let recipient = parts.next().unwrap_or_default().trim();
    let text = parts.next().unwrap_or_default().trim();

    if msg_id.is_empty() || ttl_ms.is_empty() || recipient.is_empty() || text.is_empty() {
        send_line(
            writer,
            "ERR 1001 INVALID_MESSAGE SEND format: SEND <msg_id> <ttl_ms> <recipient_did> <text>",
        )?;
        return Ok(());
    }

    let ttl_ms: u64 = match ttl_ms.parse() {
        Ok(v) => v,
        Err(_) => {
            send_line(writer, "ERR 1001 INVALID_MESSAGE ttl_ms must be uint")?;
            return Ok(());
        }
    };

    let mut deliveries = Vec::new();
    let mut immediate_delivery: Option<Delivery> = None;
    {
        let mut guard = state.lock().expect("relay state poisoned");
        guard.relay.set_now(now_ms());
        guard.relay.expire();

        let mut recipient_online = HashMap::new();
        recipient_online.insert(recipient.to_string(), guard.writers.contains_key(recipient));

        let message = Message {
            from_did: me.to_string(),
            msg_id: msg_id.to_string(),
            recipients: vec![recipient.to_string()],
            ts_ms: guard.relay.now_ms,
            ttl_ms,
        };

        if let Err(err) = guard.relay.ingress(&message, &recipient_online) {
            send_line(writer, &render_relay_error(&err))?;
            return Ok(());
        }
        println!(
            "[server] accepted SEND from={} msg_id={} to={} ttl_ms={}",
            me, msg_id, recipient, ttl_ms
        );

        if ttl_ms == 0 {
            if recipient_online[recipient] {
                immediate_delivery = Some(Delivery {
                    from_did: me.to_string(),
                    msg_id: msg_id.to_string(),
                    recipient_did: recipient.to_string(),
                    text: text.to_string(),
                });
            }
        } else {
            let key = (me.to_string(), msg_id.to_string(), recipient.to_string());
            guard.payloads.entry(key).or_insert_with(|| text.to_string());
            if recipient_online[recipient] {
                deliveries = collect_deliveries_for(&mut guard, recipient);
            }
        }
    }

    if let Some(delivery) = immediate_delivery {
        push_delivery(&delivery, state)?;
    }
    for delivery in deliveries {
        push_delivery(&delivery, state)?;
    }

    send_line(writer, &format!("OK SEND {msg_id}"))?;
    Ok(())
}

fn handle_poll(
    me: &str,
    writer: &Arc<Mutex<TcpStream>>,
    state: &Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let deliveries = {
        let mut guard = state.lock().expect("relay state poisoned");
        guard.relay.set_now(now_ms());
        guard.relay.expire();
        collect_deliveries_for(&mut guard, me)
    };

    for delivery in &deliveries {
        send_line(
            writer,
            &format!(
                "MSG {} {} {}",
                delivery.from_did, delivery.msg_id, delivery.text
            ),
        )?;
    }

    println!("[server] poll recipient={} returned={}", me, deliveries.len());
    send_line(writer, &format!("OK POLL {}", deliveries.len()))?;
    Ok(())
}

fn handle_ack(
    me: &str,
    rest: &str,
    writer: &Arc<Mutex<TcpStream>>,
    state: &Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut parts = rest.splitn(2, ' ');
    let from_did = parts.next().unwrap_or_default().trim();
    let msg_id = parts.next().unwrap_or_default().trim();

    if from_did.is_empty() || msg_id.is_empty() {
        send_line(
            writer,
            "ERR 1001 INVALID_MESSAGE ACK format: ACK <from_did> <msg_id>",
        )?;
        return Ok(());
    }

    let mut sender_notice: Option<SenderNotice> = None;
    {
        let mut guard = state.lock().expect("relay state poisoned");
        guard.relay.set_now(now_ms());
        guard.relay.expire();

        if let Err(err) = guard.relay.ack_recipient(from_did, msg_id, me) {
            send_line(writer, &render_relay_error(&err))?;
            return Ok(());
        }
        println!(
            "[server] ACK from recipient={} for from={} msg_id={}",
            me, from_did, msg_id
        );

        guard
            .payloads
            .remove(&(from_did.to_string(), msg_id.to_string(), me.to_string()));
        if guard.writers.contains_key(from_did) {
            sender_notice = Some(SenderNotice {
                sender_did: from_did.to_string(),
                msg_id: msg_id.to_string(),
                recipient_did: me.to_string(),
            });
        }
    }

    if let Some(notice) = sender_notice {
        push_sender_notice(&notice, state)?;
    }

    send_line(writer, &format!("OK ACK {msg_id}"))?;
    Ok(())
}

fn collect_deliveries_for(state: &mut RelayState, recipient_did: &str) -> Vec<Delivery> {
    let mut out = Vec::new();
    for (from_did, msg_id) in state.relay.poll(recipient_did) {
        if let Some(text) = state
            .payloads
            .get(&(from_did.clone(), msg_id.clone(), recipient_did.to_string()))
            .cloned()
        {
            out.push(Delivery {
                from_did,
                msg_id,
                recipient_did: recipient_did.to_string(),
                text,
            });
        }
    }
    out
}

fn push_delivery(
    delivery: &Delivery,
    state: &Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let maybe_writer = {
        let guard = state.lock().expect("relay state poisoned");
        guard.writers.get(&delivery.recipient_did).cloned()
    };

    if let Some(writer) = maybe_writer {
        println!(
            "[server] deliver msg_id={} from={} to={}",
            delivery.msg_id, delivery.from_did, delivery.recipient_did
        );
        send_line(
            &writer,
            &format!(
                "MSG {} {} {}",
                delivery.from_did, delivery.msg_id, delivery.text
            ),
        )?;
    }
    Ok(())
}

fn push_sender_notice(
    notice: &SenderNotice,
    state: &Arc<Mutex<RelayState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let maybe_writer = {
        let guard = state.lock().expect("relay state poisoned");
        guard.writers.get(&notice.sender_did).cloned()
    };

    if let Some(writer) = maybe_writer {
        println!(
            "[server] delivery notice msg_id={} to sender={} recipient={}",
            notice.msg_id, notice.sender_did, notice.recipient_did
        );
        send_line(
            &writer,
            &format!("DELIVERED {} {}", notice.msg_id, notice.recipient_did),
        )?;
    }
    Ok(())
}

fn send_line(writer: &Arc<Mutex<TcpStream>>, line: &str) -> std::io::Result<()> {
    let mut guard = writer.lock().expect("writer poisoned");
    guard.write_all(line.as_bytes())?;
    guard.write_all(b"\n")?;
    guard.flush()?;
    Ok(())
}

fn render_relay_error(err: &RelayError) -> String {
    format!("ERR {} {} {}", err.code, err.name, err.detail)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|v| v.as_millis() as u64)
        .unwrap_or(0)
}
