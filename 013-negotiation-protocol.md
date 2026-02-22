# RFC 013: Negotiation Protocol

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-22
**Updated**: 2026-02-22
**Version**: 0.1

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (principal binding)
- RFC 003: Relay and Store-and-Forward (delivery and retries)
- RFC 004: Capability Schema Registry and Compatibility (optional negotiation capability invocation)
- RFC 005: Delegation Credentials and Authorization (delegated negotiation boundary)
- RFC 006: Session Protocol (stateful negotiation workflows)
- RFC 007: Agent Payment Protocol (terms for payment negotiation)
- RFC 012: Task Protocol (terms for task negotiation)

---

## Abstract

This RFC defines multi-round negotiation semantics for AMP agents. It standardizes proposal, counter-proposal, acceptance, rejection, and withdrawal flows for structured term agreements using signed AMP profile bodies. Negotiation outcomes are referenceable by other protocols (e.g., RFC 007 payment, RFC 012 task) via stable `negotiation_id`.

The Negotiation Protocol is an AMP Standard Profile using `typ = 0x81` with the profile-body format defined in RFC 001 Section 4.3. The profile name is `xyz.agentries.negotiation` and uses the `xyz.agentries.*` namespace.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
   2.1 Terminology
   2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Negotiation Data Model
   4.1 Lifecycle States
   4.2 Identifiers
   4.3 Negotiation Core CDDL
5. Negotiation Protocol Semantics
   5.1 Message Type Usage and Dispatch
   5.2 Propose Flow
   5.3 Counter Flow
   5.4 Accept Flow
   5.5 Reject Flow
   5.6 Withdraw Flow
   5.7 Status Query and Status Response
   5.8 Action Direction and Correlation Matrix
   5.9 Idempotency and Replay Rules
   5.10 CAP_INVOKE Interop Profile
   5.11 Negotiation Body CDDL
6. State Machines
7. Error Handling and Retry
8. Versioning and Compatibility
9. Security Considerations
10. Privacy Considerations
11. Implementation Checklist
12. References
Appendix A. Test Vectors
Appendix B. Open Questions
Changelog

---

## 1. Problem Statement and Scope

AMP defines messaging, transport, relay, capability, delegation, and session semantics, but does not define interoperable multi-round negotiation workflows. Without a negotiation layer, agents that need to agree on terms before committing to tasks, payments, or service-level agreements must implement ad-hoc term exchange that is neither referenceable nor verifiable across protocol boundaries.

This RFC defines:
- Signed negotiation workflow bodies for propose, counter, accept, reject, withdraw, and status.
- Deterministic negotiation state transitions with round-numbered multi-round semantics.
- Expiry enforcement on terms to prevent stale agreement acceptance.
- Stable `negotiation_id` referenceable from other protocols (e.g., RFC 007 payment terms, RFC 012 task terms).
- Deterministic negotiation-specific error mapping in the `43xx` range.

This RFC does not define:
- Messaging infrastructure (RFC 001).
- Transport bindings (RFC 002).
- Relay and store-and-forward delivery (RFC 003).
- Payment settlement or capture (RFC 007).
- Task execution or result delivery (RFC 012).
- The semantic content of terms objects; term schemas are domain-specific and out of scope.

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope, signature, and idempotency semantics.
- Applies negotiation dispatch rules in Section 5.1.
- Implements required negotiation action schemas in Section 5.11.
- If CAP path is supported, implements Section 5.10 and CAP mapping schemas in Section 5.11.
- Enforces action direction/correlation and idempotency requirements in Sections 5.8 and 5.9.
- Enforces state transitions in Section 6.
- Uses deterministic error mapping in Section 7.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Negotiation ID | Stable negotiation identifier (`bstr .size 16`) for one negotiation lifecycle. |
| Initiator | Agent that sends the first `propose` action, creating the negotiation. |
| Responder | Agent that receives the initial proposal and may counter, accept, or reject. |
| Round | One proposal-response cycle; monotonic unsigned integer, starts at 1. |
| Terms Object | Structured collection of term entries under negotiation. |
| Term Entry | Single named term with a value and optional constraints. |
| Terminal State | A negotiation state from which no further transitions are possible. |

### 2.2 Role Profiles and MTI Requirements

`Initiator Profile`:
- MUST support `propose`, `withdraw`, and `status_query` actions.
- MUST generate a unique `negotiation_id` per negotiation lifecycle.
- MUST use stable `negotiation_id` per intent and preserve idempotency on retries.
- MUST set `round_num = 1` on the initial `propose`.

