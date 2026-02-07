# RFC 007: Agent Payment Protocol (APP)

**Status**: Draft
**Authors**: Ryan Cooper, Nowa
**Created**: 2026-02-04
**Updated**: 2026-02-07
**Version**: 0.34

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (principal binding)
- RFC 003: Relay and Store-and-Forward (delivery and retries)
- RFC 004: Capability Schema Registry and Compatibility (optional payment capability negotiation)
- RFC 005: Delegation Credentials and Authorization (delegated execution boundary)
- RFC 006: Session Protocol (stateful payment workflows)
- RFC 008: Agent Discovery and Directory (payment endpoint publication)
- RFC 009: Reputation and Trust Signals (post-payment trust updates)

---

## Abstract

This RFC defines deterministic payment semantics for agent-to-agent transactions over AMP. It specifies quote, authorization, capture, cancel, refund, and status flows using signed AMP bodies while keeping settlement-network specifics out of scope. The goal is interoperable economic workflows without coupling AMP to any single blockchain, currency, or custody architecture.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Payment Model and Identifiers
4.1 Lifecycle States
4.2 Amount and Asset Model
4.3 Payment Core CDDL
5. Payment Protocol Semantics
5.1 Message Type Usage and Dispatch
5.2 Quote Flow
5.3 Authorize Flow
5.4 Capture Flow
5.5 Cancel Flow
5.6 Refund Flow
5.7 Status Query
5.8 Payment-Control Operation Direction and Correlation Matrix
5.9 Idempotency and Replay Rules
5.10 CAP_INVOKE Interop Profile
5.11 Payment Body CDDL
6. Settlement Abstraction Contract
6.1 Settlement Proof Verification
7. State Machines
8. Error Handling and Retry
9. Versioning and Compatibility
10. Security Considerations
11. Privacy Considerations
12. Implementation Checklist
13. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Problem Statement and Scope

AMP defines messaging, transport, relay, capability, delegation, and session semantics, but does not define interoperable payment workflows. Without a payment layer, implementations diverge on quote validity, state transitions, settlement evidence, and error handling.

This RFC defines:
- Signed payment workflow bodies for quote, authorize, capture, cancel, refund, and status.
- Deterministic payment state transitions and idempotency expectations.
- Settlement-proof abstraction with cross-implementation verification requirements.
- Deterministic payment-specific error mapping.

This RFC does not define:
- A mandatory blockchain, token, or fiat rail.
- Wallet custody UX, key custody products, or smart-contract standards.
- Jurisdiction-specific tax/compliance policy.

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope, signature, and idempotency semantics.
- Applies payment dispatch rules in Section 5.1.
- Implements required payment-control operation schemas in Section 5.11.
- If CAP path is supported, implements Section 5.10 and CAP mapping schemas in Section 5.11.
- Enforces operation direction/correlation and idempotency requirements in Sections 5.8 and 5.9.
- Enforces state transitions in Section 7.
- Uses deterministic error mapping in Section 8.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Payment ID | Stable payment identifier (`bstr .size 16`) for one payment intent lifecycle. |
| Payment-control message | AMP `REQUEST`/`RESPONSE` with signed body `pay_v` field, parsed by this RFC. |
| Quote | Signed commercial offer (asset, amount, expiry, terms) issued by payee. |
| Authorization | Payer-side confirmation that funds are reserved/approved for capture. |
| Capture | Finalization step that moves authorized value into settlement execution. |
| Settlement proof | Signed adapter evidence that binds settlement outcome to (`payment_id`, `money`, `network`). |
| Refund | Reversal flow after capture/settlement according to policy. |

### 2.2 Role Profiles and MTI Requirements

`Payer Agent Profile`:
- MUST validate quote expiry and terms before authorization.
- MUST use stable `payment_id` per intent and preserve idempotency on retries.
- MUST reject capture requests that violate authorized amount/asset constraints.

`Payee Agent Profile`:
- MUST issue deterministic quote bodies.
- MUST enforce one payment lifecycle state machine per `payment_id`.
- MUST provide status query responses for active and terminal payments.

`Settlement Adapter Profile` (optional):
- MUST produce settlement-proof objects in Section 6 format.
- MUST report finality using deterministic adapter policy.
- MUST fail closed on unverifiable settlement evidence.

