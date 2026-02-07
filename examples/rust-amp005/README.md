# rust-amp005

RFC 003 end-to-end and semantics test project for relay store-and-forward and federation handoff.

## Coverage

- Store-and-forward queueing, polling redelivery, and recipient ACK commit
- Multi-recipient independent commit progression
- TTL=0 immediate-delivery requirement (`2003` when next hop unavailable)
- Duplicate suppression by `(from_did, msg_id, recipient_did)`
- Expiry transition to terminal `Expired`
- Federation single-custody handoff + transfer receipt acceptance
- Federation dual-custody handoff + commit receipt feedback
- Federation rollback on timeout
- Loop prevention / hop-limit exhaustion / receipt tuple mismatch / unsupported alg-version
- Per-recipient federation split for multi-recipient messages

## Test Suites

- `tests/rfc003_semantics.rs`: direct RFC 003 appendix vector coverage
- `tests/rfc003_e2e.rs`: integrated E2E flows (upstream relay + downstream relay + recipient actions)

## Run

```bash
cargo test --offline
```

## Runtime Demo (Server + Multi-Client)

Start relay server:

```bash
cargo run --bin amp005-server -- 127.0.0.1:7103
```

Start clients in separate terminals:

```bash
cargo run --bin amp005-client -- alice 127.0.0.1:7103
```

```bash
cargo run --bin amp005-client -- bob 127.0.0.1:7103
```

Client commands:

```text
/send <alice|bob|did> <text>   # ttl=60000
/send0 <alice|bob|did> <text>  # ttl=0 immediate delivery only
/poll                           # pull queued/inflight messages
/quit
```

One-shot mode (scripted):

```bash
cargo run --bin amp005-client -- bob 127.0.0.1:7103 --once alice hello
cargo run --bin amp005-client -- alice 127.0.0.1:7103 --once bob hi
```
