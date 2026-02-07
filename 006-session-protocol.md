# RFC 006: Session Protocol (State + Recovery)

**Status**: Draft
**Authors**: Ryan Cooper, Nowa
**Created**: 2026-02-04
**Updated**: 2026-02-07
**Version**: 0.8

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)
- RFC 004: Capability Schema Registry & Compatibility

**Related:**
- RFC 002: Transport Bindings (carrier + principal binding)
- RFC 003: Relay & Store-and-Forward (delivery behavior)
- RFC 005: Delegation Credentials & Authorization
- RFC 008: Agent Discovery & Directory

---

## Abstract

This RFC defines a deterministic session model for AMP stateful interactions, including session establishment, state transitions, provisional response behavior, and recovery after transient failures. It standardizes two interoperable thread modes: `coupled` (MTI baseline, `thread_id == session_id`) and optional `independent` (thread as sub-conversation key under one session). It also defines capability pinning and recovery safety guarantees.

---

## Table of Contents

1. Scope and Non-Goals
2. Conformance and Profiles
2.1 Terminology
2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Session Model and Identifiers
4.1 Thread and Session Alignment (Normative)
4.2 Session Record Model
4.3 Session CDDL
4.4 Session Context Carriage (Normative)
5. Session Control Semantics
5.1 Message Type Usage
5.2 Session Init
5.3 Session Accept / Reject
5.4 Session Update
5.5 Session Suspend / Resume
5.6 Session Close
5.7 Control Body CDDL
6. Provisional Response Semantics
6.1 PROCESSING
6.2 PROGRESS
6.3 INPUT_REQUIRED
6.4 Provisional Body CDDL
7. Recovery Protocol
7.1 Resume Request / Response
7.2 Replay and Idempotency
7.3 Capability Pinning and Delegation Cache Rules
8. State Machines
8.1 Session State Machine
8.2 Session-Scoped Request Handling State Machine
9. Error Handling and Retry
10. Versioning and Compatibility
11. Security Considerations
12. Privacy Considerations
13. Implementation Checklist
14. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions

---

## 1. Scope and Non-Goals

This RFC defines:
- Session identity and lifecycle semantics for stateful AMP interactions.
- Session control operations over existing AMP message types.
- Provisional response behavior in session-scoped flows.
- Recovery semantics after disconnects/restarts.
- Capability pinning and delegation-cache safety during recovery.

This RFC does not define:
- New AMP type codes (RFC 001 owns type code registry).
- Transport framing or endpoint auth behavior (RFC 002).
- Relay queue internals (RFC 003).
- Capability schema negotiation rules (RFC 004).
- Delegation credential semantics (RFC 005).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope/signature semantics.
- Enforces negotiated session scoping rules in Sections 4.1 and 4.4.
- Enforces session state rules in Sections 5-8.
- Uses deterministic error mapping in Section 9.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Session ID | Stable identifier for one stateful interaction context. |
| Session-scoped message | AMP message bound to an active `session_id`; context binding source depends on thread mode (Section 4.4). |
| Session control op | Structured operation in REQUEST/RESPONSE body for session lifecycle management. |
| Session checkpoint | Recovery cursor describing last confirmed progress in a session. |
| Capability pin | Session-local fixed capability ID chosen under RFC 004 rules. |
| Delegation cache fingerprint | Cached hash/fingerprint of validated delegation evidence used in session continuity checks. |
| Thread mode | Session-level rule for `thread_id`: `coupled` or `independent`. |

### 2.2 Role Profiles and MTI Requirements

`Session Client Profile`:
- MUST generate stable `session_id` values.
- MUST support `thread_mode = "coupled"` baseline.
- In `coupled` mode, MUST set `thread_id` on all session-scoped messages and keep `thread_id == session_id`.
- MUST tolerate replay duplicates using RFC 001 message ID idempotency.

`Session Provider Profile`:
- MUST enforce lifecycle transitions in Section 8.
- MUST enforce participant authorization for session operations.
- MUST support `thread_mode = "coupled"` baseline.
- MUST preserve capability pins across recovery unless explicit renegotiation is accepted.

`Session Store Profile` (optional):
- MUST persist session records and checkpoints atomically.
- MUST preserve audit-relevant transitions (`active`, `suspended`, `closed`, `expired`).
- MUST fail closed on inconsistent record writes.