`Escrow Extension` (optional):
- MAY introduce intermediary hold/release states if mapped to Section 7 transitions.
- MUST expose escrow transitions through the same payment status schema.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC reuses existing `REQUEST`/`RESPONSE` type codes and does not allocate new AMP message types.
- Payment semantics are encoded in signed body fields (`pay_v`, `op`, payload).
- `pay_v` is reserved for payment-control semantics in `REQUEST`/`RESPONSE` bodies.

With RFC 002:
- Transport principal binding remains mandatory.
- Unauthorized principal/from combinations in payment operations MUST map to `3001`.

With RFC 003:
- Relay redelivery may produce duplicates; endpoints MUST rely on RFC 001 idempotency.
- Payment operations MUST be safe under at-least-once delivery.

With RFC 004:
- Payment services MAY be exposed as capabilities.
- If payment operations are carried via `CAP_INVOKE`, capability/version negotiation follows RFC 004.
- CAP interop baseline for this RFC is `org.agentries.payment.workflow:1.0.0` (Section 5.10).

With RFC 005:
- This revision does not define delegation carriage inside payment-control bodies.
- If delegated payment execution is required, implementations MUST use delegated `CAP_INVOKE` path and follow RFC 005.

With RFC 006:
- Payment flows MAY be session-scoped.
- If session-scoped, `body.session` requirements and thread rules follow RFC 006.

With RFC 008/009:
- Discovery MAY publish payment endpoint hints and supported assets.
- Reputation updates derived from payment outcomes are out of scope and defined by RFC 009.

---

## 4. Payment Model and Identifiers

### 4.1 Lifecycle States

Canonical states:
- `quoted` -> `authorized` -> `captured` -> (`settled` / `failed`)
- `quoted` -> `canceled`
- `captured|settled` -> (`refund_pending` -> `refunded` / `failed`)

State constraints:
- `capture` is valid only from `authorized`.
- `cancel` is valid only before `captured`.
- `refund` is valid only after `captured` or `settled`.

### 4.2 Amount and Asset Model

- Amount MUST be integer minor units (`amount_minor`) to avoid floating ambiguity.
- `asset` identifies currency/token code (for example `USDC`, `USD`).
- `network` identifies settlement domain (for example `base-mainnet`, `offchain-ledger-a`).
- Amount/asset/network MUST remain immutable after authorization unless explicit cancel + re-quote occurs.
- `asset` canonical form is uppercase ASCII `[A-Z0-9._-]`, length `1..32`.
- `network` canonical form is lowercase ASCII `[a-z0-9._-]`, length `1..64`.
- Senders MUST emit canonical `asset`/`network`; receivers MUST reject non-canonical values with `4001`.
- Comparison for `asset`/`network` is byte-exact after canonical form validation (no case folding at receiver).

### 4.3 Payment Core CDDL

```cddl
payment-id = bstr .size 16
quote-id = bstr .size 16
unix-ms = uint
asset-code = tstr
network-code = tstr
did = tstr
; asset-code canonical form: uppercase ASCII [A-Z0-9._-], len 1..32
; network-code canonical form: lowercase ASCII [a-z0-9._-], len 1..64

payment-status =
  "quoted" / "authorized" / "captured" / "settled" /
  "failed" / "canceled" / "refund_pending" / "refunded"

money = {
  "amount_minor": uint,
  "asset": asset-code,
  "network": network-code
}

payment-session-context = {
  "session_id": bstr .size 16,
  "session_scope": true
}

settlement-proof-payload = {
  "proof_v": 1,
  "network": network-code,
  "payment_id": payment-id,
  "money": money,
  "reference": tstr,
  ? "confirmations": uint,
  ? "finalized_at": unix-ms
}

settlement-proof = {
  "payload": settlement-proof-payload,
  "adapter_id": did,
  "sig_alg": "Ed25519",
  "sig": bstr .size 64
}

payment-base = {
  "pay_v": 1,
  "op": tstr,
  "payment_id": payment-id,
  ? "session": payment-session-context
}
```

---

## 5. Payment Protocol Semantics

### 5.1 Message Type Usage and Dispatch

- Payment-control requests MUST use `typ = REQUEST`.
- Payment-control responses MUST use `typ = RESPONSE` and MUST set envelope `reply_to` to triggering request `id`.
- A message is treated as payment-control only when `typ` is `REQUEST`/`RESPONSE` and signed body contains `pay_v`.
- For payment-control messages, `op` and `payment_id` are REQUIRED; missing/invalid fields MUST be rejected as `1001`.
- `REQUEST`/`RESPONSE` messages without `pay_v` are not parsed as payment-control by this RFC.
- Unknown `op` values in payment-control messages MUST be rejected with `4105`.
- `op`/`typ` direction mismatch MUST be rejected with `4001`.