`Responder Profile`:
- MUST support `counter`, `accept`, `reject`, and `status_query` actions.
- MUST enforce one negotiation state machine per `negotiation_id`.
- MUST provide `status` responses for active and terminal negotiations.
- MUST validate `round_num` monotonicity on incoming `counter` actions.

`Either Party` (after initial proposal):
- After round 1, either party MAY send `counter`, `accept`, or `reject`.
- Either party MAY send `status_query` at any time.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC uses `typ = 0x81` (registered Standard Profile type code) for negotiation messages.
- Negotiation semantics are encoded in signed `profile-body` fields (`profile`, `action`, `profile_v`, and payload).
- Receivers supporting this profile MUST accept messages via either `typ = 0x81` OR `typ = 0xF0` with matching `body.profile = "xyz.agentries.negotiation"` (dual-accept rule per RFC 001 Section 13.3).
- `body.profile` MUST be `"xyz.agentries.negotiation"` in all negotiation messages. Receivers MUST validate that `body.profile` matches and SHOULD reject mismatches with `4001`.

With RFC 002:
- Transport principal binding remains mandatory.
- Unauthorized principal/from combinations in negotiation operations MUST map to `3001`.

With RFC 003:
- Relay redelivery may produce duplicates; endpoints MUST rely on RFC 001 idempotency.
- Negotiation operations MUST be safe under at-least-once delivery.

With RFC 004:
- Negotiation services MAY be exposed as capabilities.
- If negotiation operations are carried via `CAP_INVOKE`, capability/version negotiation follows RFC 004.
- CAP interop baseline for this RFC is `xyz.agentries.negotiation.workflow:1.0.0` (Section 5.10).

With RFC 005:
- This revision does not define delegation carriage inside negotiation action bodies.
- If delegated negotiation execution is required, implementations MUST use delegated `CAP_INVOKE` path and follow RFC 005.

With RFC 006:
- Negotiation flows MAY be session-scoped.
- If session-scoped, `body.session` requirements and thread rules follow RFC 006.

With RFC 007:
- Payment flows MAY reference a completed negotiation via `terms_id` equal to `negotiation_id`.
- Terms agreed through this protocol MAY include payment-related entries (amount, asset, network) that inform subsequent RFC 007 quote flows.

With RFC 012:
- Task flows MAY reference a completed negotiation via `terms_id` equal to `negotiation_id`.
- Terms agreed through this protocol MAY include task-related entries (scope, deadline, deliverables) that inform subsequent task execution.

---

## 4. Negotiation Data Model

### 4.1 Lifecycle States

Canonical states:

```
PROPOSED   -> COUNTERED / ACCEPTED / REJECTED / EXPIRED / WITHDRAWN
COUNTERED  -> COUNTERED / ACCEPTED / REJECTED / EXPIRED / WITHDRAWN
```

State constraints:
- `counter` is valid only from `PROPOSED` or `COUNTERED`.
- `accept` is valid only from `PROPOSED` or `COUNTERED`.
- `reject` is valid only from `PROPOSED` or `COUNTERED`.
- `withdraw` is valid only from `PROPOSED` or `COUNTERED`.
- `ACCEPTED`, `REJECTED`, `EXPIRED`, and `WITHDRAWN` are terminal states.
- No transitions are permitted from terminal states.

### 4.2 Identifiers

- `negotiation_id`: `bstr .size 16`, created by the initiator at `propose` time. MUST be unique per negotiation lifecycle. MUST remain stable across all actions within one negotiation.
- `round_num`: `uint`, starts at 1 for the initial `propose`. Incremented by 1 on each `counter`. The `round_num` in `accept` or `reject` MUST equal the current round at the time of response.
- `terms_hash`: `bstr`, SHA-256 hash of the deterministic CBOR encoding of the `terms-object` being accepted. Used in `accept` to bind agreement to specific terms content.

### 4.3 Negotiation Core CDDL

