# RFC 014: Handoff Protocol

**Status**: Draft
**Authors**: Nowa
**Created**: 2026-02-22
**Updated**: 2026-02-22
**Version**: 0.2

---

## Dependencies

**Depends On:**
- RFC 001: Agent Messaging Protocol (Core)

**Related:**
- RFC 002: Transport Bindings (transport binding mandatory)
- RFC 003: Relay and Store-and-Forward (at-least-once delivery safety)
- RFC 004: Capability Schema Registry and Compatibility (capability interop)
- RFC 005: Delegation Credentials and Authorization (delegated handoff execution)
- RFC 006: Session Protocol (optional session migration on handoff)
- RFC 007: Agent Payment Protocol (handoff terms may reference payment)
- RFC 008: Agent Discovery and Directory (target discovery by capability)
- RFC 009: Reputation and Trust Signals (post-handoff reputation updates)
- RFC 012: Task Protocol (optional task snapshot in context package)
- RFC 013: Negotiation Protocol (handoff terms may reference negotiation)

---

## Abstract

This RFC defines agent-to-agent responsibility transfer (handoff) semantics for AMP. It standardizes initiation, acceptance, rejection, context packaging, client notification, completion, cancellation, and status query flows for transferring ongoing work from one agent (source) to another agent (target). Context packages carry optional task snapshots (RFC 012) and conversation history for continuity. The protocol supports three transfer modes: full (complete handoff), assist (temporary delegation), and escalate (capability escalation). An optional client notification and veto mechanism provides end-user visibility and control over transfers.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
   2.1 Terminology
   2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Handoff Data Model
   4.1 Lifecycle States
   4.2 Identifiers
   4.3 Transfer Modes
   4.4 Context Package Model
   4.5 Handoff Core CDDL
5. Handoff Protocol Semantics
   5.1 Message Type Usage and Dispatch
   5.2 Initiate Flow
   5.3 Accept Flow
   5.4 Reject Flow
   5.5 Complete Flow
   5.5a Fail Flow
   5.6 Notify Flow
   5.7 Client Veto Flow
   5.8 Cancel Flow
   5.9 Status Query and Status Response
   5.10 Handoff Action Direction and Correlation Matrix
   5.11 Idempotency and Replay Rules
   5.12 CAP_INVOKE Interop Profile
   5.13 Handoff Body CDDL
6. State Machines
7. Error Handling and Retry
8. Versioning and Compatibility
9. Security Considerations
10. Privacy Considerations
11. Implementation Checklist
12. References
Appendix A. Minimal Test Vectors
Appendix B. Open Questions
Changelog

---

## 1. Problem Statement and Scope

Agents operating in multi-agent environments frequently need to transfer responsibility for ongoing work. A customer service agent may need to escalate to a specialist; a general-purpose agent may need temporary help from a domain expert; a coordinating agent may need to hand off an entire task to a more capable peer. Without standardized handoff semantics, these transfers are ad-hoc and fragile: context is lost during transfer, clients are confused about who to contact, state transitions are ambiguous, and there is no structured way to package and transfer working state between agents.

This RFC defines:
- Signed handoff lifecycle bodies for initiate, accept, reject, complete, notify, client_veto, cancel, status_query, and status actions.
- A structured context packaging model with optional task snapshot (RFC 012) and conversation history.
- Three transfer modes (full, assist, escalate) with distinct lifecycle semantics.
- Client notification and optional veto mechanism for end-user visibility and control.
- Deterministic handoff state transitions and idempotency expectations.
- Deterministic handoff-specific error mapping in the `44xx` range.

This RFC does not define:
- Task execution semantics (RFC 012).
- Payment or compensation for handoff work (RFC 007).
- Session lifecycle management (RFC 006).
- Negotiation of handoff terms (RFC 013).
- Discovery of target agents for handoff (RFC 008).
- Multi-agent coordination topology (RFC 011).

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope, signature, and idempotency semantics.
- Applies handoff dispatch rules in Section 5.1.
- Implements required handoff action schemas in Section 5.13.
- If CAP path is supported, implements Section 5.12 and CAP mapping schemas in Section 5.13.
- Enforces action direction/correlation and idempotency requirements in Sections 5.10 and 5.11.
- Enforces state transitions in Section 6.
- Uses deterministic error mapping in Section 7.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Handoff ID | Stable handoff identifier (`bstr .size 16`) for one handoff lifecycle. Created by the source agent. |
| Source | Agent initiating the handoff (transferring responsibility). |
| Target | Agent receiving the handoff (accepting responsibility). |
| Client | End-user agent or human delegate that originally commissioned the work. Optional participant. |
| Context Package | Structured data bundle transferred from source to target containing work state, conversation history, and metadata. |
| Transfer Mode | Nature of the transfer: `full` (complete transfer), `assist` (temporary help), `escalate` (capability escalation). |
| Handoff Action | Profile-defined action verb carried in `body.action` (e.g., `initiate`, `accept`, `reject`). |

### 2.2 Role Profiles and MTI Requirements

`Source Agent Profile`:
- MUST support `initiate`, `complete`, `fail`, `cancel`, and `status_query` actions.
- MUST generate a unique `handoff_id` (`bstr .size 16`) per handoff lifecycle.
- MUST package context with appropriate fidelity for the target agent.
- MUST notify the client of the handoff when a client DID is known.
- MUST enforce one handoff lifecycle state machine per `handoff_id`.
- MUST use stable `handoff_id` per intent and preserve idempotency on retries.
- MUST send `fail` when `assist` mode handoff cannot be completed.