`Thread Independence Profile` (optional extension):
- MAY support `thread_mode = "independent"` under explicit negotiation at session init.
- MUST keep session identity keyed by `session_id` even when `thread_id` varies.
- MUST enforce per-request provisional correlation by `reply_to` and (if present) `thread_id`.
- MUST require signed body session context for session-scoped non-control messages (Section 4.4).

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- RFC 001 defines message envelope, signature, `thread_id`, and provisional type codes.
- RFC 006 defines session semantics over existing types; it does not allocate new type codes.
- Session-scoped provisional messages MUST use `reply_to`; `thread_id` consistency is mode-specific (Section 4.1 and Section 6).

With RFC 002:
- Transport handshake/framing/auth are defined in RFC 002.
- Session semantics MUST be transport-agnostic.
- Principal/from binding requirements continue to apply for session-scoped messages.

With RFC 003:
- Relays treat session bodies as opaque AMP payload.
- Store-and-forward replay MAY cause duplicate session-scoped messages; endpoints MUST rely on RFC 001 idempotency.

With RFC 004:
- Session MAY pin negotiated capability IDs for a session scope.
- Recovery MUST NOT change pinned capability version without explicit session update/renegotiation.

With RFC 005:
- Session MAY cache validated delegation fingerprints.
- Recovery before privileged operations MUST re-check delegation revocation freshness per RFC 005 policy.

With RFC 008:
- Discovery MAY expose session endpoint hints.
- Discovery metadata MUST NOT override signed session control messages.

---

## 4. Session Model and Identifiers

### 4.1 Thread and Session Alignment (Normative)

Session identity rule:
- Session ID is represented as `session_id` and MUST be encoded as 16-byte binary (`bstr .size 16`).

Thread mode rules:
- `coupled` mode (MTI baseline):
  - For any session-scoped message, envelope `thread_id` MUST be present and MUST equal `session_id`.
  - If body contains `session_id`, it MUST equal envelope `thread_id`; mismatch MUST be rejected with `4001`.
- `independent` mode (optional profile):
  - Session scope is determined by signed session context (`session_id`) while `thread_id` acts as optional sub-thread key.
  - Envelope `thread_id` MAY be absent.
  - If present, `thread_id` MAY differ from `session_id`.
  - For a given in-flight request, provisional/terminal replies MUST preserve triggering `thread_id` if one was present.

### 4.2 Session Record Model

A provider/session store tracks at minimum:
- Identity: `session_id`, `owner`, `participants`.
- Lifecycle: `status`, `created_at`, `expires_at`, optional `suspended_at`, `closed_at`.
- Recovery: `checkpoint`, `last_activity_at`.
- Interop safety: `pinned_capabilities`, optional `delegation_cache` fingerprints.

### 4.3 Session CDDL

```cddl
did = tstr
unix-ms = uint
session-id = bstr .size 16
message-id = bstr .size 16

session-status = "pending" / "active" / "suspended" / "closing" / "closed" / "expired" / "failed"

session-checkpoint = {
  ? "last_seen_msg_id": message-id,
  ? "last_activity_at": unix-ms
}

capability-pin-map = {
  * tstr => tstr ; key=capability name/alias, value=capability-id
}

delegation-cache-entry = {
  "fingerprint": bstr,
  "updated_at": unix-ms,
  ? "max_age_s": uint
}

session-context = {
  "session_id": session-id,
  "session_scope": true
}

session-record = {
  "sess_v": 1,
  "session_id": session-id,
  "owner": did,
  "participants": [+ did],
  "status": session-status,
  "created_at": unix-ms,
  "expires_at": unix-ms,
  "last_activity_at": unix-ms,
  ? "suspended_at": unix-ms,
  ? "closed_at": unix-ms,
  ? "checkpoint": session-checkpoint,
  ? "pinned_capabilities": capability-pin-map,
  ? "delegation_cache": [* delegation-cache-entry]
}
```

### 4.4 Session Context Carriage (Normative)

This section defines how session identity is bound on session-scoped messages that are not RFC 006 session-control operations.