```cddl
negotiation-id = bstr .size 16
unix-ms = uint
did = tstr
semver = tstr

negotiation-status =
  "proposed" / "countered" / "accepted" / "rejected" / "expired" / "withdrawn"

term-entry = {
  "key": tstr,                    ; term name (e.g., "price", "deadline", "scope")
  "value": any,                   ; term value (domain-specific)
  ? "constraints": any            ; optional constraints on the term (domain-specific)
}

terms-object = {
  "terms": [+ term-entry],        ; one or more term entries
  ? "expires_at": unix-ms,        ; expiry timestamp for these terms (milliseconds)
  ? "context": tstr               ; human-readable or machine-readable context string
}

negotiation-record = {
  "negotiation_id": negotiation-id,
  "initiator": did,
  "responder": did,
  "status": negotiation-status,
  "round_num": uint,
  "current_terms": terms-object,
  "created_at": unix-ms,
  "updated_at": unix-ms
}

negotiation-session-context = {
  "session_id": bstr .size 16,
  "session_scope": true
}

negotiation-base = {
  "profile": "xyz.agentries.negotiation",
  "profile_v": "1.0.0",
  "action": tstr,
  "negotiation_id": negotiation-id,
  ? "session": negotiation-session-context
}
```

Notes:
- `negotiation-record` is a logical data model for implementation state tracking. It is not transmitted on the wire as a single message.
- `negotiation-base` defines the common fields present in all negotiation action bodies. All fields within `negotiation-base` are inside signed `body` and therefore integrity-protected.
- `term-entry` is intentionally generic. Domain-specific term vocabularies (e.g., payment terms, task terms, SLA terms) are defined by consuming protocols or by mutual agreement between agents.

---

## 5. Negotiation Protocol Semantics

### 5.1 Message Type Usage and Dispatch

- Negotiation messages MUST use `typ = 0x81` when the peer declared this profile with `typ` in HELLO negotiation. If the peer did not declare `typ`, or no HELLO was exchanged, the sender MUST use `typ = 0xF0` with `body.profile = "xyz.agentries.negotiation"` (per RFC 001 Section 13.3 typ selection rule).
- Receivers supporting this profile MUST accept messages via either `typ = 0x81` OR `typ = 0xF0` with matching `body.profile = "xyz.agentries.negotiation"` (dual-accept rule per RFC 001 Section 13.3).
- A message is treated as a negotiation message only when `typ` is `0x81` or (`typ` is `0xF0` and signed body contains `"profile": "xyz.agentries.negotiation"`).
- For negotiation messages, `action` and `negotiation_id` are REQUIRED; missing or invalid fields MUST be rejected as `1001`.
- `body.profile` MUST be `"xyz.agentries.negotiation"`. Mismatch between `typ = 0x81` and a different `body.profile` value MUST be rejected with `4001`.
- `body.profile_v` MUST be a supported profile version. Unsupported versions MUST be rejected with `4006`.
- Unknown `action` values in negotiation messages MUST be rejected with `4305`.
- Messages without `"profile": "xyz.agentries.negotiation"` in signed body are not parsed as negotiation messages by this RFC.

### 5.2 Propose Flow

The initiator sends `action: "propose"` to begin a negotiation.

Semantics:
- The initiator MUST generate a unique `negotiation_id` for this negotiation.
- The initiator MUST set `round_num = 1`.
- The `terms` field MUST contain at least one `term-entry`.
- The `responder` field MUST identify the DID of the intended counterparty.
- If `terms.expires_at` is present, it MUST be a future timestamp. Receivers MUST reject proposals with expired `terms.expires_at` with `4304`.
- On receipt, the responder creates a negotiation state machine for this `negotiation_id` in state `PROPOSED`.
- If a `negotiation_id` already exists and the incoming `propose` is a semantic duplicate (same `negotiation_id`, `round_num`, and terms content), the receiver MUST return an idempotent response.
- If a `negotiation_id` already exists but the incoming `propose` has conflicting content, the receiver MUST reject with `4001`.

### 5.3 Counter Flow

Either party sends `action: "counter"` to modify the proposed terms.

Semantics:
- `counter` is valid only when the negotiation is in state `PROPOSED` or `COUNTERED`. Invalid state transitions MUST be rejected with `4302`.
- The sender MUST increment `round_num` by exactly 1 from the current round. Mismatched `round_num` MUST be rejected with `4303`.
- The `terms` field MUST contain the complete replacement terms (not a delta). Receivers MUST treat the counter terms as the new current terms.
- If `terms.expires_at` is present, it MUST be a future timestamp.
- On receipt, the negotiation transitions to state `COUNTERED` with the new `round_num` and terms.
- After round 1, either party (initiator or responder) MAY send a `counter`.

### 5.4 Accept Flow

Either party sends `action: "accept"` to accept the current terms.