### 5.2 Quote Flow

- `quote_request`: payer asks for payable terms.
- `quote`: payee returns `quote_id`, `money`, expiry, and optional terms hash.
- Quote validity MUST be enforced by both sides using `quote_expires_at`.

### 5.3 Authorize Flow

- `authorize` binds `payment_id` to a valid `quote_id` and payer approval.
- Authorization response MUST indicate `status = "authorized"` or terminal failure.
- Repeated authorize with same semantic request MUST be idempotent (Section 5.9).

### 5.4 Capture Flow

- `capture` finalizes previously authorized payment intent.
- Capture success MUST return status `captured` or `settled` plus optional settlement proof.
- Capture before authorization MUST fail deterministically.
- Repeated capture with same semantic request MUST be idempotent (Section 5.9).

### 5.5 Cancel Flow

- `cancel` is pre-capture termination.
- `cancel_result` MUST return `status = "canceled"` or `"failed"`.
- Successful cancel MUST move lifecycle to terminal `canceled`.
- Repeated cancel with same semantic request MUST be idempotent (Section 5.9).

### 5.6 Refund Flow

- `refund` is post-capture reversal request.
- `refund_result` MUST return `status = "refund_pending"` / `"refunded"` / `"failed"`.
- Refund policies are implementation-specific, but state transitions MUST remain deterministic.
- Repeated refund with same semantic request MUST be idempotent (Section 5.9).

### 5.7 Status Query

- `status_query` requests current `payment_status` and optional settlement/refund metadata.
- `status` response MUST include current canonical state.
- Unknown `payment_id` in `status_query` MUST be rejected with `4106`.

### 5.8 Payment-Control Operation Direction and Correlation Matrix

The following matrix is normative:

| Request `op` | Request sender -> receiver | Response `op` | Response sender -> receiver | `reply_to` requirement |
|--------------|----------------------------|---------------|-----------------------------|------------------------|
| `quote_request` | payer -> payee | `quote` | payee -> payer | `quote.reply_to` MUST reference triggering `quote_request.id` |
| `authorize` | payer -> payee | `authorize_result` | payee -> payer | `authorize_result.reply_to` MUST reference triggering `authorize.id` |
| `capture` | payer -> payee | `capture_result` | payee -> payer | `capture_result.reply_to` MUST reference triggering `capture.id` |
| `cancel` | payer -> payee | `cancel_result` | payee -> payer | `cancel_result.reply_to` MUST reference triggering `cancel.id` |
| `refund` | payer -> payee | `refund_result` | payee -> payer | `refund_result.reply_to` MUST reference triggering `refund.id` |
| `status_query` | payer or payee -> counterparty | `status` | queried party -> requester | `status.reply_to` MUST reference triggering `status_query.id` |

Rules:
- This section applies only to payment-control carriage parsed by Section 5.1 (`typ` in `REQUEST`/`RESPONSE` with signed body `pay_v`).
- CAP carriage uses `CAP_INVOKE`/`CAP_RESULT` semantics in Section 5.10 and RFC 004.
- `quote`, `authorize_result`, `capture_result`, `cancel_result`, `refund_result`, and `status` MUST be sent as `RESPONSE`.
- `quote_request`, `authorize`, `capture`, `cancel`, `refund`, and `status_query` MUST be sent as `REQUEST`.
- Unsolicited payment response operations (missing/invalid `reply_to`) MUST be rejected with `4001`.

### 5.9 Idempotency and Replay Rules

Payment logic MUST remain safe under RFC 003 at-least-once delivery.

Operation idempotency key:
- `op_key = (payment_id, op, from_did)`.