Dispatch boundary:
- Session-control operations are only `REQUEST`/`RESPONSE` messages carrying `sess_v` (Section 5.1).
- Session-scoped non-control messages (for example `MESSAGE`, `CAP_INVOKE`, `CAP_RESULT`, provisional types) MUST follow this section.
- A non-control message enters RFC 006 session-scoped validation when ANY of the following is true:
  - signed `body.session.session_scope = true`; or
  - message is provisional (`PROCESSING`/`PROGRESS`/`INPUT_REQUIRED`) and `reply_to` resolves to an in-flight session-scoped request.
- Messages that do not meet the above conditions are treated as non-session application payload and MUST NOT mutate session state.
- Envelope `thread_id` matching an active `session_id` MUST NOT, by itself, establish session scope.
- If envelope `thread_id` matches an active `session_id` but `body.session.session_scope` is absent, receiver SHOULD reject with `4001` to avoid ambiguous interpretation.

`coupled` mode:
- Session-scoped non-control messages MUST include signed `body.session` context with `session_scope = true`.
- Envelope `thread_id == session_id` remains required MTI binding.
- Signed `body.session.session_id` MUST equal envelope `thread_id`; mismatch MUST be rejected with `4001`.

`independent` mode:
- Session-scoped non-control messages MUST include signed `body.session` context with `session_scope = true`.
- `body.session.session_id` MUST match an active session known to receiver state.
- Envelope `thread_id` MAY be absent; if present it is a sub-thread key and MAY differ from `session_id`.
- For reply/provisional correlation, if triggering request had `thread_id`, reply/provisional MUST preserve that `thread_id`.

Validation and parsing:
- Session-scoped non-control bodies that carry session context MUST be CBOR maps.
- If `body.session` exists but is not a map, or `body.session.session_id` has wrong type/size, or `body.session.session_scope` has wrong type/value, receiver MUST reject with `1001`.
- In `independent` mode, missing required `body.session.session_id` on a message that entered session-scoped validation MUST be rejected with `4001`.
- In `independent` mode, if `thread_id` is present or provisional `reply_to` resolves to a session-scoped request, omission of `body.session.session_id` MUST be rejected with `4001`.
- Missing required `body.session.session_scope = true` on a message that entered session-scoped validation MUST be rejected with `4001`.

---

## 5. Session Control Semantics

### 5.1 Message Type Usage

This RFC reuses existing RFC 001 type codes:
- Control requests MUST use `typ = REQUEST`.
- Control responses MUST use `typ = RESPONSE`.
- Provisional progress signals use `PROCESSING`, `PROGRESS`, `INPUT_REQUIRED` (Section 6).
- Session-scoped non-control message carriage rules are defined in Section 4.4.

Session control operations are encoded in signed body fields (`op`, `sess_v`, payload).

Session-control dispatch rules (normative):
- A message is treated as RFC 006 session-control only when `typ` is `REQUEST`/`RESPONSE` and signed body contains `sess_v`.
- For session-control messages, `op` and `session_id` are REQUIRED; missing/invalid fields MUST be rejected as `1001`.
- `sess_v` is reserved for RFC 006 session-control semantics in `REQUEST`/`RESPONSE` bodies.
- `REQUEST`/`RESPONSE` messages without `sess_v` are not parsed as RFC 006 control operations.

### 5.2 Session Init

Request requirements:
- `op = "init"`, `sess_v = 1`.
- `session_id` MUST be present.
- `thread_mode` MAY be supplied; if omitted, default is `coupled`.
- In `thread_mode = "coupled"`, envelope `thread_id` MUST equal `session_id`.
- `participants` MUST include sender DID.
- `expires_in_ms` MUST be > 0.

Provider behavior:
- Validate sender authorization and participant policy.
- Negotiate `thread_mode` (`coupled` mandatory to support; `independent` optional).
- Create session record in `pending` then `active` on acceptance.
- Return `RESPONSE` with `op = "accept"` or `op = "reject"`.

### 5.3 Session Accept / Reject

`accept` response requirements:
- `reply_to` MUST reference init request `id`.
- Body MUST include `session_id`, `status = "active"`, `expires_at`.
- Body MUST include selected `thread_mode`.

`reject` response requirements:
- `reply_to` MUST reference init request `id`.
- Body MUST include `session_id`, `status = "failed"`, and reason code/detail.