Semantics:
- `accept` is valid only when the negotiation is in state `PROPOSED` or `COUNTERED`. Invalid state transitions MUST be rejected with `4302`.
- The `round_num` in the `accept` MUST equal the current `round_num` of the negotiation. Mismatched `round_num` MUST be rejected with `4303`.
- The `terms_hash` field MUST contain the SHA-256 hash of the deterministic CBOR encoding (RFC 8949 Section 4.2) of the `terms-object` being accepted.
- On receipt, the receiver MUST verify that `terms_hash` matches the hash of the current terms. If the hash does not match, the receiver MUST reject with `4001`.
- On successful acceptance, the negotiation transitions to terminal state `ACCEPTED`.
- The accepted terms become the final agreement and are referenceable by other protocols via `negotiation_id`.

### 5.5 Reject Flow

Either party sends `action: "reject"` to decline the current terms.

Semantics:
- `reject` is valid only when the negotiation is in state `PROPOSED` or `COUNTERED`. Invalid state transitions MUST be rejected with `4302`.
- The sender MAY include `reason_code` and/or `reason` fields to explain the rejection.
- On receipt, the negotiation transitions to terminal state `REJECTED`.
- No further actions are valid on this `negotiation_id` after rejection.

### 5.6 Withdraw Flow

The initiator sends `action: "withdraw"` to unilaterally cancel the negotiation.

Semantics:
- `withdraw` is valid only when the negotiation is in state `PROPOSED` or `COUNTERED`. Invalid state transitions MUST be rejected with `4302`.
- Only the initiator of the negotiation (the agent that sent the original `propose`) MAY send `withdraw`. A `withdraw` from any other party MUST be rejected with `3001`.
- The sender MAY include a `reason` field.
- On receipt, the negotiation transitions to terminal state `WITHDRAWN`.
- No further actions are valid on this `negotiation_id` after withdrawal.

### 5.7 Status Query and Status Response

Either party sends `action: "status_query"` to request the current negotiation state.

Semantics:
- `status_query` is valid at any time for any `negotiation_id` known to the receiver.
- If `negotiation_id` is unknown to the receiver, the receiver MUST reject with `4301`.
- The receiver responds with `action: "status"` containing the current `status`, `round_num`, optional `current_terms`, and `updated_at`.
- `status_query` is read-only and has no effect on negotiation state.
- The `status` response MUST set envelope `reply_to` to the triggering `status_query` message `id`.

### 5.8 Action Direction and Correlation Matrix

The following matrix is normative:

| Request `action` | Request sender -> receiver | Response `action` | `reply_to` requirement |
|------------------|----------------------------|-------------------|------------------------|
| `propose` | initiator -> responder | `counter` / `accept` / `reject` | response `reply_to` MUST reference triggering `propose` message `id` |
| `counter` | either -> either | `counter` / `accept` / `reject` | response `reply_to` MUST reference triggering `counter` message `id` |
| `accept` | either -> either | (none — terminal) | N/A |
| `reject` | either -> either | (none — terminal) | N/A |
| `withdraw` | initiator -> responder | (none — terminal) | N/A |
| `status_query` | either -> either | `status` | `status` `reply_to` MUST reference triggering `status_query` message `id` |

Rules:
- This section applies only to negotiation messages parsed by Section 5.1 (`typ = 0x81` or dual-accept `0xF0` with signed body `profile = "xyz.agentries.negotiation"`).
- CAP carriage uses `CAP_INVOKE`/`CAP_RESULT` semantics in Section 5.10 and RFC 004.
- `propose` MUST originate from the initiator. Receivers MUST reject `propose` from a non-initiator party with `3001`.
- `withdraw` MUST originate from the initiator. Receivers MUST reject `withdraw` from a non-initiator party with `3001`.
- `counter`, `accept`, and `reject` after round 1 MAY originate from either party.
- `status_query` MAY originate from either party.
- Unsolicited `status` responses (missing or invalid `reply_to`) MUST be rejected with `4001`.
- Section 5.8 direction rules MUST NOT be applied to CAP envelope types; CAP carriage direction is governed by RFC 004.

### 5.9 Idempotency and Replay Rules

Negotiation logic MUST remain safe under RFC 003 at-least-once delivery.

Operation idempotency key:
- `op_key = (negotiation_id, action, round_num, from_did)`.

Rules:
- This section applies to both profile-body carriage (Section 5.1) and CAP carriage (Section 5.10).
- In CAP carriage, `negotiation_id`, `action`, and `round_num` are read from `CAP_INVOKE.params`.
- Replaying a request with same semantic body under the same `op_key` MUST return a deterministic equivalent result and MUST NOT create duplicate lifecycle transitions.
- Replaying with same `op_key` but conflicting semantic body (for example changed `terms`, counterparty fields, or `terms_hash`) MUST be rejected with `4001`.
- `propose` replay after already `PROPOSED` with same semantic body MUST return prior equivalent acknowledgment.
- `accept` replay after already `ACCEPTED` MUST return prior terminal-equivalent result.
- `reject` replay after already `REJECTED` MUST return prior terminal-equivalent result.
- `withdraw` replay after already `WITHDRAWN` MUST return prior terminal-equivalent result.
- `status_query` is read-only and MAY be retried freely; unknown `negotiation_id` MUST map to `4301`.