Rules:
- This section applies to both payment-control carriage (Section 5.1) and CAP carriage (Section 5.10).
- In CAP carriage, `payment_id` and `op` are read from `CAP_INVOKE.params`.
- Replaying a request with same semantic body under the same `op_key` MUST return a deterministic equivalent result and MUST NOT create duplicate lifecycle transitions.
- Replaying with same `op_key` but conflicting semantic body (for example changed `quote_id`, `money`, or counterparty fields) MUST be rejected with `4001`.
- `capture` replay after already `captured` or `settled` MUST return prior terminal-equivalent `capture_result`.
- `cancel` replay after `canceled` MUST return prior terminal-equivalent `cancel_result`.
- `refund` replay in `refund_pending` or `refunded` MUST return latest deterministic `refund_result`.
- `status_query` is read-only and MAY be retried freely; unknown `payment_id` MUST map to `4106`.

### 5.10 CAP_INVOKE Interop Profile

This section defines the RFC 007 capability interoperability baseline for RFC 004 invocation path.

Capability identity:
- `id = "org.agentries.payment.workflow:1.0.0"`.

Rules:
- When using CAP path, `CAP_INVOKE` MUST target the capability ID above.
- `CAP_INVOKE.params` MUST contain one request operation body from this RFC with `pay_v = 1`.
- Allowed request ops in CAP path: `quote_request`, `authorize`, `capture`, `cancel`, `refund`, `status_query`.
- `CAP_RESULT(status="success").result` MUST contain one corresponding response operation body from this RFC.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005 before payment execution.
- Invalid/unsupported delegation evidence in CAP payment path MUST fail with `3004` (RFC 004/005).
- Section 5.8 payment-control `REQUEST`/`RESPONSE` direction rules MUST NOT be applied to CAP envelope types.
- Providers supporting this capability MUST publish an RFC 004-compliant descriptor for `org.agentries.payment.workflow:1.0.0`.
- Descriptor/schema integrity verification (hash/signature/trust profile behavior) MUST follow RFC 004 Sections 4.2 and 5.2 before schema validation/execution.
- Descriptor input schema MUST correspond to `app-cap-invoke-params`; success result schema MUST correspond to `app-cap-result-success`.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path (validation/authorization/compatibility/schema) MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

### 5.11 Payment Body CDDL

```cddl
payment-op =
  "quote_request" / "quote" /
  "authorize" / "authorize_result" /
  "capture" / "capture_result" /
  "cancel" / "cancel_result" /
  "refund" / "refund_result" /
  "status_query" / "status"

quote-request-body = {
  payment-base,
  "op": "quote_request",
  "payee": did,
  "money": money,
  ? "service_ref": tstr
}

quote-body = {
  payment-base,
  "op": "quote",
  "quote_id": quote-id,
  "payee": did,
  "payer": did,
  "money": money,
  "quote_expires_at": unix-ms,
  ? "terms_hash": bstr
}

authorize-body = {
  payment-base,
  "op": "authorize",
  "quote_id": quote-id,
  "payer": did,
  "payee": did,
  "money": money
}

authorize-result-body = {
  payment-base,
  "op": "authorize_result",
  "status": "authorized" / "failed",
  ? "reason_code": uint,
  ? "reason": tstr,
  ? "authorized_at": unix-ms
}

capture-body = {
  payment-base,
  "op": "capture",
  "quote_id": quote-id,
  "money": money
}

capture-result-body = {
  payment-base,
  "op": "capture_result",
  "status": "captured" / "settled" / "failed",
  ? "settlement": settlement-proof,
  ? "reason_code": uint,
  ? "reason": tstr
}

cancel-body = {
  payment-base,
  "op": "cancel",
  ? "reason": tstr
}

cancel-result-body = {
  payment-base,
  "op": "cancel_result",
  "status": "canceled" / "failed",
  ? "reason_code": uint,
  ? "reason": tstr
}

refund-body = {
  payment-base,
  "op": "refund",
  ? "money": money,
  ? "reason": tstr
}

refund-result-body = {
  payment-base,
  "op": "refund_result",
  "status": "refund_pending" / "refunded" / "failed",
  ? "settlement": settlement-proof,
  ? "reason_code": uint,
  ? "reason": tstr
}

status-query-body = {
  payment-base,
  "op": "status_query"
}

status-body = {
  payment-base,
  "op": "status",
  "status": payment-status,
  ? "settlement": settlement-proof,
  ? "updated_at": unix-ms
}

app-capability-id = "org.agentries.payment.workflow:1.0.0"

app-cap-invoke-params =
  quote-request-body /
  authorize-body /
  capture-body /
  cancel-body /
  refund-body /
  status-query-body

app-cap-result-success =
  quote-body /
  authorize-result-body /
  capture-result-body /
  cancel-result-body /
  refund-result-body /
  status-body
```