### 5.4 Session Update

Purpose:
- Mutate session metadata (participants, expiry extension, capability renegotiation marker).

Rules:
- `op = "update"` with `session_id`.
- Provider MUST reject updates for terminal sessions (`closed`/`expired`) with `4001`.
- Capability pin changes MUST require explicit `allow_renegotiate = true`.

### 5.5 Session Suspend / Resume

Suspend rules:
- `op = "suspend"` allowed only from `active`.
- Provider sets `status = "suspended"` and `suspended_at`.

Resume rules:
- `op = "resume"` allowed from `suspended` or `active`.
- Provider validates participant authorization and expiry.
- Provider returns recovery metadata (Section 7.1).

### 5.6 Session Close

Close rules:
- `op = "close"` may be initiated by authorized participant.
- Provider transitions `active|suspended|closing -> closed`.
- Repeated close on already `closed` session SHOULD be idempotent success.

### 5.7 Control Body CDDL

```cddl
session-op = "init" / "accept" / "reject" / "update" / "suspend" / "resume" / "close"
thread-mode = "coupled" / "independent"

session-control-base = {
  "sess_v": 1,
  "op": session-op,
  "session_id": session-id
}

session-init-body = {
  session-control-base,
  "op": "init",
  ? "thread_mode": thread-mode,
  "participants": [+ did],
  "expires_in_ms": uint,
  ? "purpose": tstr
}

session-accept-body = {
  session-control-base,
  "op": "accept",
  "thread_mode": thread-mode,
  "status": "active",
  "expires_at": unix-ms,
  ? "checkpoint": session-checkpoint
}

session-reject-body = {
  session-control-base,
  "op": "reject",
  "status": "failed",
  ? "reason_code": uint,
  ? "reason": tstr
}

session-update-body = {
  session-control-base,
  "op": "update",
  ? "expires_in_ms": uint,
  ? "participants": [+ did],
  ? "allow_renegotiate": bool
}

session-suspend-body = {
  session-control-base,
  "op": "suspend",
  ? "reason": tstr
}

session-resume-body = {
  session-control-base,
  "op": "resume",
  ? "checkpoint": session-checkpoint
}

session-close-body = {
  session-control-base,
  "op": "close",
  ? "reason": tstr
}
```

---

## 6. Provisional Response Semantics

### 6.1 PROCESSING

Use when request accepted and work is ongoing.

Rules:
- MUST include `reply_to` of triggering request.
- In `coupled` mode, MUST include `thread_id == session_id`.
- In `independent` mode, if triggering request has `thread_id`, provisional message MUST use the same `thread_id`.

### 6.2 PROGRESS

Use for quantifiable progress.

Rules:
- MUST include `reply_to`.
- `progress_pct` MUST be in `0..100`.
- `progress_pct=100` SHOULD be followed by terminal `RESPONSE`/`CAP_RESULT`/`PROC_*`.

### 6.3 INPUT_REQUIRED

Use when provider cannot proceed without additional client input.

Rules:
- MUST include `reply_to`.
- Session remains `active` unless local policy suspends after timeout.
- If `timeout_ms` elapses without input, provider SHOULD return terminal failure.

### 6.4 Provisional Body CDDL

```cddl
processing-body = {
  ? "session": session-context,
  ? "status_text": tstr,
  ? "eta_ms": uint,
  ? "cancellable": bool
}

progress-body = {
  ? "session": session-context,
  "progress_pct": uint,
  ? "status_text": tstr,
  ? "eta_ms": uint,
  ? "cancellable": bool
}

input-required-body = {
  ? "session": session-context,
  "prompt": tstr,
  ? "options": [* tstr],
  ? "timeout_ms": uint
}
```

---

## 7. Recovery Protocol

### 7.1 Resume Request / Response

Resume request (`REQUEST` + `op="resume"`) MUST include:
- `session_id`.
- In `coupled` mode: aligned `thread_id == session_id`.
- In `independent` mode: `thread_id` is optional and, if present, identifies sub-thread recovery scope.
- Optional `checkpoint` (`last_seen_msg_id`, `last_activity_at`).

Resume response (`RESPONSE`) SHOULD include:
- Current session `status`.
- Effective `checkpoint` accepted by provider.
- Optional replay hint (for example `replay_after_msg_id`).