### 5.10 CAP_INVOKE Interop Profile

This section defines the RFC 013 capability interoperability baseline for RFC 004 invocation path.

Capability identity:
- `id = "xyz.agentries.negotiation.workflow:1.0.0"`.

Rules:
- When using CAP path, `CAP_INVOKE` MUST target the capability ID above.
- `CAP_INVOKE.params` MUST contain one request action body from this RFC with `profile = "xyz.agentries.negotiation"` and `profile_v = "1.0.0"`.
- Allowed request actions in CAP path: `propose`, `counter`, `accept`, `reject`, `withdraw`, `status_query`.
- `CAP_RESULT(status="success").result` MUST contain one corresponding response action body from this RFC.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005 before negotiation execution.
- Invalid/unsupported delegation evidence in CAP negotiation path MUST fail with `3004` (RFC 004/005).
- Section 5.8 profile-body direction rules MUST NOT be applied to CAP envelope types.
- Providers supporting this capability MUST publish an RFC 004-compliant descriptor for `xyz.agentries.negotiation.workflow:1.0.0`.
- Descriptor/schema integrity verification (hash/signature/trust profile behavior) MUST follow RFC 004 Sections 4.2 and 5.2 before schema validation/execution.
- Descriptor input schema MUST correspond to `app-cap-invoke-params`; success result schema MUST correspond to `app-cap-result-success`.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path (validation/authorization/compatibility/schema) MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

### 5.11 Negotiation Body CDDL

```cddl
negotiation-action =
  "propose" / "counter" / "accept" / "reject" / "withdraw" /
  "status_query" / "status"

propose-body = {
  negotiation-base,
  "action": "propose",
  "responder": did,
  "terms": terms-object,
  "round_num": 1
}

counter-body = {
  negotiation-base,
  "action": "counter",
  "terms": terms-object,
  "round_num": uint .gt 1         ; round_num > 1, monotonically increasing
}

accept-body = {
  negotiation-base,
  "action": "accept",
  "round_num": uint,
  "terms_hash": bstr               ; SHA-256 of deterministic CBOR of accepted terms-object
}

reject-body = {
  negotiation-base,
  "action": "reject",
  ? "reason_code": uint,
  ? "reason": tstr
}

withdraw-body = {
  negotiation-base,
  "action": "withdraw",
  ? "reason": tstr
}

status-query-body = {
  negotiation-base,
  "action": "status_query"
}

status-body = {
  negotiation-base,
  "action": "status",
  "status": negotiation-status,
  "round_num": uint,
  ? "current_terms": terms-object,
  "updated_at": unix-ms
}

app-capability-id = "xyz.agentries.negotiation.workflow:1.0.0"

app-cap-invoke-params =
  propose-body /
  counter-body /
  accept-body /
  reject-body /
  withdraw-body /
  status-query-body

app-cap-result-success =
  status-body
```

---

## 6. State Machines

The following state machine is normative:

```
NONE
  -> PROPOSED              (propose received/sent)

PROPOSED
  |-- counter  -> COUNTERED     (round_num incremented)
  |-- accept   -> ACCEPTED      (terminal)
  |-- reject   -> REJECTED      (terminal)
  |-- expiry   -> EXPIRED       (terminal; terms.expires_at elapsed)
  +-- withdraw -> WITHDRAWN     (terminal; initiator only)

COUNTERED
  |-- counter  -> COUNTERED     (round_num incremented)
  |-- accept   -> ACCEPTED      (terminal)
  |-- reject   -> REJECTED      (terminal)
  |-- expiry   -> EXPIRED       (terminal; terms.expires_at elapsed)
  +-- withdraw -> WITHDRAWN     (terminal; initiator only)

ACCEPTED / REJECTED / EXPIRED / WITHDRAWN
  -> TERMINAL (no further transitions)
```

Expiry processing:
- If `terms.expires_at` is set on the current terms and the timestamp has elapsed, the negotiation MUST be treated as `EXPIRED`.
- Implementations MUST check expiry before processing any incoming action. If the negotiation has expired, incoming `counter`, `accept`, or `reject` actions MUST be rejected with `4304`.
- Expiry MAY be detected lazily (on next interaction) or eagerly (via background timer). Either approach is conformant as long as no post-expiry actions are accepted.

