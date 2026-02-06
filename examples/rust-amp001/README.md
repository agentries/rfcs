# AMP RFC 001 Rust Demo

This directory now contains two runnable demos:

- `amp-server`: TCP relay server
- `amp-client`: interactive client (`alice` or `bob`)

It uses real crypto and real AMP wire messages:

- Ed25519 signatures (`ed25519-dalek`)
- `authcrypt` encryption (`X25519-XSalsa20-Poly1305` with `crypto_box::SalsaBox`)
- CBOR wire encoding (`serde_cbor`)

## Start relay server

```bash
cargo run --bin amp-server -- 127.0.0.1:7001
```

## Start two clients (two terminals)

Terminal A:

```bash
cargo run --bin amp-client -- alice 127.0.0.1:7001
```

Terminal B:

```bash
cargo run --bin amp-client -- bob 127.0.0.1:7001
```

In either client:

- `/send bob hello from alice`
- `/send alice hi from bob`
- `/quit`
- Or type plain text directly (sends to default peer)

## One-shot in-process demo

You can still run the single-process flow:

```bash
cargo run --bin amp001-example
```

## Tests

```bash
cargo test
```

## Notes

- This is a demo environment; DID resolution is in-memory and deterministic.
- Server relays frames by `to` DID and does not perform full semantic validation.
- Client performs decrypt + signature verification + ACK behavior.