### 7.2 Replay and Idempotency

- Providers MAY replay uncommitted or uncertain responses after resume.
- Clients MUST deduplicate by RFC 001 message ID.
- Duplicate resume requests with same `id` MUST be idempotent.
- Implementations MUST keep RFC 001 idempotency key semantics: dedupe key is `(sender_did, message_id)`.
- In `independent` mode, implementations MAY additionally index by `(session_id, thread_id)` for local routing/lookup optimization, but MUST NOT replace RFC 001 dedupe key.

### 7.3 Capability Pinning and Delegation Cache Rules

Capability pinning:
- If session has `pinned_capabilities`, recovery MUST keep pinned IDs stable.
- Any pinned capability change requires explicit successful session update with `allow_renegotiate=true`.
- If provider cannot honor pinned version, it MUST reject with `4003`.

Delegation cache safety:
- Session MAY cache delegation fingerprints for performance.
- Before privileged resumed operations, provider MUST re-check revocation freshness per RFC 005 policy.
- Revocation source unavailable in strict mode MUST fail closed with `5002`.
- Revoked/invalid delegation after refresh MUST fail with `3004`.

---

## 8. State Machines

### 8.1 Session State Machine

```
NONE
  -> PENDING               (valid init received)
PENDING
  -> ACTIVE                (accept)
  -> FAILED                (reject)
ACTIVE
  -> SUSPENDED             (suspend)
  -> CLOSING               (close requested)
  -> EXPIRED               (ttl reached)
SUSPENDED
  -> ACTIVE                (resume accepted)
  -> CLOSING               (close requested)
  -> EXPIRED               (ttl reached)
CLOSING
  -> CLOSED                (close finalized)
FAILED / CLOSED / EXPIRED
  -> TERMINAL
```

### 8.2 Session-Scoped Request Handling State Machine

```
REQUEST_RECEIVED
  -> VALIDATING
VALIDATING
  -> REJECTED              (ERROR 1xxx/3xxx/4xxx/5xxx)
  -> EXECUTING
EXECUTING
  -> (optional) PROCESSING/PROGRESS/INPUT_REQUIRED
  -> TERMINAL_RESPONSE
TERMINAL_RESPONSE
  -> DONE
```

---

## 9. Error Handling and Retry

This RFC reuses RFC 001 error code space.

Deterministic precedence:
- Parse/shape/type errors in session-related body fields (session-control body or `body.session` context object) MUST map to `1001`.
- If shape is valid but `sess_v` or `thread_mode` is unsupported, implementation MUST return `1004`.
- After parse/version checks, authorization failures MUST map to `3001` (with non-leaking detail).
- Remaining session semantic/state/correlation failures MUST map to `4001`/`4003`/`3004`/`500x` as applicable.

| Condition | Code | Retry |
|----------|------|-------|
| Malformed session control body | `1001` | No |
| Unsupported `sess_v` | `1004` | No |
| Unsupported `thread_mode` | `1004` | No |
| Malformed `body.session` context object (type/shape/size) | `1001` | No |
| `thread_id` matches active `session_id` but `body.session.session_scope` missing | `4001` | No |
| `session_id` and `thread_id` mismatch in `coupled` mode | `4001` | No |
| `independent` mode reply/provisional `thread_id` mismatch to triggering request | `4001` | No |
| `independent` mode message entered session-scoped validation but missing `body.session.session_id` | `4001` | No |
| Message entered session-scoped validation but missing `body.session.session_scope=true` | `4001` | No |
| Unknown/expired/closed session for requested op | `4001` | No |
| Unauthorized participant/session actor | `3001` | No |
| Pinned capability cannot be preserved on recovery | `4003` | No |
| Delegation invalid after recovery refresh | `3004` | No |
| Revocation source unavailable in strict mode | `5002` | Yes |
| Internal session store/engine failure | `5001` | Yes |

Retry guidance:
- `300x/400x` failures SHOULD NOT be retried without identity/session/payload mutation.
- `500x` failures MAY be retried with bounded exponential backoff.
- Resume retries SHOULD preserve same session context and checkpoint intent.

---

## 10. Versioning and Compatibility