Round number invariants:
- `round_num` MUST start at 1 (set by `propose`).
- Each `counter` MUST increment `round_num` by exactly 1 from the current value.
- `accept` and `reject` MUST carry a `round_num` equal to the current negotiation round.
- Incoming actions with incorrect `round_num` MUST be rejected with `4303`.

---

## 7. Error Handling and Retry

This RFC reuses RFC 001 error model and introduces negotiation-specific business codes in the `43xx` range.

Deterministic precedence:
- Parse/shape/type failures -> `1001`.
- Unsupported `profile_v` -> `4006`.
- Unknown profile (`body.profile`) -> `4005`.
- Authorization identity/policy failure -> `3001`.
- CAP pre-resolution/coarse policy denial -> `3001` (RFC 004 validation order).
- CAP delegation evidence failure (after coarse auth checks) -> `3004`.
- CAP descriptor signature/trust-profile verification failure -> `3001`.
- Negotiation semantic/request-shape conflicts -> `4001`.
- Negotiation state/business failures -> `43xx`.
- CAP descriptor/schema artifact unavailable or integrity source unavailable -> `5002`.
- Transient backend failures -> `500x`.

| Condition | Code | Name | Retry |
|-----------|------|------|-------|
| Malformed negotiation body | `1001` | INVALID_MESSAGE | No |
| Unsupported profile version (`body.profile_v`) | `4006` | PROFILE_VERSION_UNSUPPORTED | No |
| Unknown profile (`body.profile` not recognized) | `4005` | UNKNOWN_PROFILE | No |
| Unauthorized negotiation actor | `3001` | UNAUTHORIZED | No |
| CAP pre-resolution/coarse policy denial | `3001` | UNAUTHORIZED | No |
| CAP delegation evidence invalid | `3004` | DELEGATION_INVALID | No |
| CAP descriptor signature required but missing/invalid | `3001` | UNAUTHORIZED | No |
| Profile/action field mismatch or conflicting replay payload | `4001` | BAD_REQUEST | No |
| CAP session context mismatch (envelope vs payload) | `4001` | BAD_REQUEST | No |
| Negotiation not found (`negotiation_id` unknown) | `4301` | NEGOTIATION_NOT_FOUND | No |
| Invalid state transition | `4302` | INVALID_NEGOTIATION_STATE | No |
| Round number mismatch | `4303` | ROUND_MISMATCH | No |
| Terms expired (`terms.expires_at` elapsed) | `4304` | TERMS_EXPIRED | No |
| Unknown negotiation action (`body.action` not recognized) | `4305` | UNKNOWN_NEGOTIATION_ACTION | No |
| CAP descriptor/schema artifact unavailable | `5002` | UNAVAILABLE | Yes |
| Internal negotiation engine failure | `5001` | INTERNAL_ERROR | Yes |

Registry note:
- `43xx` negotiation codes MUST be registered via RFC 001 Section 17 process before status advances beyond Draft.

---

## 8. Versioning and Compatibility

Version dimensions:
- AMP envelope version `v` remains governed by RFC 001.
- Profile version is `profile_v` in the signed body, currently fixed at `"1.0.0"`.

Compatibility rules:
- Unknown required fields MUST fail with `1001`.
- Unknown optional fields MAY be ignored unless security-sensitive.
- Backward-compatible extensions MUST use optional fields only.
- Major version changes (`profile_v` major increment) indicate breaking changes and MUST be negotiated via HELLO profile matching (RFC 001 Section 13.3).
- Minor and patch version changes MUST be backward compatible within the same major version.

Profile negotiation:
- Implementations SHOULD declare `xyz.agentries.negotiation` in HELLO `profiles` field for connection-time compatibility discovery.
- If the peer declares `typ = 0x81` for this profile in HELLO, senders MUST use `typ = 0x81`.
- If the peer does not declare `typ` or no HELLO was exchanged, senders MUST use `typ = 0xF0` (Private Profile fallback).
- Receivers MUST accept both `typ = 0x81` and `typ = 0xF0` with matching `body.profile` (dual-accept rule).

HELLO profile descriptor example:

```cbor-diag
{
  "name": "xyz.agentries.negotiation",
  "version": "1.0.0",
  "typ": 129,
  "version_range": ">=1.0.0 <2.0.0",
  "required": false,
  "depends": []
}
```

