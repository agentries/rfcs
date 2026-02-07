# rust-amp002-004

Transport-focused test project for RFC 002.

## Covered Cases

- `rfc002_tcp_frame_boundary_checks`
- `rfc002_websocket_mapping_rules`
- `rfc002_http_polling_and_relay_forward_wrapper_validation`
- `rfc002_http_relay_commit_wrapper_validation`
- `rfc002_principal_binding_rules`
- `rfc002_e2e_tcp_forward_between_two_clients`
- `rfc002_e2e_http_submit_then_poll`
- `rfc002_e2e_http_relay_forward_and_commit_with_principal_binding`

The three `rfc002_e2e_*` tests start local in-process relay servers and verify
end-to-end transport behavior over TCP and HTTP, including relay wrapper
validation and principal binding checks.

## Run

```bash
cargo test
```

## Runtime Demo (Server + Multi-Client)

Start relay server:

```bash
cargo run --bin amp002-server -- 127.0.0.1:7002
```

Start two interactive clients (in two terminals):

```bash
cargo run --bin amp002-client -- alice 127.0.0.1:7002
```

```bash
cargo run --bin amp002-client -- bob 127.0.0.1:7002
```

Then send messages with:

```text
/send bob hello-from-alice
/send alice hello-from-bob
```

One-shot mode (for scripted E2E):

```bash
cargo run --bin amp002-client -- alice 127.0.0.1:7002 --once bob hi
cargo run --bin amp002-client -- bob 127.0.0.1:7002 --once alice hi
```