Version dimensions:
- AMP envelope version (`v`) remains governed by RFC 001 negotiation.
- Session control schema version is `sess_v`, currently fixed at `1`.
- Session thread behavior is negotiated by `thread_mode` (`coupled` default, `independent` optional).

Compatibility rules:
- Implementations MUST reject unsupported `sess_v` with `1004`.
- Implementations MUST reject unsupported `thread_mode` with `1004`.
- Unknown optional fields MAY be ignored unless they affect security semantics.
- Backward-compatible additions MUST be optional fields only.

---

## 11. Security Considerations

- Session fixation protection: receiver MUST validate participant membership on every control operation.
- Replay protection: implementations MUST enforce RFC 001 message ID idempotency.
- Session takeover prevention: resume requests MUST pass transport/auth policy and participant checks.
- Capability downgrade prevention: pinned capability IDs MUST remain stable unless explicit renegotiation succeeds.
- Delegation safety: resumed privileged actions MUST re-check revocation freshness in strict mode.
- Provisional spam resistance: endpoints SHOULD rate-limit provisional emissions per session and request.

---

## 12. Privacy Considerations

- Session metadata (`participants`, `purpose`, timing) can reveal workflow structure.
- Implementations SHOULD minimize retention of session history and sensitive state payloads.
- Logs SHOULD prefer message IDs and session IDs over raw body content.
- Recovery traces SHOULD avoid leaking unauthorized session existence.

---

## 13. Implementation Checklist

- Enforce negotiated thread mode rules (`coupled` mandatory, `independent` optional).
- Enforce session context carriage in session-scoped non-control messages (Section 4.4).
- Dispatch RFC 006 parsing only for `REQUEST`/`RESPONSE` bodies carrying `sess_v`.
- Parse and validate all session control operations (`init`, `accept`, `reject`, `update`, `suspend`, `resume`, `close`).
- Enforce lifecycle transitions from Section 8.
- Enforce provisional `reply_to` correlation for session-scoped in-flight requests.
- Preserve capability pins across recovery unless explicit renegotiation is accepted.
- Re-check delegation revocation freshness before resumed privileged operations.
- Emit deterministic errors per Section 9.
- Add conformance tests from Appendix A.

---

## 14. References

### 14.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 004: Capability Schema Registry & Compatibility
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8949: CBOR

### 14.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay & Store-and-Forward
- RFC 005: Delegation Credentials & Authorization
- RFC 008: Agent Discovery & Directory

---

## Appendix A. Minimal Test Vectors

### A.1 Session Init Accept Positive

Input:
- `REQUEST` with valid `session-init-body`, aligned `thread_id/session_id`, `thread_mode=\"coupled\"` (or omitted).

Expected:
- Provider returns `RESPONSE` `op="accept"`, session transitions to `active`.

### A.2 Session ID/Thread ID Mismatch Negative

Input:
- `REQUEST` in `coupled` mode where body `session_id` differs from envelope `thread_id`.

Expected:
- `4001 BAD_REQUEST`.

### A.3 Provisional Reply Correlation Positive

Input:
- Session-scoped long-running request followed by `PROCESSING` and `PROGRESS` carrying correct `reply_to`.

Expected:
- Client accepts provisional updates and awaits terminal response.

### A.4 Provisional Missing Reply_To Negative

Input:
- Session-scoped `PROGRESS` without `reply_to`.

Expected:
- Reject as invalid session/provisional usage (`4001` recommended).

### A.4b Independent Mode Provisional Thread Mismatch Negative

Input:
- Session negotiated with `thread_mode=\"independent\"`.
- Triggering request has `thread_id = T1`.
- `PROGRESS` arrives with `thread_id = T2` for same `reply_to`.

Expected:
- Reject with `4001 BAD_REQUEST`.

### A.5 Resume Positive

Input:
- Valid `resume` from authorized participant with checkpoint.

Expected:
- Session remains/returns `active`, provider returns effective checkpoint.

### A.6 Resume Unknown Session Negative

Input:
- `resume` with unknown session ID.

Expected:
- `4001 BAD_REQUEST` with non-leaking detail.

### A.7 Capability Pin Preservation Positive

Input:
- Session has pinned `org.agentries.code-review:2.1.0`, then resume.

Expected:
- Pinned capability remains unchanged after recovery.