---

## 9. Security Considerations

- **Replay safety**: Implementations MUST enforce RFC 001 idempotency with stable `negotiation_id` and `round_num` semantics. The idempotency key `(negotiation_id, action, round_num, from_did)` prevents duplicate state transitions under at-least-once delivery.
- **Terms tamper protection**: Terms are carried in signed `body` and therefore integrity-protected by AMP envelope signatures. Receivers MUST verify signatures before processing terms content. The `terms_hash` in `accept` provides additional binding to specific terms content.
- **State machine integrity**: Implementations MUST enforce deterministic state transitions as defined in Section 6. Race conditions where both parties simultaneously send conflicting terminal actions (e.g., both send `accept` and `reject`) MUST be resolved by the state machine owner (the agent maintaining the negotiation record) accepting the first valid transition and rejecting subsequent conflicting actions with `4302`.
- **Expiry enforcement**: Implementations MUST check `terms.expires_at` before processing actions to prevent stale acceptances. Clock skew tolerance SHOULD follow RFC 001 Section 8.3 MAX_CLOCK_SKEW policy.
- **Principal binding**: Principal/from binding from RFC 002 applies to all negotiation operations. Unauthorized senders MUST be rejected with `3001`.
- **Withdrawal restriction**: Only the initiator MAY withdraw a negotiation. This prevents responders from unilaterally canceling negotiations where they could instead reject.
- **Terms content security**: This RFC does not define term entry schemas. Implementations processing domain-specific terms MUST validate term content against expected schemas to prevent injection or unexpected semantic interpretation.

---

## 10. Privacy Considerations

- Negotiation metadata (term keys, values, round counts, timing) can reveal business relationships, pricing strategies, and negotiation patterns.
- Implementations SHOULD minimize retention of sensitive negotiation details after terminal states are reached.
- Logs SHOULD prefer IDs and status codes over full terms content where possible.
- Implementations SHOULD consider that the number of counter-rounds and timing patterns may reveal negotiation strategy to observers with access to message metadata.
- Session-scoped negotiations (via RFC 006) inherit the privacy properties and considerations of the session protocol.
- Relay operators (RFC 003) MUST NOT parse or interpret negotiation bodies for routing decisions, but message size and frequency patterns may still be observable.

---

## 11. Implementation Checklist

- [ ] Implement negotiation dispatch guard (Section 5.1), including dual-accept for `typ = 0x81` and `typ = 0xF0`.
- [ ] Implement `propose`, `counter`, `accept`, `reject`, `withdraw`, `status_query`, and `status` action handlers.
- [ ] Implement action direction/correlation matrix (Section 5.8).
- [ ] Implement idempotency/replay rules (Section 5.9).
- [ ] Implement CAP interop profile (Section 5.10) if CAP path is supported.
- [ ] Implement all Section 5.11 action schemas.
- [ ] Enforce lifecycle transitions (Section 6), including expiry checking.
- [ ] Enforce round number monotonicity and matching.
- [ ] Compute and verify `terms_hash` using SHA-256 of deterministic CBOR encoding.
- [ ] Enforce deterministic error mapping (Section 7).
- [ ] Ensure idempotent behavior on repeated negotiation action retries.
- [ ] Declare `xyz.agentries.negotiation` in HELLO `profiles` if profile negotiation is supported.
- [ ] Add conformance tests from Appendix A.

---

## 12. References

### 12.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8949: CBOR (Concise Binary Object Representation)
- RFC 8610: CDDL (Concise Data Definition Language)

### 12.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 004: Capability Schema Registry and Compatibility
- RFC 005: Delegation Credentials and Authorization
- RFC 006: Session Protocol
- RFC 007: Agent Payment Protocol
- RFC 012: Task Protocol

---

## Appendix A. Test Vectors

### A.1 Propose to Accept Positive

Input:
- Initiator sends `propose` with `negotiation_id = 0x0102...10` (16 bytes), `round_num = 1`, terms containing `[{"key": "price", "value": 100}]`, `responder = "did:web:responder.example"`.
- Responder sends `accept` with same `negotiation_id`, `round_num = 1`, `terms_hash` = SHA-256 of deterministic CBOR of terms-object.

Expected:
- State transitions: `NONE -> PROPOSED -> ACCEPTED`.
- `ACCEPTED` is terminal; no further transitions.

### A.2 Propose to Reject Positive

Input:
- Initiator sends `propose` with valid terms.
- Responder sends `reject` with same `negotiation_id`, optional `reason = "Terms unacceptable"`.