---

## 6. Settlement Abstraction Contract

Settlement adapter requirements:
- Adapter MUST bind proof to (`payment_id`, `money`, `network`) via signed `settlement-proof.payload`.
- Adapter MUST expose pending/finalized evidence through payload fields.
- Adapter MUST be deterministic for a given input/reference.

### 6.1 Settlement Proof Verification

Verification steps:
1. Parse `settlement` object; shape/type failures -> `1001`.
2. Verify `payload.payment_id` equals current payment intent `payment_id`.
3. Verify `payload.money` and `payload.network` equal captured payment values.
4. Resolve `adapter_id` DID verification key per local trust policy.
5. Verify `sig_alg == "Ed25519"` and signature over:
   - `CBOR_Encode(["APP-proof-v1", deterministic_cbor(payload)])`
6. On signature/binding mismatch -> `3001`.

Interoperability rule:
- AMP payment logic uses only `settlement-proof` contract above.
- Chain-specific proof internals remain adapter-private and out of scope.

---

## 7. State Machines

```
NONE
  -> QUOTED              (quote issued)
QUOTED
  -> AUTHORIZED          (authorize_result success)
  -> CANCELED            (cancel_result success)
  -> FAILED              (quote expired/invalid)
AUTHORIZED
  -> CAPTURED            (capture_result captured)
  -> SETTLED             (capture_result settled)
  -> FAILED              (capture_result failed)
CAPTURED
  -> SETTLED             (settlement final)
  -> REFUND_PENDING      (refund_result pending)
SETTLED
  -> REFUND_PENDING      (refund_result pending)
REFUND_PENDING
  -> REFUNDED            (refund_result finalized)
  -> FAILED              (refund_result failed)
FAILED / CANCELED / REFUNDED
  -> TERMINAL
```

---

## 8. Error Handling and Retry

This RFC reuses RFC 001 error model and introduces payment-specific business codes in `41xx` range.

Deterministic precedence:
- Parse/shape/type failures -> `1001`.
- Unsupported `pay_v` -> `1004`.
- Authorization identity/policy failure -> `3001`.
- CAP pre-resolution/coarse policy denial -> `3001` (RFC 004 validation order).
- CAP delegation evidence failure (after coarse auth checks) -> `3004`.
- CAP descriptor signature/trust-profile verification failure -> `3001`.
- Payment semantic/request-shape conflicts -> `4001`.
- Payment state/business failures -> `41xx`.
- CAP descriptor/schema artifact unavailable or integrity source unavailable -> `5002`.
- Transient backend/adapter failures -> `500x`.

| Condition | Code | Retry |
|----------|------|-------|
| Malformed payment body | `1001` | No |
| Unsupported `pay_v` | `1004` | No |
| Invalid/unsupported CAP delegation evidence (`CAP_INVOKE.body.delegation`) | `3004` | No |
| Unauthorized payment actor | `3001` | No |
| CAP pre-resolution/coarse policy denial | `3001` | No |
| Payment `op`/`typ` direction mismatch | `4001` | No |
| CAP session context mismatch (envelope vs payload) | `4001` | No |
| Non-canonical `asset`/`network` format | `4001` | No |
| Conflicting retry payload for same (`payment_id`, `op`, `from_did`) | `4001` | No |
| Malformed settlement proof object | `1001` | No |
| Settlement proof signature/binding verification failed | `3001` | No |
| Insufficient funds | `4101` | No |
| Quote expired | `4102` | No |
| Invalid state transition | `4103` | No |
| Unsupported asset/network by payee policy | `4104` | No |
| Unknown payment operation | `4105` | No |
| Payment not found (`payment_id` unknown) | `4106` | No |
| CAP descriptor signature required but missing/invalid under trust profile | `3001` | No |
| CAP descriptor/schema artifact missing, unreadable, or integrity source unavailable | `5002` | Yes |
| Settlement adapter unavailable | `5002` | Yes |
| Internal payment engine failure | `5001` | Yes |

Registry note:
- `41xx` payment codes MUST be registered via RFC 001 Section 17 process before status advances beyond Draft.

---

## 9. Versioning and Compatibility

Version dimensions:
- AMP envelope version `v` remains governed by RFC 001.
- Payment body schema version is `pay_v`, currently fixed at `1`.