### A.8 Capability Pin Mismatch Negative

Input:
- Resume path attempts to continue with incompatible pinned capability version.

Expected:
- `4003 VERSION_MISMATCH`.

### A.9 Delegation Revocation Refresh Negative

Input:
- Resumed privileged operation where delegation refresh shows revoked credential.

Expected:
- `3004 DELEGATION_INVALID`.

### A.10 Delegation Freshness Source Unavailable (Strict Mode)

Input:
- Resumed privileged operation, delegation source unavailable, strict mode on.

Expected:
- `5002 UNAVAILABLE`.

### A.11 Unauthorized Session Actor Negative

Input:
- Non-participant DID sends `session-update`.

Expected:
- `3001 UNAUTHORIZED`.

### A.12 Session Close Idempotent Positive

Input:
- `close` on already closed session.

Expected:
- Idempotent success response, no invalid transition.

### A.13 Independent Thread Mode Negotiation Positive

Input:
- `session-init-body` requests `thread_mode=\"independent\"`.
- Provider supports independent profile and accepts.

Expected:
- Accept response includes `thread_mode=\"independent\"`.
- Subsequent session-scoped messages may use distinct per-branch `thread_id` values under same `session_id`.
- Those messages carry signed `body.session` with `session_scope=true` and `session_id` for binding.

### A.14 Session-Control Dispatch Guard Positive

Input:
- `REQUEST` body contains business field `op` but no `sess_v`.

Expected:
- Message is processed as non-session application payload (RFC 001 path), not as RFC 006 session-control.
- No RFC 006-specific `1001/1004` rejection is emitted solely due to missing `session_id`.

### A.15 Independent Mode Missing Session Context Negative

Input:
- Session negotiated with `thread_mode=\"independent\"`.
- Session-scoped `PROGRESS` has `reply_to` resolving to an in-flight session request but omits `body.session.session_scope` and `body.session.session_id`.

Expected:
- Reject with `4001 BAD_REQUEST`.
- No implicit fallback to `thread_id`/`reply_to` only as session binding.

### A.16 Malformed Session Context Shape Negative

Input:
- Session-scoped non-control message carries `body.session` as non-map (for example string), or `body.session.session_id` with wrong size, or non-true `session_scope`.

Expected:
- Reject with `1001 INVALID_FORMAT`.

### A.17 Byte-Level Error Code Checks

Input:
- Malformed session context shape case mapped to `1001`.
- Session binding mismatch case mapped to `4001`.

Expected:
- `1001` CBOR uint encoding bytes: `19 03 e9`.
- `4001` CBOR uint encoding bytes: `19 0f a1`.

---

## Appendix B. Open Questions

No open questions in this revision.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-04 | Proposal | Ryan Cooper | Initial proposal outline |
| 2026-02-07 | 0.1 | Nowa | Rewrote RFC 006 into normative draft structure with conformance profiles, boundary contracts, thread/session alignment rules, recovery protocol, error mapping, and test vectors |
| 2026-02-07 | 0.2 | Nowa | Added optional independent thread profile with negotiated `thread_mode`, kept coupled mode as MTI baseline, and aligned CDDL/errors/tests for mode-specific behavior |
| 2026-02-07 | 0.3 | Nowa | Aligned idempotency key with RFC 001, made session-control dispatch deterministic via `sess_v`, and added deterministic error-code precedence and dispatch test vector |
| 2026-02-07 | 0.4 | Nowa | Added normative session context carriage for independent mode, clarified session-scoped non-control binding rules, and expanded error/test coverage to prevent implementation divergence |
| 2026-02-07 | 0.5 | Nowa | Made session-scoped non-control dispatch deterministic, aligned provisional CDDL with session context carriage, and added malformed-session-context negative vector |
| 2026-02-07 | 0.6 | Nowa | Aligned request-handling state machine error ranges with Section 9 deterministic mappings (including 1xxx parse/version failures) |
| 2026-02-07 | 0.7 | Nowa | Added explicit `session_scope=true` marker for session-scoped non-control dispatch, removed implicit thread-only session entry, and aligned error/vector coverage for ambiguity prevention |
| 2026-02-07 | 0.8 | Nowa | Added minimal byte-level error-code checks for session interop vectors (`1001`/`4001`) |