`Target Agent Profile`:
- MUST support `accept`, `reject`, `fail`, and `status_query` actions.
- MUST validate the context package before accepting a handoff.
- MUST send `complete` when handoff work is finished (for `full` and `escalate` modes).
- MUST send `fail` when handoff work cannot be completed (for `full` and `escalate` modes).
- MUST provide status query responses for active and terminal handoffs.

`Client Profile` (optional participation):
- MAY receive `notify` messages about handoffs affecting work the client commissioned.
- MAY send `client_veto` to block a pending handoff (only valid while handoff is in INITIATED state).
- MAY send `cancel` to terminate an active handoff.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

With RFC 001:
- This RFC uses `typ = 0x82` as its registered Standard Profile type code.
- Handoff semantics are encoded in signed body fields using the `profile-body` structure from RFC 001 Section 4.3.
- `body.profile` MUST equal `"xyz.agentries.handoff"` for all handoff messages.
- `body.action` carries the handoff action verb (e.g., `"initiate"`, `"accept"`).
- `body.profile_v` carries the profile version (semver).
- Receivers supporting this profile MUST accept messages via either `typ = 0x82` OR `typ = 0xF0` with matching `body.profile = "xyz.agentries.handoff"` (dual-accept per RFC 001 Section 13.3).

With RFC 002:
- Transport principal binding remains mandatory.
- Unauthorized principal/from combinations in handoff actions MUST map to `3001`.

With RFC 003:
- Relay redelivery may produce duplicates; endpoints MUST rely on RFC 001 idempotency.
- Handoff actions MUST be safe under at-least-once delivery.

With RFC 004:
- Handoff services MAY be exposed as capabilities.
- If handoff actions are carried via `CAP_INVOKE`, capability/version negotiation follows RFC 004.
- CAP interop baseline for this RFC is `xyz.agentries.handoff.workflow:1.0.0` (Section 5.12).

With RFC 005:
- This revision does not define delegation carriage inside handoff action bodies.
- If delegated handoff execution is required, implementations MUST use delegated `CAP_INVOKE` path and follow RFC 005.

With RFC 006:
- Handoff flows MAY trigger session migration between source and target agents.
- If session-scoped, `body.session` requirements and thread rules follow RFC 006.
- Session lifecycle is independent of handoff lifecycle; a session MAY outlive or predate a handoff.

With RFC 007:
- Handoff initiation MAY reference payment terms via `terms_id` (linking to a prior RFC 013 negotiation that includes payment).
- Payment lifecycle is independent of handoff lifecycle.

With RFC 008/009:
- Discovery MAY be used to find suitable handoff targets based on capabilities.
- Reputation updates derived from handoff outcomes are out of scope and defined by RFC 009.

With RFC 012:
- Context packages MAY include `task_id` and `task_snapshot` from RFC 012.
- Task lifecycle is independent of handoff lifecycle; a task MAY continue after handoff completes.
- Handoff does not create, modify, or terminate tasks; it transfers responsibility for work that may or may not be task-tracked.

With RFC 013:
- Handoff terms (e.g., scope, SLA, compensation) MAY be pre-negotiated via RFC 013.
- If pre-negotiated, `initiate` body MAY include `terms_id` referencing the negotiation outcome.
- Negotiation lifecycle is independent of handoff lifecycle.

---

## 4. Handoff Data Model

### 4.1 Lifecycle States

Canonical states:

```
INITIATED  -> ACCEPTED / REJECTED / CANCELED
ACCEPTED   -> COMPLETED / FAILED / CANCELED
```

State constraints:
- `accept` is valid only from `INITIATED`.
- `reject` is valid only from `INITIATED`.
- `complete` is valid only from `ACCEPTED`.
- `fail` is valid only from `ACCEPTED`.
- `cancel` is valid from `INITIATED` or `ACCEPTED`.
- `client_veto` is valid only from `INITIATED`.
- Terminal states (`REJECTED`, `COMPLETED`, `FAILED`, `CANCELED`) are irreversible.

### 4.2 Identifiers

- `handoff_id`: `bstr .size 16`, created by source. MUST be unique per handoff lifecycle. Follows RFC 001 Section 4.2 message ID design (timestamp prefix + random suffix).
- `task_id` (optional): `bstr .size 16` from RFC 012, linking the handoff to a tracked task.
- `session_id` (optional): `bstr .size 16` from RFC 006, linking the handoff to an active session.
- `terms_id` (optional): `bstr .size 16` from RFC 013, linking the handoff to pre-negotiated terms.

### 4.3 Transfer Modes

| Mode | Semantics | Completion Sender | Source Role After Accept |
|------|-----------|-------------------|--------------------------|
| `full` | Complete responsibility transfer. Source disengages after target accepts. Target owns the work until completion. | Target sends `complete` | Disengaged. Source SHOULD NOT send further instructions. |
| `assist` | Temporary delegation. Source remains primary owner. Target provides help and returns control. | Source sends `complete` (acknowledging assist is done) or target sends `complete` (returning control) | Active. Source monitors progress and may cancel. |
| `escalate` | Capability escalation. Source cannot handle the work; target has required capabilities. Similar to `full` but implies capability gap as the reason. | Target sends `complete` | Disengaged. Source receives completion notification. |

Mode constraints:
- Transfer mode MUST remain immutable after `initiate`. Changing mode requires cancel + re-initiate.
- Transfer mode MUST be included in `initiate` and `status` bodies.

### 4.4 Context Package Model

