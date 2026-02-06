use std::collections::HashMap;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

use amp001_example::{peek_routing, read_frame, write_frame};

#[derive(Default)]
struct RelayState {
    writers: HashMap<String, Arc<Mutex<TcpStream>>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:7001".to_string());

    let listener = TcpListener::bind(&addr)?;
    let state = Arc::new(Mutex::new(RelayState::default()));

    println!("AMP relay server listening on {addr}");

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
    let mut reader = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));

    let mut registered_did: Option<String> = None;

    loop {
        let frame = match read_frame(&mut reader) {
            Ok(frame) => frame,
            Err(err) => {
                eprintln!("[server] read_frame ended: {err}");
                break;
            }
        };

        let routing = match peek_routing(&frame) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("[server] drop malformed frame: {err}");
                continue;
            }
        };

        match &registered_did {
            Some(current) if current != &routing.from => {
                eprintln!(
                    "[server] sender DID switched on same connection: {} -> {} (drop)",
                    current, routing.from
                );
                continue;
            }
            None => {
                registered_did = Some(routing.from.clone());
                let mut guard = state.lock().expect("relay state poisoned");
                guard
                    .writers
                    .insert(routing.from.clone(), Arc::clone(&writer));
                println!("[server] registered {}", routing.from);
            }
            _ => {}
        }

        let recipients: Vec<(String, Arc<Mutex<TcpStream>>)> = {
            let guard = state.lock().expect("relay state poisoned");
            routing
                .to
                .iter()
                .filter_map(|did| {
                    guard
                        .writers
                        .get(did)
                        .map(|w| (did.clone(), Arc::clone(w)))
                })
                .collect()
        };

        if recipients.is_empty() {
            println!(
                "[server] no online recipient for typ=0x{:02x} from={} to={:?}",
                routing.typ, routing.from, routing.to
            );
            continue;
        }

        for (recipient, socket) in recipients {
            let mut guard = socket.lock().expect("recipient socket poisoned");
            if let Err(err) = write_frame(&mut *guard, &frame) {
                eprintln!("[server] forward to {recipient} failed: {err}");
            } else {
                println!(
                    "[server] forwarded typ=0x{:02x} from={} to={}",
                    routing.typ, routing.from, recipient
                );
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