Compatibility rules:
- Unknown required fields MUST fail with `1001`.
- Unknown optional fields MAY be ignored unless security-sensitive.
- Backward-compatible extensions MUST use optional fields only.

---

## 10. Security Considerations

- Replay safety: implementations MUST enforce RFC 001 idempotency with stable `payment_id` semantics.
- Quote tamper protection: quote terms MUST be in signed body and revalidated at authorize/capture.
- Double-spend resistance: capture MUST require prior valid authorization state.
- Settlement proof authenticity and binding MUST be verified before terminal `settled`/`refunded` states.
- Principal/from binding from RFC 002 applies to all payment operations.

---

## 11. Privacy Considerations

- Payment metadata can reveal business relationships and pricing.
- Implementations SHOULD minimize retention of sensitive quote/payment details.
- Logs SHOULD prefer IDs and status codes over full commercial terms where possible.

---

## 12. Implementation Checklist

- Implement payment dispatch guard in Section 5.1.
- Implement operation direction/correlation matrix in Section 5.8.
- Implement idempotency/replay rules in Section 5.9.
- Implement CAP interop profile in Section 5.10 if CAP path is supported.
- Implement all Section 5.11 operation schemas.
- Enforce lifecycle transitions in Section 7.
- Enforce deterministic error mapping in Section 8.
- Ensure idempotent behavior on repeated payment operation retries.
- Verify settlement proofs using Section 6.1 before `settled`/`refunded`.
- Add conformance tests from Appendix A.

---

## 13. References

### 13.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords

### 13.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 004: Capability Schema Registry and Compatibility
- RFC 005: Delegation Credentials and Authorization
- RFC 006: Session Protocol
- RFC 008: Agent Discovery and Directory
- RFC 009: Reputation and Trust Signals

---

## Appendix A. Minimal Test Vectors

### A.1 Quote to Capture Positive

Input:
- `quote_request` -> valid `quote` -> valid `authorize` -> valid `capture`.

Expected:
- State transitions `quoted -> authorized -> captured|settled`.

### A.2 Quote Expiry Negative

Input:
- `authorize` arrives after `quote_expires_at`.

Expected:
- Reject with `4102`.

### A.3 Capture Before Authorization Negative

Input:
- `capture` for `payment_id` still in `quoted`.

Expected:
- Reject with `4103`.

### A.4 Duplicate Authorize Idempotent Positive

Input:
- Same `authorize` retried with same `payment_id` and same signed body.

Expected:
- Deterministic idempotent outcome; no duplicate state transition.

### A.5a Settlement Proof Malformed Negative

Input:
- `capture_result` includes malformed `settlement` object shape.

Expected:
- Reject with `1001 INVALID_FORMAT`.

### A.5b Settlement Proof Binding/Signature Failure Negative

Input:
- `capture_result` includes parseable `settlement`, but signature invalid or payload binding mismatches `payment_id`/`money`/`network`.

Expected:
- Reject with `3001 UNAUTHORIZED`.

### A.6 Session-Scoped Payment Positive

Input:
- Payment flow carries RFC 006-compliant `body.session` context.

Expected:
- Payment flow accepted with session correlation preserved.

### A.7 Delegated Payment via CAP_INVOKE Boundary Positive

Input:
- Payment execution invoked via delegated `CAP_INVOKE` path.

Expected:
- Delegation validation handled by RFC 005; payment semantics in this RFC remain unchanged.

### A.8 Unsupported pay_v Negative

Input:
- Payment-control message with unsupported `pay_v`.

Expected:
- Reject with `1004 UNSUPPORTED_VERSION`.

### A.9 Payment Dispatch Guard Positive

Input:
- `REQUEST` body contains business `op` but no `pay_v`.

Expected:
- Message is not parsed as payment-control by RFC 007.

### A.10 Cancel Result Positive

Input:
- Valid `cancel` before capture, followed by `cancel_result` success.

Expected:
- State transitions to `canceled` terminal.

### A.11 Refund Result Positive

Input:
- Valid `refund` after settlement, followed by `refund_result` pending then `refunded`.

Expected:
- State transitions `settled -> refund_pending -> refunded`.

### A.12 Unknown Payment ID Negative

Input:
- `status_query` for `payment_id` with no known lifecycle record.

Expected:
- Reject with `4106`.

### A.13 Duplicate Capture Idempotent Positive

Input:
- Same `capture` replayed with same semantic body and same `payment_id` after first success.