The context package is a structured data bundle that carries working state from source to target. It is designed for loose coupling: all fields except `entries` are optional.

Context package contents:
- `entries`: Array of key-value context entries, each with a content type. REQUIRED (may be empty array).
- `task_id`: Optional reference to an RFC 012 task. Allows target to query task state independently.
- `task_snapshot`: Optional point-in-time snapshot of task state from RFC 012. Avoids requiring target to have RFC 012 access.
- `conversation_history`: Optional array of conversation turns for continuity. Each turn includes role, content, and timestamp.
- `session_id`: Optional reference to an RFC 006 session for session migration.
- `metadata`: Optional extensible metadata map for implementation-specific context.

Design rationale:
- The context package is intentionally loosely coupled to RFC 012 and RFC 006. A handoff can occur without any task or session context, with only a task reference, with a full task snapshot, or with conversation history alone.
- `task_snapshot` is an opaque `any` type because task state schemas are defined by RFC 012, not this RFC. Implementations that understand RFC 012 task state MAY parse and validate the snapshot; others MUST preserve it as-is.

### 4.5 Handoff Core CDDL

```cddl
handoff-id = bstr .size 16
task-id = bstr .size 16
unix-ms = uint
did = tstr
semver = tstr

handoff-status =
  "initiated" / "accepted" / "rejected" /
  "completed" / "failed" / "canceled"

transfer-mode = "full" / "assist" / "escalate"

context-entry = {
  "key": tstr,                    ; entry identifier (e.g., "project_brief", "user_preferences")
  "content_type": tstr,           ; MIME type or structural hint (e.g., "text/plain", "application/json")
  "data": any                     ; entry payload (type depends on content_type)
}

conversation-turn = {
  "role": tstr,                   ; e.g., "user", "assistant", "system"
  "content": any,                 ; turn content (text, structured data, or mixed)
  "ts": unix-ms                   ; when this turn occurred
}

context-package = {
  "entries": [* context-entry],               ; structured context entries (REQUIRED, may be empty)
  ? "task_id": task-id,                       ; RFC 012 task reference
  ? "task_snapshot": any,                     ; RFC 012 task state snapshot (opaque)
  ? "conversation_history": [* conversation-turn],  ; conversation turns for continuity
  ? "session_id": bstr .size 16,              ; RFC 006 session reference
  ? "metadata": { * tstr => any }             ; extensible implementation-specific metadata
}

handoff-session-context = {
  "session_id": bstr .size 16,
  "session_scope": true
}

handoff-base = {
  "profile": "xyz.agentries.handoff",
  "action": tstr,
  "profile_v": semver,
  "handoff_id": handoff-id,
  ? "session": handoff-session-context
}
```

---

## 5. Handoff Protocol Semantics

### 5.1 Message Type Usage and Dispatch

- Handoff messages MUST use `typ = 0x82` (registered Standard Profile type code) when the peer has declared the profile with `typ` in HELLO negotiation.
- If the peer has not declared `typ`, or no HELLO was exchanged, the sender MUST use `typ = 0xF0` (Private Profile fallback) with `body.profile = "xyz.agentries.handoff"`.
- Receivers supporting this profile MUST accept messages via either `typ = 0x82` OR `typ = 0xF0` with `body.profile = "xyz.agentries.handoff"` (dual-accept per RFC 001 Section 13.3).
- A message is treated as a handoff message when `typ` is `0x82`, or when `typ` is `0xF0` and `body.profile` equals `"xyz.agentries.handoff"`.
- For handoff messages, `body.action` and `body.handoff_id` are REQUIRED; missing or invalid fields MUST be rejected as `1001 INVALID_MESSAGE`.
- `body.profile` MUST equal `"xyz.agentries.handoff"`; receivers MUST validate this and SHOULD reject mismatches with `4001 BAD_REQUEST` (per RFC 001 Section 4.3).
- `body.profile_v` MUST be a supported profile version. For this revision, `profile_v` MUST be `"1.0.0"`. Unsupported values MUST be rejected with `4006 PROFILE_VERSION_UNSUPPORTED`.
- Unknown `action` values in handoff messages MUST be rejected with `4405 UNKNOWN_HANDOFF_ACTION`.

### 5.2 Initiate Flow

The source agent sends `action: "initiate"` to the target agent to propose a handoff.

Initiate body MUST include:
- `handoff_id`: Unique identifier for this handoff lifecycle.
- `target`: DID of the target agent.
- `mode`: Transfer mode (`full`, `assist`, or `escalate`).
- `context`: Context package containing work state for the target.

Initiate body MAY include:
- `client`: DID of the end-user client (enables client notification and veto).
- `reason`: Human-readable reason for the handoff.
- `terms_id`: Reference to pre-negotiated terms from RFC 013.

Processing rules:
- Source MUST generate a fresh `handoff_id` for each new handoff intent.
- Source MUST set handoff state to `INITIATED` upon sending.
- Source SHOULD notify the client (Section 5.6) concurrently with or shortly after sending `initiate`.
- Target MUST validate the context package before accepting (Section 5.3).
- Target MUST respond with `accept` or `reject`.

### 5.3 Accept Flow

The target agent sends `action: "accept"` to the source agent to confirm acceptance of the handoff.

Accept body MAY include:
- `estimated_completion`: Unix timestamp (milliseconds) indicating when the target expects to complete the handoff work.

Processing rules:
- Target MUST validate that the handoff is in `INITIATED` state before accepting.
- Upon acceptance, handoff state transitions to `ACCEPTED`.
- For `full` and `escalate` modes, the target assumes primary responsibility for the work.
- For `assist` mode, the target begins the assist work while the source remains primary.