Expected:
- State transitions: `NONE -> PROPOSED -> REJECTED`.
- `REJECTED` is terminal; no further transitions.

### A.3 Multi-Round Counter Positive

Input:
- Initiator sends `propose` with `round_num = 1`, terms `[{"key": "price", "value": 100}]`.
- Responder sends `counter` with `round_num = 2`, terms `[{"key": "price", "value": 80}]`.
- Initiator sends `counter` with `round_num = 3`, terms `[{"key": "price", "value": 90}]`.
- Responder sends `accept` with `round_num = 3`, `terms_hash` matching round 3 terms.

Expected:
- State transitions: `NONE -> PROPOSED -> COUNTERED -> COUNTERED -> ACCEPTED`.
- Final agreed terms: `[{"key": "price", "value": 90}]`.

### A.4 Withdraw Positive

Input:
- Initiator sends `propose` with valid terms.
- Before responder acts, initiator sends `withdraw` with `reason = "Changed requirements"`.

Expected:
- State transitions: `NONE -> PROPOSED -> WITHDRAWN`.
- `WITHDRAWN` is terminal; no further transitions.

### A.5 Accept After Counter Positive

Input:
- Initiator sends `propose` with `round_num = 1`.
- Responder sends `counter` with `round_num = 2`, modified terms.
- Initiator sends `accept` with `round_num = 2`, `terms_hash` matching counter terms.

Expected:
- State transitions: `NONE -> PROPOSED -> COUNTERED -> ACCEPTED`.
- Initiator accepted the responder's counter-proposal.

### A.6 Invalid State Transition Negative (4302)

Input:
- Initiator sends `propose`, responder sends `accept`.
- After `ACCEPTED`, initiator sends `counter` with `round_num = 2`.

Expected:
- Reject with `4302 INVALID_NEGOTIATION_STATE`.
- Byte-level check: error code `4302` is CBOR uint bytes `19 10 ce`.

### A.7 Round Number Mismatch Negative (4303)

Input:
- Initiator sends `propose` with `round_num = 1`.
- Responder sends `counter` with `round_num = 3` (skipping round 2).

Expected:
- Reject with `4303 ROUND_MISMATCH`.
- Byte-level check: error code `4303` is CBOR uint bytes `19 10 cf`.

### A.8 Terms Expired Negative (4304)

Input:
- Initiator sends `propose` with `terms.expires_at = 1700000000000` (past timestamp).
- Responder attempts `accept`.

Expected:
- Reject with `4304 TERMS_EXPIRED`.
- Byte-level check: error code `4304` is CBOR uint bytes `19 10 d0`.

### A.9 Negotiation Not Found Negative (4301)

Input:
- Either party sends `status_query` with `negotiation_id` that has no known lifecycle record.

Expected:
- Reject with `4301 NEGOTIATION_NOT_FOUND`.
- Byte-level check: error code `4301` is CBOR uint bytes `19 10 cd`.

### A.10 Duplicate Propose Idempotent Positive

Input:
- Initiator sends `propose` with `negotiation_id = X`, `round_num = 1`, terms `T`.
- Initiator replays same `propose` with identical `negotiation_id = X`, `round_num = 1`, terms `T`.

Expected:
- Deterministic idempotent outcome; no duplicate state transition.
- Second `propose` returns equivalent acknowledgment to the first.

---

## Appendix B. Open Questions

1. **Maximum round limit**: Should there be a configurable or protocol-defined maximum number of negotiation rounds to prevent unbounded back-and-forth? If so, what error code should be returned when the limit is exceeded?

2. **Partial term acceptance**: Should the protocol support accepting some terms while countering others within a single action, or should partial acceptance be modeled as a `counter` with the accepted terms unchanged?

3. **Third-party mediator/arbitrator roles**: Should the protocol define additional roles beyond initiator and responder, such as a mediator agent that can facilitate negotiation or an arbitrator that can impose a binding resolution?

4. **Terms schema registry**: Should there be a standardized vocabulary of common term keys (e.g., `"price"`, `"deadline"`, `"scope"`) with defined semantics, or should term keys remain entirely domain-specific?

5. **Negotiation groups**: Should the protocol support linked negotiations where the outcome of one negotiation depends on or is contingent upon the outcome of another?

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-22 | 0.1 | Nowa | Initial draft: profile-body negotiation protocol with propose/counter/accept/reject/withdraw/status flows, deterministic state machine, `43xx` error codes, CAP interop profile, round-numbered multi-round semantics, `terms_hash` binding, and 10 test vectors |