Expected:
- Deterministic terminal-equivalent `capture_result`; no duplicate capture transition.

### A.14 CAP_INVOKE Payment Profile Positive

Input:
- `CAP_INVOKE` by `id = org.agentries.payment.workflow:1.0.0` with `params = authorize-body`.

Expected:
- RFC 004 capability negotiation/validation passes.
- Payment semantics execute per this RFC and return `CAP_RESULT` with `result = authorize-result-body`.

### A.15 Non-Canonical Asset Code Negative

Input:
- `quote_request.money.asset = "usd"` (lowercase, non-canonical).

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.16 CAP Carriage Direction Independence Positive

Input:
- `CAP_INVOKE` request and `CAP_RESULT` response carrying valid RFC 007 payment bodies in params/result.

Expected:
- Flow MUST NOT be rejected by Section 5.8 payment-control direction checks.
- Flow is validated using Section 5.10 + RFC 004 semantics.

### A.17a CAP Descriptor Signature Invalid Negative

Input:
- Capability ID matches `org.agentries.payment.workflow:1.0.0`.
- Active trust profile requires descriptor signature.
- Descriptor signature is missing or invalid.

Expected:
- Reject with `3001 UNAUTHORIZED`.
- Byte-level check: error code `3001` is CBOR uint bytes `19 0b b9`.

### A.17b CAP Descriptor Artifact Unavailable Negative

Input:
- Capability ID matches `org.agentries.payment.workflow:1.0.0`.
- Descriptor/schema artifact is missing, unreadable, or integrity source is unavailable.

Expected:
- Reject with `5002 UNAVAILABLE`.
- Byte-level check: error code `5002` is CBOR uint bytes `19 13 8a`.

### A.18 CAP Delegation Evidence Invalid Negative

Input:
- CAP payment invocation includes invalid `CAP_INVOKE.body.delegation` evidence.

Expected:
- Reject with `3004 DELEGATION_INVALID`.
- Byte-level check: error code `3004` is CBOR uint bytes `19 0b bc`.

### A.19 CAP Coarse Policy Denial Negative

Input:
- CAP payment invocation fails coarse authorization/policy before delegation/capability resolution.

Expected:
- Reject with `3001 UNAUTHORIZED` (RFC 004 validation order).

### A.20 CAP Session Context Mismatch Negative

Input:
- `CAP_INVOKE.body.session` and `CAP_INVOKE.params.session` both present but semantically inconsistent.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.21 CAP Rejection vs Execution Failure Split

Input:
- Case 1: validation-time rejection before execution start.
- Case 2: execution-time failure after request accepted.

Expected:
- Case 1 returns AMP `ERROR` (not `CAP_RESULT`).
- Case 2 returns `CAP_RESULT(status="error")`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-04 | Proposal | Ryan Cooper | Initial proposal outline |
| 2026-02-07 | 0.1 | Nowa | Rewrote RFC 007 into normative draft structure with profiles, boundary contracts, CDDL, lifecycle state machine, error mapping, and minimal test vectors |
| 2026-02-07 | 0.2 | Nowa | Added deterministic payment dispatch rules, typed session context, settlement-proof verification contract, cancel/refund result schemas, and stricter vector/error consistency |
| 2026-02-07 | 0.3 | Nowa | Added operation direction matrix, full replay/idempotency contract, CAP_INVOKE payment interop profile, canonical asset/network format rules, `4106 PAYMENT_NOT_FOUND`, and expanded conformance vectors |
| 2026-02-07 | 0.31 | Nowa | Resolved CAP-vs-payment-control direction conflict, made idempotency rules explicitly apply to CAP carriage, and added RFC 004 descriptor/schema integrity binding for payment capability interop |
| 2026-02-07 | 0.32 | Nowa | Made CAP-path delegation failure mapping explicit (`3004`), split CAP descriptor integrity failures into deterministic `3001` vs `5002`, and aligned conformance wording for optional CAP path |
| 2026-02-07 | 0.33 | Nowa | Aligned CAP error precedence with RFC 004 validation order, defined CAP session context source-of-truth and mismatch handling, and made pre-execution ERROR vs post-execution CAP_RESULT failure split explicit |
| 2026-02-07 | 0.34 | Nowa | Split CAP descriptor integrity vector into deterministic `3001` and `5002` cases and added minimal byte-level error-code checks (`3001`/`3004`/`5002`) |