### 5.4 Reject Flow

The target agent sends `action: "reject"` to the source agent to decline the handoff.

Reject body MAY include:
- `reason_code`: Numeric reason code for the rejection.
- `reason`: Human-readable reason for the rejection.

Processing rules:
- Target MUST validate that the handoff is in `INITIATED` state before rejecting.
- Upon rejection, handoff state transitions to `REJECTED` (terminal).
- Source SHOULD notify the client if the handoff was rejected.

### 5.5 Complete Flow

Either the source or the target sends `action: "complete"` to signal that the handoff work is done.

Completion sender depends on transfer mode (normative):
- `full` mode: Target MUST send `complete` to source. Source MUST NOT send `complete`.
- `assist` mode: Either party MAY send `complete`. Target sends `complete` to return control; source sends `complete` to acknowledge that the assist is finished.
- `escalate` mode: Target MUST send `complete` to source. Source MUST NOT send `complete`.
- A `complete` from a disallowed sender for the given mode MUST be rejected with `4001 BAD_REQUEST`.

Complete body MAY include:
- `result`: Outcome data from the handoff work (opaque to this protocol).
- `return_context`: Context package returned to source (primarily for `assist` mode, carrying updated state back).

Processing rules:
- Sender MUST validate that the handoff is in `ACCEPTED` state before completing.
- Upon completion, handoff state transitions to `COMPLETED` (terminal).
- Source SHOULD notify the client of completion.

### 5.5a Fail Flow

The source or target sends `action: "fail"` to signal that the handoff cannot be completed due to an error during execution.

Fail body MUST include:
- `error_code`: Numeric error code indicating the failure reason.

Fail body MAY include:
- `message`: Human-readable failure description.
- `details`: Structured error details (opaque to this protocol).

Processing rules:
- `fail` is valid only from `ACCEPTED` state. Fail from other states MUST be rejected with `4402 INVALID_STATE_TRANSITION`.
- Upon failure, handoff state transitions to `FAILED` (terminal).
- Fail sender depends on transfer mode (normative):
  - `full` mode: Target MUST send `fail` to source. Source MUST NOT send `fail`.
  - `assist` mode: Source MUST send `fail` to target. Target MUST NOT send `fail`.
  - `escalate` mode: Target MUST send `fail` to source. Source MUST NOT send `fail`.
  - A `fail` from a disallowed sender for the given mode MUST be rejected with `4001 BAD_REQUEST`.
- Source SHOULD notify the client of the failure.

### 5.6 Notify Flow

The source agent sends `action: "notify"` to the client agent to inform them of the handoff.

Notify body MUST include:
- `client`: DID of the client being notified.
- `target`: DID of the target agent.
- `mode`: Transfer mode.

Notify body MAY include:
- `reason`: Human-readable reason for the handoff.

Processing rules:
- Source SHOULD send `notify` to the client when a handoff is initiated.
- Source SHOULD also send `notify` on terminal state transitions (rejection, completion, cancellation).
- Client MAY respond with `client_veto` if the handoff is still in `INITIATED` state.
- If no client DID is known, notification is skipped.

### 5.7 Client Veto Flow

The client agent sends `action: "client_veto"` to the source agent to block a pending handoff.

Client veto body MAY include:
- `reason`: Human-readable reason for the veto.

Processing rules:
- Client veto is valid ONLY when the handoff is in `INITIATED` state.
- If the handoff has already been accepted, the veto MUST be rejected with `4402 INVALID_STATE_TRANSITION`.
- Upon successful veto, handoff state transitions to `CANCELED` (terminal).
- Source MUST forward the cancellation to the target (if `initiate` was already sent).

### 5.8 Cancel Flow

The source or client sends `action: "cancel"` to terminate a handoff.

Cancel body MAY include:
- `reason`: Human-readable reason for the cancellation.

Processing rules:
- Cancel is valid from `INITIATED` or `ACCEPTED` states.
- Cancel from a terminal state MUST be rejected with `4402 INVALID_STATE_TRANSITION`.
- Upon successful cancellation, handoff state transitions to `CANCELED` (terminal).
- When source cancels: source sends `cancel` to target.
- When client cancels: client sends `cancel` to source; source MUST forward to target.
- For `assist` mode, cancel returns control to the source without completion.

### 5.9 Status Query and Status Response

Any party sends `action: "status_query"` to request the current handoff state. The counterparty responds with `action: "status"`.

Status body MUST include:
- `status`: Current canonical handoff state.
- `mode`: Transfer mode.
- `source`: DID of the source agent.
- `target`: DID of the target agent.
- `updated_at`: Unix timestamp (milliseconds) of last state change.

Status body MAY include:
- `client`: DID of the client.
- `context_summary`: Human-readable summary of the current context (for status visibility without full context transfer).

Processing rules:
- `status_query` is read-only and MAY be retried freely.
- Unknown `handoff_id` in `status_query` MUST be rejected with `4401 HANDOFF_NOT_FOUND`.
- Both source and target MUST maintain handoff state sufficient to answer status queries for active and recently-terminal handoffs.

### 5.10 Handoff Action Direction and Correlation Matrix

The following matrix is normative:

| Action | Sender -> Receiver | Response Action | `reply_to` Requirement |
|--------|-------------------|-----------------|------------------------|
| `initiate` | source -> target | `accept` / `reject` | response `reply_to` MUST reference `initiate` message `id` |
| `accept` | target -> source | (none required) | N/A |
| `reject` | target -> source | (none required) | N/A |
| `complete` (full/escalate) | target -> source | (none required) | N/A |
| `complete` (assist) | either -> counterpart | (none required) | N/A |
| `fail` (full/escalate) | target -> source | (none required) | N/A |
| `fail` (assist) | source -> target | (none required) | N/A |
| `notify` | source -> client | `client_veto` (optional) | veto `reply_to` MUST reference `notify` message `id` |
| `client_veto` | client -> source | (none required) | N/A |
| `cancel` (source-initiated) | source -> target | (none required) | N/A |
| `cancel` (client-initiated) | client -> source | (none required; source MUST forward to target) | N/A |
| `status_query` | any -> any | `status` | `status` `reply_to` MUST reference `status_query` message `id` |
| `status` | queried party -> requester | (none required) | N/A |

Rules:
- `accept`, `reject`, and `status` are response actions and their `reply_to` MUST reference the triggering message's `id`.
- Unsolicited response actions (missing or invalid `reply_to` on `accept`, `reject`, or `status`) MUST be rejected with `4001 BAD_REQUEST`.
- The direction matrix applies to profile-body carriage (Section 5.1). CAP carriage uses `CAP_INVOKE`/`CAP_RESULT` semantics (Section 5.12) and RFC 004.

### 5.11 Idempotency and Replay Rules

Handoff logic MUST remain safe under RFC 003 at-least-once delivery.

Operation idempotency key:
- `op_key = (handoff_id, action, from_did)`.

Rules:
- This section applies to both profile-body carriage (Section 5.1) and CAP carriage (Section 5.12).
- In CAP carriage, `handoff_id` and `action` are read from `CAP_INVOKE.params`.
- Replaying a message with the same semantic body under the same `op_key` MUST return a deterministic equivalent result and MUST NOT create duplicate lifecycle transitions.
- Replaying with same `op_key` but conflicting semantic body (e.g., changed `mode`, `context`, or counterparty fields) MUST be rejected with `4001 BAD_REQUEST`.
- `accept` replay after already `ACCEPTED` MUST return prior equivalent acceptance.
- `reject` replay after already `REJECTED` MUST return prior equivalent rejection.
- `cancel` replay after already `CANCELED` MUST return prior equivalent cancellation.
- `complete` replay after already `COMPLETED` MUST return prior equivalent completion.
- `status_query` is read-only and MAY be retried freely; unknown `handoff_id` MUST map to `4401`.

### 5.12 CAP_INVOKE Interop Profile

This section defines the RFC 014 capability interoperability baseline for RFC 004 invocation path.

Capability identity:
- `id = "xyz.agentries.handoff.workflow:1.0.0"`.

Rules:
- When using CAP path, `CAP_INVOKE` MUST target the capability ID above.
- `CAP_INVOKE.params` MUST contain one request action body from this RFC with `profile = "xyz.agentries.handoff"` and `profile_v = "1.0.0"`.
- Allowed request actions in CAP path: `initiate`, `cancel`, `client_veto`, `status_query`. Terminal-state actions (`complete`, `fail`) are NOT carried via CAP path; they are delivered as profile-body messages (§5.5, §5.5a). CAP callers observe terminal states via `status_query`/`status` round-trips or by receiving direct profile-body `complete`/`fail` messages.
- `CAP_RESULT(status="success").result` MUST contain one corresponding response action body from this RFC.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005 before handoff execution.
- Invalid/unsupported delegation evidence in CAP handoff path MUST fail with `3004` (RFC 004/005).
- Section 5.10 profile-body direction rules MUST NOT be applied to CAP envelope types.
- Providers supporting this capability MUST publish an RFC 004-compliant descriptor for `xyz.agentries.handoff.workflow:1.0.0`.
- Descriptor/schema integrity verification (hash/signature/trust profile behavior) MUST follow RFC 004 Sections 4.2 and 5.2 before schema validation/execution.
- Descriptor input schema MUST correspond to `app-cap-invoke-params`; success result schema MUST correspond to `app-cap-result-success`.
- Session context source-of-truth in CAP path is RFC 004 envelope extension (`CAP_INVOKE.body.session`, `CAP_RESULT.body.session`) with semantics governed by RFC 006.
- `CAP_INVOKE.params.session` and `CAP_RESULT.result.session` MAY exist for payload-level compatibility, but if both payload and envelope session context are present, they MUST be semantically equivalent; mismatch MUST fail with `4001`.
- Pre-execution rejection in CAP path (validation/authorization/compatibility/schema) MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

### 5.13 Handoff Body CDDL

```cddl
handoff-action =
  "initiate" / "accept" / "reject" / "complete" / "fail" /
  "notify" / "client_veto" / "cancel" /
  "status_query" / "status"

initiate-body = {
  handoff-base,
  "action": "initiate",
  "target": did,
  "mode": transfer-mode,
  "context": context-package,
  ? "client": did,
  ? "reason": tstr,
  ? "terms_id": bstr .size 16
}

accept-body = {
  handoff-base,
  "action": "accept",
  ? "estimated_completion": unix-ms
}

reject-body = {
  handoff-base,
  "action": "reject",
  ? "reason_code": uint,
  ? "reason": tstr
}

complete-body = {
  handoff-base,
  "action": "complete",
  ? "result": any,
  ? "return_context": context-package
}

fail-body = {
  handoff-base,
  "action": "fail",
  "error_code": uint,
  ? "message": tstr,
  ? "details": any
}

notify-body = {
  handoff-base,
  "action": "notify",
  "client": did,
  "target": did,
  "mode": transfer-mode,
  ? "reason": tstr
}

client-veto-body = {
  handoff-base,
  "action": "client_veto",
  ? "reason": tstr
}

cancel-body = {
  handoff-base,
  "action": "cancel",
  ? "reason": tstr
}

status-query-body = {
  handoff-base,
  "action": "status_query"
}

status-body = {
  handoff-base,
  "action": "status",
  "status": handoff-status,
  "mode": transfer-mode,
  "source": did,
  "target": did,
  "updated_at": unix-ms,
  ? "client": did,
  ? "context_summary": tstr
}

app-capability-id = "xyz.agentries.handoff.workflow:1.0.0"

app-cap-invoke-params =
  initiate-body /
  cancel-body /
  client-veto-body /
  status-query-body

app-cap-result-success =
  accept-body /
  reject-body /
  status-body
```

---

## 6. State Machines

### 6.1 Handoff Lifecycle State Machine

```
NONE
  -> INITIATED             (initiate sent by source)
INITIATED
  -> ACCEPTED              (accept received from target)
  -> REJECTED              (reject received from target)     [terminal]
  -> CANCELED              (cancel from source/client, or client_veto)  [terminal]
ACCEPTED
  -> COMPLETED             (complete per mode: target for full/escalate, either for assist)  [terminal]
  -> FAILED                (fail per mode: target for full/escalate, source for assist)  [terminal]
  -> CANCELED              (cancel from source/client)       [terminal]
REJECTED / COMPLETED / FAILED / CANCELED
  -> TERMINAL              (no further transitions allowed)
```

### 6.2 State Machine Invariants

- Each `handoff_id` MUST have exactly one state machine instance.
- State transitions MUST be monotonic (no backward transitions).
- Terminal states are irreversible.
- All state transitions MUST be serialized per `handoff_id`; concurrent transitions MUST be resolved deterministically (first valid transition wins).
- Invalid state transitions MUST be rejected with `4402 INVALID_STATE_TRANSITION`.

### 6.3 ASCII State Diagram

```
                    ┌─────────────┐
                    │    NONE     │
                    └──────┬──────┘
                           │ initiate
                           v
                    ┌─────────────┐
              ┌─────│  INITIATED  │─────┐
              │     └──────┬──────┘     │
              │            │            │
         reject/      accept       cancel/
         client_veto       │       client_veto
              │            │            │
              v            v            v
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ REJECTED │ │ ACCEPTED │ │ CANCELED │
        │(terminal)│ └─────┬────┘ │(terminal)│
        └──────────┘       │      └──────────┘
                     ┌─────┼─────┐
                     │     │     │
                complete  fail  cancel
                     │     │     │
                     v     v     v
              ┌─────────┐ ┌────────┐ ┌──────────┐
              │COMPLETED│ │ FAILED │ │ CANCELED │
              │(terminal)│ │(terminal)│(terminal)│
              └─────────┘ └────────┘ └──────────┘
```

---

## 7. Error Handling and Retry

This RFC reuses the RFC 001 error model and introduces handoff-specific business codes in the `44xx` range.

Deterministic precedence (evaluated in order):
1. Parse/shape/type failures (missing required fields, invalid CBOR) -> `1001`.
2. Profile/typ mismatch (`typ = 0x82` but `body.profile` not `"xyz.agentries.handoff"`) -> `4001`.
3. Unknown profile (message via `typ = 0xF0` with unrecognized `body.profile`) -> `4005`.
4. Unsupported `profile_v` -> `4006`.
5. Authorization identity/policy failure -> `3001`.
6. CAP pre-resolution/coarse policy denial -> `3001` (RFC 004 validation order).
7. CAP delegation evidence failure (after coarse auth checks) -> `3004`.
8. CAP descriptor signature/trust-profile verification failure -> `3001`.
9. Handoff semantic/request-shape conflicts -> `4001`.
10. Handoff state/business failures -> `44xx`.
11. CAP descriptor/schema artifact unavailable or integrity source unavailable -> `5002`.
12. Transient backend failures -> `500x`.

| Condition | Code | Name | Retry |
|-----------|------|------|-------|
| Malformed handoff body | `1001` | INVALID_MESSAGE | No |
| Unsupported profile version (`body.profile_v`) | `4006` | PROFILE_VERSION_UNSUPPORTED | No |
| Unknown profile (`body.profile`) | `4005` | UNKNOWN_PROFILE | No |
| Unauthorized handoff actor | `3001` | UNAUTHORIZED | No |
| CAP pre-resolution/coarse policy denial | `3001` | UNAUTHORIZED | No |
| Invalid/unsupported CAP delegation evidence | `3004` | DELEGATION_INVALID | No |
| Handoff direction/correlation mismatch | `4001` | BAD_REQUEST | No |
| CAP session context mismatch (envelope vs payload) | `4001` | BAD_REQUEST | No |
| Conflicting retry payload for same (`handoff_id`, `action`, `from_did`) | `4001` | BAD_REQUEST | No |
| Handoff not found (`handoff_id` unknown) | `4401` | HANDOFF_NOT_FOUND | No |
| Invalid state transition | `4402` | INVALID_STATE_TRANSITION | No |
| Invalid transfer mode | `4403` | INVALID_TRANSFER_MODE | No |
| Context package too large | `4404` | CONTEXT_TOO_LARGE | No |
| Unknown handoff action | `4405` | UNKNOWN_HANDOFF_ACTION | No |
| CAP descriptor signature required but missing/invalid under trust profile | `3001` | UNAUTHORIZED | No |
| CAP descriptor/schema artifact missing, unreadable, or integrity source unavailable | `5002` | UNAVAILABLE | Yes |
| Target agent unavailable | `5002` | UNAVAILABLE | Yes |
| Internal handoff engine failure | `5001` | INTERNAL_ERROR | Yes |

Registry note:
- `44xx` handoff codes MUST be registered via RFC 001 Section 17 process before status advances beyond Draft.

---

## 8. Versioning and Compatibility

Version dimensions:
- AMP envelope version `v` remains governed by RFC 001.
- Handoff profile version is `profile_v`, currently fixed at `"1.0.0"`.

Compatibility rules:
- Unknown required fields MUST fail with `1001`.
- Unknown optional fields MAY be ignored unless security-sensitive.
- Backward-compatible extensions MUST use optional fields only.
- `profile_v` follows semantic versioning (semver). Major version changes indicate breaking changes; minor version changes indicate backward-compatible additions.

HELLO negotiation:
- Implementations supporting this profile SHOULD declare it in HELLO `profiles`:
  ```
  {"name": "xyz.agentries.handoff", "version": "1.0.0", "typ": 130}
  ```
  (where `130` is `0x82` in decimal).
- If the peer declared this profile in HELLO but without a `typ` field, or no HELLO was exchanged, the sender MUST fall back to `typ = 0xF0`. If the peer's HELLO or `profile_status` indicates no support for this profile, sending handoff messages to that peer will fail regardless of `typ` used.

---

## 9. Security Considerations

- **Replay safety**: Implementations MUST enforce RFC 001 idempotency with stable `handoff_id` semantics. Replayed handoff actions MUST NOT create duplicate state transitions.
- **Context integrity**: Context packages are carried inside the signed `body` and are therefore integrity-protected. Intermediaries MUST NOT tamper with context package contents.
- **Authorization model**: Implementations MUST validate that the sender is authorized to perform the given action. For `initiate`, only the source MAY send. For `accept`/`reject`, only the target MAY send. For `client_veto`, only the declared client MAY send. Unauthorized actions MUST be rejected with `3001`.
- **Principal/from binding**: RFC 002 principal/from binding applies to all handoff actions. The `from` field in the AMP envelope MUST match the expected role for the action.
- **Context package sensitivity**: Context packages may contain sensitive work state, conversation history, and task data. Implementations SHOULD use `authcrypt` encryption (RFC 001 Section 8.5) for handoff messages carrying sensitive context.
- **Client veto authenticity**: Client veto messages MUST be signed by the declared client DID. Implementations MUST verify that the veto sender matches the `client` DID declared in the original `initiate`.
- **Denial of service**: Implementations SHOULD rate-limit handoff initiation to prevent adversarial agents from flooding targets with handoff requests. RFC 001 rate limiting (`3005`) applies.
- **Transfer mode immutability**: Transfer mode is fixed at initiation and cannot be changed. This prevents an attacker from escalating a temporary `assist` to a permanent `full` transfer after acceptance.

---

## 10. Privacy Considerations

- Handoff messages reveal relationships between source, target, and client agents. Implementations SHOULD minimize metadata exposure in logs and telemetry.
- Context packages may contain personal data, conversation history, and task details. Implementations SHOULD minimize retention of context package contents after handoff completion.
- Status query responses SHOULD prefer IDs, status codes, and summaries over full context data where possible.
- Conversation history in context packages may contain sensitive user interactions. Implementations SHOULD allow source agents to redact or summarize conversation history before inclusion in the context package.
- Logs SHOULD prefer handoff IDs and status codes over full context package contents.

---

## 11. Implementation Checklist

- [ ] Implement handoff dispatch guard in Section 5.1 (dual-accept for `typ = 0x82` and `typ = 0xF0`).
- [ ] Implement action direction/correlation matrix in Section 5.10.
- [ ] Implement idempotency/replay rules in Section 5.11.
- [ ] Implement CAP interop profile in Section 5.12 if CAP path is supported.
- [ ] Implement all Section 5.13 action body schemas.
- [ ] Enforce lifecycle state transitions in Section 6.
- [ ] Enforce deterministic error mapping in Section 7.
- [ ] Ensure idempotent behavior on repeated handoff action retries.
- [ ] Implement context package validation (structure, size limits).
- [ ] Implement client notification flow (Section 5.6).
- [ ] Implement client veto flow (Section 5.7) if client participation is supported.
- [ ] Declare `xyz.agentries.handoff` profile in HELLO negotiation.
- [ ] Add conformance tests from Appendix A.

---

## 12. References

### 12.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs
- RFC 8174: Ambiguity of uppercase/lowercase in RFC 2119 keywords
- RFC 8949: CBOR
- RFC 8610: CDDL

### 12.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 004: Capability Schema Registry and Compatibility
- RFC 005: Delegation Credentials and Authorization
- RFC 006: Session Protocol
- RFC 007: Agent Payment Protocol
- RFC 008: Agent Discovery and Directory
- RFC 009: Reputation and Trust Signals
- RFC 012: Task Protocol
- RFC 013: Negotiation Protocol

---

## Appendix A. Minimal Test Vectors

### A.1 Full Handoff to Complete Positive

Input:
- Source sends `initiate` with `mode = "full"`, valid context package, target DID.
- Target sends `accept`.
- Target completes work, sends `complete` with result.

Expected:
- State transitions: `INITIATED -> ACCEPTED -> COMPLETED`.
- All `reply_to` correlation rules in Section 5.10 satisfied.
- Source receives `complete` with optional result data.

### A.2 Handoff Rejection Positive

Input:
- Source sends `initiate` with `mode = "full"`.
- Target sends `reject` with `reason = "Insufficient capabilities"`.

Expected:
- State transitions: `INITIATED -> REJECTED`.
- Handoff lifecycle terminates.
- Source MAY notify client of rejection.

### A.3 Assist Mode Complete Positive

Input:
- Source sends `initiate` with `mode = "assist"`, context package including conversation history.
- Target sends `accept`.
- Target completes assist work, sends `complete` with `return_context` containing updated state.

Expected:
- State transitions: `INITIATED -> ACCEPTED -> COMPLETED`.
- `return_context` carries updated work state back to source.
- Source resumes primary responsibility with enriched context.

### A.4 Escalate Mode Positive

Input:
- Source sends `initiate` with `mode = "escalate"`, `reason = "Requires legal expertise"`.
- Target sends `accept` with `estimated_completion`.
- Target completes escalated work, sends `complete`.

Expected:
- State transitions: `INITIATED -> ACCEPTED -> COMPLETED`.
- Source disengages after acceptance (similar to `full` mode).

### A.5 Client Veto Positive

Input:
- Source sends `initiate` to target.
- Source sends `notify` to client.
- Client sends `client_veto` with `reason = "I prefer the current agent"`.

Expected:
- State transitions: `INITIATED -> CANCELED`.
- `client_veto.reply_to` MUST reference `notify.id`.
- Source forwards cancellation to target.
- Handoff lifecycle terminates.

### A.6 Cancel Active Handoff Positive

Input:
- Source sends `initiate`, target sends `accept`.
- Source sends `cancel` with `reason = "Requirements changed"`.

Expected:
- State transitions: `INITIATED -> ACCEPTED -> CANCELED`.
- Target receives `cancel` and stops work.
- Handoff lifecycle terminates.

### A.7 Invalid State Transition Negative (4402)

Input:
- Handoff is in `ACCEPTED` state.
- Target sends `accept` again (duplicate acceptance attempt with different semantic body).

Expected:
- Reject with `4402 INVALID_STATE_TRANSITION`.
- Byte-level check: error code `4402` is CBOR uint bytes `19 11 32`.

### A.8 Handoff Not Found Negative (4401)

Input:
- `status_query` for `handoff_id` with no known lifecycle record.

Expected:
- Reject with `4401 HANDOFF_NOT_FOUND`.
- Byte-level check: error code `4401` is CBOR uint bytes `19 11 31`.

### A.9 Context Too Large Negative (4404)

Input:
- Source sends `initiate` with `context` package exceeding implementation-defined size limit.

Expected:
- Reject with `4404 CONTEXT_TOO_LARGE`.
- Byte-level check: error code `4404` is CBOR uint bytes `19 11 34`.

### A.10 Unauthorized Handoff Negative (3001)

Input:
- Agent that is not the declared target sends `accept` for a handoff.

Expected:
- Reject with `3001 UNAUTHORIZED`.
- Byte-level check: error code `3001` is CBOR uint bytes `19 0b b9`.

### A.11 Fail from Accepted Positive

Input:
- Source sends `initiate` with `mode = "full"`.
- Target sends `accept`.
- Target sends `fail` with `error_code = 5001` and `message = "Internal processing error"`.

Expected:
- State transitions: `INITIATED -> ACCEPTED -> FAILED`.
- Source receives `fail` with `error_code` and `message`.
- Handoff lifecycle terminates.

### A.12 Fail from Invalid State Negative (4402)

Input:
- Source sends `initiate` with `mode = "full"`.
- Target sends `fail` (without first accepting).

Expected:
- Reject with `4402 INVALID_STATE_TRANSITION`.
- Byte-level check: error code `4402` is CBOR uint bytes `19 11 32`.
- Handoff remains in `INITIATED` state.

---

## Appendix B. Open Questions

1. **Context package size limits**: Should context packages have a standard maximum size, or should this be left to implementation policy? Current design leaves size limits to implementations and uses `4404` for enforcement.

2. **Multi-hop handoffs**: Should multi-hop handoffs (A -> B -> C) be explicitly modeled with chained `handoff_id` references, or should each hop be an independent handoff lifecycle? Current design treats each hop independently.

3. **Handoff audit history**: Should handoff history be queryable for audit purposes (e.g., "show all handoffs for this task")? This may be better addressed by RFC 010 (Observability) or a dedicated audit profile.

4. **Context package streaming**: For very large context packages (e.g., extensive conversation history), should streaming delivery be supported via RFC 001 STREAM_START/DATA/END? Current design assumes context fits in a single message.

5. **Handoff timeout**: Should there be a standard timeout for the `INITIATED` state (auto-cancel if target does not respond)? Current design relies on AMP TTL for message expiry but does not define handoff-level timeouts.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-22 | 0.1 | Nowa | Initial draft: handoff lifecycle (initiate/accept/reject/complete/cancel/notify/veto/status), three transfer modes, context packaging model, client notification and veto, state machine, error codes (44xx range), CDDL schemas, CAP interop profile, and 10 minimal test vectors |
| 2026-02-22 | 0.2 | Nowa | Added `fail` action with mode-specific sender rules (§5.5a), CDDL body, direction matrix entries, and role profile MTI. Strengthened complete/fail sender constraints to normative MUST per mode. Clarified CAP interop terminal-state observability. Added test vectors A.11 (fail positive) and A.12 (fail invalid state negative). Added RFC 008/009 to Related dependencies. |
