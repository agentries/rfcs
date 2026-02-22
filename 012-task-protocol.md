# RFC 012: Task Protocol

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
- RFC 004: Capability Schema Registry and Compatibility (optional task capability negotiation)
- RFC 005: Delegation Credentials and Authorization (delegated task execution boundary)
- RFC 006: Session Protocol (transient provisional responses vs persistent task state)
- RFC 007: Agent Payment Protocol (optional payment linkage)
- RFC 013: Negotiation Protocol (optional terms linkage)
- RFC 014: Handoff Protocol (optional task snapshot handoff)

---

## Abstract

This RFC defines long-running task lifecycle semantics for AMP agents. It specifies creation, acceptance, progress tracking, input collection, completion, failure, and cancellation flows using signed AMP profile bodies. The protocol enables persistent, queryable task state independent of session or connection lifetime. Sub-task delegation allows workers to decompose work across multiple agents while maintaining lifecycle coherence.

This is the first AMP Standard Profile RFC. Task messages use `typ = 0x80` with profile-body format (`profile`, `action`, `profile_v`) as defined in RFC 001 Section 4.3, not the `REQUEST`/`RESPONSE` + version-marker pattern used by earlier application RFCs.

---

## Table of Contents

1. Problem Statement and Scope
2. Conformance and Profiles
   2.1 Terminology
   2.2 Role Profiles and MTI Requirements
3. Boundary Contracts with Other RFCs
4. Task Data Model
   4.1 Lifecycle States
   4.2 Task Identifiers
   4.3 Task Core CDDL
5. Task Protocol Semantics
   5.1 Message Type Usage and Dispatch
   5.2 Create Flow
   5.3 Accept / Reject Flow
   5.4 Progress Flow
   5.5 Input Request / Input Response Flow
   5.6 Complete Flow
   5.7 Fail Flow
   5.8 Cancel / Cancel Result Flow
   5.8a Action Direction Matrix
   5.9 Status Query / Status Flow
   5.10 Subtask Create / Subtask Result Flow
   5.11 CAP_INVOKE Interop Profile
   5.12 Task Body CDDL
   5.13 Idempotency and Replay Rules
6. Relationship to RFC 006 Provisional Responses
7. State Machines
8. Error Handling and Retry
9. Versioning and Compatibility
10. Security Considerations
11. Privacy Considerations
12. Implementation Checklist
13. References
Appendix A. Test Vectors
Appendix B. Open Questions
Changelog

---

## 1. Problem Statement and Scope

AMP provides messaging, sessions, and provisional responses but no persistent task lifecycle abstraction. Without standardized task semantics, implementations diverge on progress reporting, input collection, sub-task delegation, and task querying. Agents that delegate work to other agents lack a common protocol for tracking whether that work has been accepted, is in progress, requires additional input, or has completed.

This RFC defines:
- Signed task lifecycle bodies for create, accept, reject, progress, input request, input response, complete, fail, cancel, and status flows.
- Deterministic task state transitions and idempotency expectations.
- Sub-task coordination semantics for hierarchical delegation.
- Persistent, queryable task state independent of session or connection lifetime.
- Loose coupling to payment (RFC 007) and negotiation (RFC 013) via optional reference identifiers.

This RFC does not define:
- Session management (RFC 006).
- Payment settlement (RFC 007).
- Negotiation of task terms (RFC 013).
- Context handoff between agents (RFC 014).
- A mandatory task scheduling or orchestration engine.
- Jurisdiction-specific compliance or SLA enforcement.

---

## 2. Conformance and Profiles

The key words MUST, MUST NOT, REQUIRED, SHOULD, SHOULD NOT, MAY, and OPTIONAL are interpreted as in RFC 2119 and RFC 8174.

An implementation is conformant only if it:
- Preserves RFC 001 envelope, signature, and idempotency semantics.
- Applies task dispatch rules in Section 5.1.
- Implements required task action schemas in Section 5.12.
- If CAP path is supported, implements Section 5.11 and CAP mapping schemas in Section 5.12.
- Enforces action direction/correlation requirements in Section 5.8a.
- Enforces idempotency requirements in Section 5.13.
- Enforces state transitions in Section 7.
- Uses deterministic error mapping in Section 8.

### 2.1 Terminology

| Term | Definition |
|------|------------|
| Task ID | Stable task identifier (`bstr .size 16`) for one task lifecycle. |
| Task Client | Agent that creates, monitors, and cancels tasks. |
| Task Worker | Agent that accepts and executes tasks. |
| Sub-task | A task created by a worker to delegate part of its work to another agent. |
| Milestone | Named progress checkpoint within a task, carrying a name, timestamp, and optional data. |
| Parent Task ID | Optional reference from a sub-task to the task that spawned it. |

### 2.2 Role Profiles and MTI Requirements

`Task Client Profile`:
- MUST support sending `create`, `cancel`, `input_response`, and `status_query` actions.
- MUST use stable `task_id` per task and preserve idempotency on retries.
- MUST handle incoming `accept`, `reject`, `progress`, `input_request`, `complete`, `fail`, `cancel_result`, and `status` actions.
- MUST NOT send worker-role actions (`accept`, `reject`, `progress`, `complete`, `fail`, `input_request`).

`Task Worker Profile`:
- MUST support sending `accept`, `reject`, `progress`, `complete`, `fail`, `input_request`, `cancel_result`, and `status` actions.
- MUST enforce one task lifecycle state machine per `task_id`.
- MUST provide `status` responses for active and terminal tasks when queried.
- MUST NOT send client-role actions (`create`, `cancel`, `input_response`) except where the worker is also a client of a sub-task.

`Sub-task Coordinator Profile` (optional):
- MAY create sub-tasks via `subtask_create` and receive `subtask_result`.
- MUST propagate parent task cancellation to all active sub-tasks.
- MUST track sub-task lifecycle independently from parent task lifecycle.

---

## 3. Boundary Contracts with Other RFCs

This section is normative.

**With RFC 001:**
- This RFC uses type code `0x80` (Standard Profile) for task messages. This is the first allocation in the `0x80-0xEF` Standard Profile range.
- Task semantics are encoded in signed profile body fields (`profile`, `action`, `profile_v`) as defined in RFC 001 Section 4.3.
- `body.profile` MUST be `"xyz.agentries.task"` for all task protocol messages.
- Receivers MUST validate that `body.profile` matches `"xyz.agentries.task"` when `typ = 0x80`; mismatches MUST be rejected with `4001 BAD_REQUEST`.
- Receivers MUST also accept task messages via `typ = 0xF0` with matching `body.profile = "xyz.agentries.task"` (dual-accept per RFC 001 Section 13.3). This enables migration from Private Profile to Standard Profile without coordinated cutover.

**With RFC 002:**
- Transport principal binding remains mandatory.
- Unauthorized principal/from combinations in task operations MUST map to `3001 UNAUTHORIZED`.

**With RFC 003:**
- Relay redelivery may produce duplicates; endpoints MUST rely on RFC 001 idempotency.
- Task operations MUST be safe under at-least-once delivery.
- Workers and clients MUST tolerate duplicate delivery of any task action without creating duplicate state transitions.

**With RFC 004:**
- Task services MAY be exposed as capabilities.
- If task operations are carried via `CAP_INVOKE`, capability/version negotiation follows RFC 004.
- CAP interop baseline for this RFC is `xyz.agentries.task.workflow:1.0.0` (Section 5.11).

**With RFC 005:**
- If delegated task execution is required, implementations MUST use the delegated `CAP_INVOKE` path and follow RFC 005.
- Delegation evidence in the CAP path MUST be validated before task execution proceeds.

**With RFC 006:**
- Task progress messages are persistent and queryable; RFC 006 provisional responses are transient and request-scoped (see Section 6 for detailed comparison).
- Tasks MAY be session-scoped but task lifecycle is independent of session lifecycle.
- If session-scoped, `body.session` requirements follow RFC 006.

**With RFC 007:**
- Tasks MAY carry an optional `payment_id` field (`bstr .size 16`) linking to a payment intent governed by RFC 007.
- Payment lifecycle is independent of task lifecycle. Task completion does not imply payment capture; task failure does not imply payment cancellation.
- Implementations MUST NOT couple task state transitions to payment state transitions within this protocol.

**With RFC 013:**
- Tasks MAY carry an optional `terms_id` field (`bstr .size 16`) linking to negotiated terms governed by RFC 013.
- Negotiation lifecycle is independent of task lifecycle. Task creation MAY reference previously negotiated terms, but terms expiry and task expiry are orthogonal.

**With RFC 014:**
- Context handoff MAY carry an optional `task_id` and `task_snapshot` from this protocol to transfer task context to another agent.
- Handoff is independent of task lifecycle. The receiving agent MAY resume, re-create, or decline the task.

---

## 4. Task Data Model

### 4.1 Lifecycle States

Canonical states and valid transitions:

- `requested` -- Task has been created by the client and is awaiting worker response.
- `accepted` -- Worker has accepted the task and will begin execution.
- `rejected` -- Worker has declined the task. **Terminal.**
- `working` -- Worker is actively executing the task.
- `input_required` -- Worker needs additional input from the client to continue.
- `completed` -- Worker has finished the task successfully. **Terminal.**
- `failed` -- Task execution encountered an unrecoverable error. **Terminal.**
- `canceled` -- Task was canceled by the client. **Terminal.**

Valid state transitions:

```
requested  -> accepted | rejected | canceled
accepted   -> working  | canceled
working    -> input_required | completed | failed | canceled
input_required -> working | failed | canceled
```

State constraints:
- `accept` is valid only from `requested`.
- `reject` is valid only from `requested`.
- `progress` is valid only in `working` or `input_required`.
- `complete` is valid only from `working`.
- `fail` is valid from `working` or `input_required`.
- `cancel` is valid from any non-terminal state.
- Operations on terminal tasks MUST return the cached terminal result (idempotent).

### 4.2 Task Identifiers

- `task_id`: `bstr .size 16`, globally unique, created by the client. The client MUST generate a cryptographically random 16-byte value. The same `task_id` MUST NOT be reused for different tasks.
- `parent_task_id`: `bstr .size 16`, optional. Present when this task is a sub-task, referencing the parent task that spawned it.
- `input_id`: `tstr`, scoped to a single input request/response exchange within a task. The worker MUST generate a unique `input_id` per input request.

### 4.3 Task Core CDDL

```cddl
task-id = bstr .size 16
unix-ms = uint
did = tstr
semver = tstr

task-status =
  "requested" / "accepted" / "rejected" / "working" /
  "input_required" / "completed" / "failed" / "canceled"

task-milestone = {
  "name": tstr,
  "completed_at": unix-ms,
  ? "data": any
}

subtask-ref = {
  "subtask_id": task-id,
  "worker": did,
  "status": task-status
}

task-record = {
  "task_id": task-id,
  "client": did,
  "worker": did,
  "status": task-status,
  "created_at": unix-ms,
  "updated_at": unix-ms,
  ? "description": tstr,
  ? "input_schema": any,
  ? "output_schema": any,
  ? "milestones": [* task-milestone],
  ? "subtasks": [* subtask-ref],
  ? "result": any,
  ? "error": { "code": uint, ? "message": tstr },
  ? "payment_id": bstr .size 16,
  ? "terms_id": bstr .size 16,
  ? "parent_task_id": task-id
}

task-session-context = {
  "session_id": bstr .size 16,
  "session_scope": true
}

task-base = {
  "profile": "xyz.agentries.task",
  "profile_v": "1.0.0",
  "action": tstr,
  "task_id": task-id,
  ? "session": task-session-context
}
```

**Field Notes**:
- `task-record` is an informational data model describing the full state of a task. It is not carried in messages directly; individual action bodies carry subsets of these fields.
- `task-base` is the common structure shared by all task action bodies. The `profile`, `profile_v`, `action`, and `task_id` fields are REQUIRED in every task message body.
- `task-session-context` follows RFC 006 session scoping semantics. When present, the task is associated with the named session, but task lifecycle is independent of session lifecycle.

---

## 5. Task Protocol Semantics

### 5.1 Message Type Usage and Dispatch

Task messages use the profile-body format defined in RFC 001 Section 4.3. Dispatch rules:

- Task messages MUST use `typ = 0x80` when the peer declared this profile with `typ` in HELLO negotiation. If the peer did not declare `typ`, or no HELLO was exchanged, the sender MUST use `typ = 0xF0` with `body.profile = "xyz.agentries.task"` (per RFC 001 Section 13.3 typ selection rule).
- Receivers supporting this profile MUST accept messages via either `typ = 0x80` OR `typ = 0xF0` with matching `body.profile = "xyz.agentries.task"` (dual-accept per RFC 001 Section 13.3).
- A message is task-protocol only when `typ` is `0x80` or (`typ` is `0xF0` and the signed body contains `"profile": "xyz.agentries.task"`).
- Messages with `typ = 0x80` but `body.profile` not equal to `"xyz.agentries.task"` MUST be rejected with `4001 BAD_REQUEST`.
- `action` and `task_id` are REQUIRED in every task body; missing or invalid fields MUST be rejected with `1001 INVALID_MESSAGE`.
- Unknown `action` values MUST be rejected with `4205 UNKNOWN_TASK_ACTION`.
- `profile_v` MUST be `"1.0.0"` for this version of the protocol. Unsupported `profile_v` values MUST be rejected with `4006 PROFILE_VERSION_UNSUPPORTED`.

**HELLO declaration**: Implementations supporting this profile SHOULD declare it in HELLO `profiles`:

```json
{
  "name": "xyz.agentries.task",
  "version": "1.0.0",
  "typ": 128
}
```

**Example: Diagnostic JSON representation of a task create message envelope**:

```json
{
  "v": 1,
  "id": "h'0123456789abcdef0123456789abcdef'",
  "typ": 128,
  "ts": 1740268800000,
  "ttl": 300000,
  "from": "did:key:z6MkClientAgent...",
  "to": "did:key:z6MkWorkerAgent...",
  "body": {
    "profile": "xyz.agentries.task",
    "profile_v": "1.0.0",
    "action": "create",
    "task_id": "h'aabbccddeeff00112233445566778899'",
    "worker": "did:key:z6MkWorkerAgent...",
    "description": "Summarize the Q4 financial report",
    "input": {
      "document_url": "https://example.com/reports/q4-2025.pdf"
    },
    "output_schema": {
      "type": "object",
      "properties": {
        "summary": { "type": "string" },
        "key_figures": { "type": "array" }
      }
    },
    "priority": 2
  },
  "sig": "h'...64 bytes...'"
}
```

> Note: This is a diagnostic JSON rendering. The actual wire format is deterministic CBOR (RFC 8949) with `sig` computed over `Sig_Input` per RFC 001 Section 8.1.

### 5.2 Create Flow

The client initiates a task by sending `action: "create"`.

**Sender**: Task Client.
**Receiver**: Task Worker.

Semantics:
- The client MUST generate a cryptographically random `task_id` and include it in the body.
- The `worker` field MUST match the envelope `to` field.
- The client MAY include `description`, `input`, `input_schema`, `output_schema`, `payment_id`, `terms_id`, `deadline`, and `priority` fields.
- On receipt, the worker MUST validate the body shape, verify authorization, and transition the task to `requested` state.
- The worker MUST respond with either `accept` or `reject`.
- If the worker cannot process the create within a reasonable time, it SHOULD acknowledge receipt (via AMP ACK per RFC 001 Section 16) and respond asynchronously.

### 5.3 Accept / Reject Flow

The worker responds to a `create` with either `accept` or `reject`.

**Sender**: Task Worker.
**Receiver**: Task Client.

Accept semantics:
- `action: "accept"` transitions the task from `requested` to `accepted`.
- The worker MAY include `estimated_completion` as advisory guidance.
- `reply_to` in the AMP envelope MUST reference the `id` of the `create` message.

Reject semantics:
- `action: "reject"` transitions the task from `requested` to `rejected` (terminal).
- The worker MAY include `reason_code` and `reason` for diagnostic purposes.
- `reply_to` in the AMP envelope MUST reference the `id` of the `create` message.
- After rejection, the `task_id` MUST NOT be reused. The client MUST create a new task with a new `task_id` if retrying.

### 5.4 Progress Flow

The worker reports execution progress to the client.

**Sender**: Task Worker.
**Receiver**: Task Client.

Semantics:
- `action: "progress"` is valid when the task is in `working` or `input_required` state.
- The worker MAY include `percent` (0-100), `message`, and/or `milestone`.
- `percent` MUST be a non-negative integer not exceeding 100. Values outside this range MUST be rejected with `1001 INVALID_MESSAGE`.
- Progress messages are informational; no response is required from the client.
- Multiple progress messages MAY be sent for the same task. The `percent` value SHOULD be monotonically non-decreasing but this is not enforced.
- Milestones provide named checkpoints. Each milestone carries a `name`, `completed_at` timestamp, and optional `data`.

### 5.5 Input Request / Input Response Flow

The worker requests additional input from the client during execution.

**Input Request**:
- **Sender**: Task Worker.
- **Receiver**: Task Client.
- `action: "input_request"` transitions the task from `working` to `input_required`.
- The worker MUST include a unique `input_id` to correlate the request with its response.
- The worker MAY include `prompt` (human-readable description of needed input), `schema` (machine-readable schema for the expected data), and `deadline` (timestamp after which the input request expires).
- If `deadline` is present, the client SHOULD respond before the deadline. After the deadline, the worker MAY fail the task with `4204 INPUT_REQUEST_EXPIRED`.

**Input Response**:
- **Sender**: Task Client.
- **Receiver**: Task Worker.
- `action: "input_response"` transitions the task from `input_required` back to `working`.
- The client MUST include the `input_id` from the corresponding `input_request`.
- The client MUST include `data` containing the requested information.
- `reply_to` in the AMP envelope MUST reference the `id` of the `input_request` message.
- Mismatched `input_id` MUST be rejected with `4001 BAD_REQUEST`.

### 5.6 Complete Flow

The worker signals successful task completion.

**Sender**: Task Worker.
**Receiver**: Task Client.

Semantics:
- `action: "complete"` transitions the task from `working` to `completed` (terminal).
- The worker MAY include `result` containing the task output and `milestones` summarizing completed checkpoints.
- Once `completed`, subsequent operations on this `task_id` MUST return the cached terminal result.
- If `output_schema` was provided in the `create`, the `result` SHOULD conform to that schema. Schema validation is the responsibility of the client; the worker SHOULD make a best effort to conform.

### 5.7 Fail Flow

The worker signals unrecoverable task failure.

**Sender**: Task Worker.
**Receiver**: Task Client.

Semantics:
- `action: "fail"` transitions the task from `working` or `input_required` to `failed` (terminal).
- The worker MUST include `error_code` (a numeric code from the 42xx range or a general AMP error code).
- The worker MAY include `message` (human-readable error description) and `details` (machine-readable error context).
- Once `failed`, subsequent operations on this `task_id` MUST return the cached terminal result.

### 5.8 Cancel / Cancel Result Flow

The client requests task cancellation.

**Cancel**:
- **Sender**: Task Client.
- **Receiver**: Task Worker.
- `action: "cancel"` requests cancellation of a task in any non-terminal state.
- The client MAY include `reason` for diagnostic purposes.
- Cancel on a terminal task MUST return the cached terminal result (idempotent if already `canceled`, or `4202 INVALID_STATE_TRANSITION` if in another terminal state).

**Cancel Result**:
- **Sender**: Task Worker.
- **Receiver**: Task Client.
- `action: "cancel_result"` confirms the outcome of the cancellation request.
- `status` MUST be `"canceled"` (cancellation succeeded) or `"failed"` (cancellation could not be performed, for example because the task completed concurrently).
- `reply_to` in the AMP envelope MUST reference the `id` of the `cancel` message.
- If `status` is `"canceled"`, the task transitions to `canceled` (terminal).
- The worker MAY include `reason` explaining the cancel result.

### 5.8a Action Direction Matrix

The following matrix is normative:

| Action | Sender -> Receiver | Response Action | `reply_to` requirement |
|--------|--------------------|-----------------|------------------------|
| `create` | client -> worker | `accept` / `reject` | `accept.reply_to` or `reject.reply_to` MUST reference triggering `create.id` |
| `accept` | worker -> client | (no response required) | N/A |
| `reject` | worker -> client | (no response required) | N/A |
| `progress` | worker -> client | (no response required) | N/A |
| `input_request` | worker -> client | `input_response` | `input_response.reply_to` MUST reference triggering `input_request.id` |
| `input_response` | client -> worker | (no response required) | N/A |
| `complete` | worker -> client | (no response required) | N/A |
| `fail` | worker -> client | (no response required) | N/A |
| `cancel` | client -> worker | `cancel_result` | `cancel_result.reply_to` MUST reference triggering `cancel.id` |
| `status_query` | either -> either | `status` | `status.reply_to` MUST reference triggering `status_query.id` |
| `subtask_create` | worker -> sub-worker | `accept` / `reject` | same as `create` direction/correlation rules |
| `subtask_result` | sub-worker -> worker | (no response required) | N/A |

Rules:
- This section applies to Standard Profile and Private Profile carriage parsed by Section 5.1 (`typ` in `0x80`/`0xF0` with signed body `profile = "xyz.agentries.task"`).
- CAP carriage uses `CAP_INVOKE`/`CAP_RESULT` semantics in Section 5.11 and RFC 004.
- Action/direction mismatch (for example, a client sending `accept`) MUST be rejected with `4001 BAD_REQUEST`.
- Unsolicited response actions (missing or invalid `reply_to`) MUST be rejected with `4001 BAD_REQUEST`.

### 5.9 Status Query / Status Flow

Either party queries the current state of a task.

**Status Query**:
- **Sender**: Task Client or Task Worker.
- **Receiver**: Counterparty.
- `action: "status_query"` requests the current task state.
- No additional fields beyond `task-base` are required.

**Status**:
- **Sender**: Queried party.
- **Receiver**: Requester.
- `action: "status"` returns the current task state.
- `status` and `updated_at` are REQUIRED.
- The responder MAY include `percent`, `milestones`, `subtasks`, `result` (if terminal), and `error` (if failed).
- `reply_to` in the AMP envelope MUST reference the `id` of the `status_query` message.
- `status_query` for an unknown `task_id` MUST be rejected with `4201 TASK_NOT_FOUND`.

### 5.10 Subtask Create / Subtask Result Flow

A worker delegates part of its work to another agent via sub-tasks.

**Subtask Create**:
- **Sender**: Task Worker (acting as sub-task client).
- **Receiver**: Sub-task Worker.
- `action: "subtask_create"` creates a new task that is a child of the sender's parent task.
- The sender MUST include `parent_task_id` referencing its own task and `worker` identifying the sub-task worker.
- The `task_id` in the sub-task MUST be a new, unique identifier generated by the sender.
- The sub-task worker receives this as a task creation and MUST respond with `accept` or `reject` following the same semantics as Section 5.3.

**Subtask Result**:
- **Sender**: Sub-task Worker.
- **Receiver**: Parent Task Worker.
- `action: "subtask_result"` reports the terminal outcome of a sub-task.
- `parent_task_id` MUST reference the parent task.
- `status` MUST be `"completed"` or `"failed"`.
- The sub-worker MAY include `result` (on success) or `error` (on failure).

**Cascading cancellation**:
- When a parent task is canceled, the parent worker MUST send `cancel` to all active (non-terminal) sub-tasks.
- Sub-task cancellation follows the same cancel/cancel_result flow as Section 5.8.
- Sub-task failure does not automatically fail the parent task; the parent worker decides how to handle sub-task failures.

### 5.11 CAP_INVOKE Interop Profile

This section defines the RFC 012 capability interoperability baseline for the RFC 004 invocation path.

Capability identity:
- `id = "xyz.agentries.task.workflow:1.0.0"`.

Rules:
- When using the CAP path, `CAP_INVOKE` MUST target the capability ID above.
- `CAP_INVOKE.params` MUST contain one task operation body from this RFC with `profile = "xyz.agentries.task"` and `profile_v = "1.0.0"`.
- Allowed request actions in CAP path: `create`, `cancel`, `input_response`, `status_query`, `subtask_create`.
- `CAP_RESULT(status="success").result` MUST contain the corresponding response body from this RFC.
- If `CAP_INVOKE.body.delegation` is present, validation MUST follow RFC 005 before task execution.
- Invalid/unsupported delegation evidence in the CAP task path MUST fail with `3004 DELEGATION_INVALID` (RFC 004/005).
- Section 5.8a profile-carriage direction rules MUST NOT be applied to CAP envelope types. CAP carriage uses `CAP_INVOKE`/`CAP_RESULT` semantics.
- Providers supporting this capability MUST publish an RFC 004-compliant descriptor for `xyz.agentries.task.workflow:1.0.0`.
- Descriptor input schema MUST correspond to `app-cap-invoke-params`; success result schema MUST correspond to `app-cap-result-success`.
- Pre-execution rejection in CAP path (validation/authorization/compatibility/schema) MUST return AMP `ERROR` per RFC 004 Section 7.2.
- Only post-accept execution failures in CAP path MAY return `CAP_RESULT(status="error")`.

### 5.12 Task Body CDDL

```cddl
task-action =
  "create" / "accept" / "reject" / "progress" /
  "input_request" / "input_response" /
  "complete" / "fail" /
  "cancel" / "cancel_result" /
  "status_query" / "status" /
  "subtask_create" / "subtask_result"

create-body = {
  task-base,
  "action": "create",
  "worker": did,
  ? "description": tstr,
  ? "input": any,
  ? "input_schema": any,
  ? "output_schema": any,
  ? "payment_id": bstr .size 16,
  ? "terms_id": bstr .size 16,
  ? "deadline": unix-ms,
  ? "priority": uint
}

accept-body = {
  task-base,
  "action": "accept",
  ? "estimated_completion": unix-ms
}

reject-body = {
  task-base,
  "action": "reject",
  ? "reason_code": uint,
  ? "reason": tstr
}

progress-body = {
  task-base,
  "action": "progress",
  ? "percent": uint,          ; 0-100
  ? "message": tstr,
  ? "milestone": task-milestone
}

input-request-body = {
  task-base,
  "action": "input_request",
  "input_id": tstr,
  ? "prompt": tstr,
  ? "schema": any,
  ? "deadline": unix-ms
}

input-response-body = {
  task-base,
  "action": "input_response",
  "input_id": tstr,
  "data": any
}

complete-body = {
  task-base,
  "action": "complete",
  ? "result": any,
  ? "milestones": [* task-milestone]
}

fail-body = {
  task-base,
  "action": "fail",
  "error_code": uint,
  ? "message": tstr,
  ? "details": any
}

cancel-body = {
  task-base,
  "action": "cancel",
  ? "reason": tstr
}

cancel-result-body = {
  task-base,
  "action": "cancel_result",
  "status": "canceled" / "failed",
  ? "reason": tstr
}

status-query-body = {
  task-base,
  "action": "status_query"
}

status-body = {
  task-base,
  "action": "status",
  "status": task-status,
  "updated_at": unix-ms,
  ? "percent": uint,
  ? "milestones": [* task-milestone],
  ? "subtasks": [* subtask-ref],
  ? "result": any,
  ? "error": { "code": uint, ? "message": tstr }
}

subtask-create-body = {
  task-base,
  "action": "subtask_create",
  "parent_task_id": task-id,
  "worker": did,
  ? "description": tstr,
  ? "input": any
}

subtask-result-body = {
  task-base,
  "action": "subtask_result",
  "parent_task_id": task-id,
  "status": "completed" / "failed",
  ? "result": any,
  ? "error": { "code": uint, ? "message": tstr }
}

app-capability-id = "xyz.agentries.task.workflow:1.0.0"

app-cap-invoke-params =
  create-body /
  cancel-body /
  input-response-body /
  status-query-body /
  subtask-create-body

app-cap-result-success =
  accept-body /
  reject-body /
  cancel-result-body /
  status-body /
  complete-body /
  fail-body
```

### 5.13 Idempotency and Replay Rules

Task logic MUST remain safe under RFC 003 at-least-once delivery.

Operation idempotency key:
- `op_key = (task_id, action, from_did)`.

Rules:
- This section applies to both profile carriage (Section 5.1) and CAP carriage (Section 5.11).
- In CAP carriage, `task_id` and `action` are read from `CAP_INVOKE.params`.
- Replaying a message with the same semantic body under the same `op_key` MUST return a deterministic equivalent result and MUST NOT create duplicate lifecycle transitions.
- Replaying with the same `op_key` but conflicting semantic body (for example changed `description`, `input`, or `worker` fields) MUST be rejected with `4001 BAD_REQUEST`.
- `create` replay with identical body after task already exists MUST return the current task state (idempotent).
- `cancel` replay after `canceled` MUST return prior terminal-equivalent `cancel_result`.
- `complete` replay after `completed` MUST return cached terminal result.
- `status_query` is read-only and MAY be retried freely; unknown `task_id` MUST map to `4201 TASK_NOT_FOUND`.
- `input_response` replay with the same `input_id` and `data` after already processed MUST be idempotent.

---

## 6. Relationship to RFC 006 Provisional Responses

RFC 006 defines transient provisional responses (PROCESSING, PROGRESS, INPUT_REQUIRED) within a session/request context. This RFC defines persistent, queryable task state that survives session boundaries. The key differences:

| Aspect | RFC 006 Provisionals | RFC 012 Task Progress |
|--------|----------------------|------------------------|
| Lifetime | Tied to a single REQUEST/RESPONSE exchange | Persists across requests, sessions, and connections |
| Queryable | No -- lost after request completes | Yes -- via `status_query` at any time |
| Scope | Single REQUEST/RESPONSE pair | Full task lifecycle from creation to terminal state |
| Sub-delegation | Not supported | Via `subtask_create` / `subtask_result` |
| State machine | Request-scoped (PROCESSING -> PROGRESS -> final RESPONSE) | Task-scoped (Section 7 state machine) |
| Message type | RFC 006 provisional type codes | `typ = 0x80` with `profile = "xyz.agentries.task"` |
| Cancellation | Not applicable (request completes or times out) | Explicit `cancel` / `cancel_result` flow |

**Coexistence**: Workers MAY emit both RFC 006 PROCESSING/PROGRESS provisional responses for individual messages within a session AND RFC 012 `progress` actions for the overall task lifecycle. These are complementary, not conflicting:
- RFC 006 provisionals give real-time feedback within a single request.
- RFC 012 progress gives persistent lifecycle tracking across the full task.

---

## 7. State Machines

### 7.1 Task State Machine

```
REQUESTED
  +-- accept -----------> ACCEPTED
  +-- reject -----------> REJECTED (terminal)
  +-- cancel -----------> CANCELED (terminal) [via cancel_result]

ACCEPTED
  +-- (worker begins) --> WORKING
  +-- cancel -----------> CANCELED (terminal) [via cancel_result]

WORKING
  +-- input_request ----> INPUT_REQUIRED
  +-- complete ---------> COMPLETED (terminal)
  +-- fail -------------> FAILED (terminal)
  +-- cancel -----------> CANCELED (terminal) [via cancel_result]

INPUT_REQUIRED
  +-- input_response ---> WORKING
  +-- fail -------------> FAILED (terminal)
  +-- cancel -----------> CANCELED (terminal) [via cancel_result]
```

Terminal states: `REJECTED`, `COMPLETED`, `FAILED`, `CANCELED`.

### 7.2 Terminal State Rules

- Operations on terminal tasks MUST return the cached terminal result (idempotent).
- A `cancel` on a `completed` task MUST be rejected with `4202 INVALID_STATE_TRANSITION` (the task succeeded; cancellation is meaningless).
- A `cancel` on a `canceled` task MUST return the cached `cancel_result` (idempotent).
- A `create` replay for an existing `task_id` MUST return the current task state (idempotent, not a new task).
- New `task_id` values MUST NOT be recycled from terminal tasks.

### 7.3 Transition Validation

Implementations MUST validate state transitions before applying them. Invalid transitions MUST be rejected with `4202 INVALID_STATE_TRANSITION`. The transition table in Section 4.1 is exhaustive; any transition not listed is invalid.

---

## 8. Error Handling and Retry

This RFC reuses the RFC 001 error model and introduces task-specific business codes in the `42xx` range.

Deterministic precedence (evaluated in order):
1. Parse/shape/type failures (missing required fields, invalid CBOR) -> `1001 INVALID_MESSAGE`.
2. Profile/typ mismatch (`typ = 0x80` but `body.profile` not `"xyz.agentries.task"`) -> `4001 BAD_REQUEST`.
3. Unknown profile (message via `typ = 0xF0` with unrecognized `body.profile`) -> `4005 UNKNOWN_PROFILE`.
4. Unsupported `profile_v` -> `4006 PROFILE_VERSION_UNSUPPORTED`.
5. Authorization identity/policy failure -> `3001 UNAUTHORIZED`.
6. Action/typ direction mismatch or malformed correlation -> `4001 BAD_REQUEST`.
7. Task semantic/state failures -> `42xx`.
8. Transient backend failures -> `500x`.

| Condition | Code | Name | Retry |
|-----------|------|------|-------|
| Malformed task body (missing required fields, invalid CBOR shape) | `1001` | INVALID_MESSAGE | No |
| Unsupported `profile_v` | `4006` | PROFILE_VERSION_UNSUPPORTED | No |
| Unknown profile (`body.profile` not recognized) | `4005` | UNKNOWN_PROFILE | No |
| Unauthorized task actor (sender not authorized for this task) | `3001` | UNAUTHORIZED | No |
| Task action/typ direction mismatch | `4001` | BAD_REQUEST | No |
| Unsolicited response (missing/invalid `reply_to`) | `4001` | BAD_REQUEST | No |
| Conflicting retry payload for same `op_key` | `4001` | BAD_REQUEST | No |
| Task not found (`task_id` unknown) | `4201` | TASK_NOT_FOUND | No |
| Invalid state transition (action not valid for current state) | `4202` | INVALID_STATE_TRANSITION | No |
| Sub-task limit exceeded (implementation-defined maximum) | `4203` | SUBTASK_LIMIT_EXCEEDED | No |
| Input request expired (`deadline` passed) | `4204` | INPUT_REQUEST_EXPIRED | No |
| Unknown task action (`action` value not recognized) | `4205` | UNKNOWN_TASK_ACTION | No |
| Internal task engine failure | `5001` | INTERNAL_ERROR | Yes |
| Task worker unavailable | `5002` | UNAVAILABLE | Yes |

Registry note:
- `42xx` task codes MUST be registered via the RFC 001 Section 17 process before this RFC advances beyond Draft status.

---

## 9. Versioning and Compatibility

Version dimensions:
- AMP envelope version `v` remains governed by RFC 001.
- Task profile version is `profile_v`, currently `"1.0.0"`.

Compatibility rules:
- Unknown required fields MUST fail with `1001 INVALID_MESSAGE`.
- Unknown optional fields MAY be ignored unless security-sensitive.
- Backward-compatible extensions MUST use optional fields only and MUST NOT change the semantics of existing fields.
- Minor version increments (for example `"1.1.0"`) indicate backward-compatible additions.
- Major version increments (for example `"2.0.0"`) indicate breaking changes and MUST be negotiated via HELLO profile matching (RFC 001 Section 13.3).

Profile negotiation:
- Implementations SHOULD declare `xyz.agentries.task` in HELLO `profiles` field for connection-time compatibility discovery.
- If the peer declares `typ = 0x80` for this profile in HELLO, senders MUST use `typ = 0x80`.
- If the peer does not declare `typ` or no HELLO was exchanged, senders MUST use `typ = 0xF0` (Private Profile fallback).
- Receivers MUST accept both `typ = 0x80` and `typ = 0xF0` with matching `body.profile` (dual-accept rule).

---

## 10. Security Considerations

- **Replay safety**: Implementations MUST enforce RFC 001 idempotency with stable `task_id` semantics. The `op_key = (task_id, action, from_did)` tuple ensures replayed messages do not create duplicate transitions.
- **Tamper protection**: All task data is carried in the signed body. The `profile`, `action`, `profile_v`, `task_id`, and all payload fields are integrity-protected by the AMP envelope signature.
- **Authorization**: Workers MUST verify that the client is authorized to create tasks before accepting. Clients MUST verify that the worker is the expected agent before processing responses. Principal/from binding from RFC 002 applies to all task operations.
- **Privilege escalation via sub-tasks**: Sub-task delegation MUST NOT escalate privileges. A sub-task worker MUST NOT receive greater authorization than the parent task worker holds. Implementations SHOULD enforce delegation scope constraints when creating sub-tasks.
- **Cancellation authentication**: Only the original client (or an authorized delegate per RFC 005) MAY cancel a task. Workers MUST verify the `from` field of cancel messages against the task's `client` DID.
- **Input request abuse**: Workers could abuse `input_request` to phish for sensitive data. Clients SHOULD validate `input_request` fields against the original task description and SHOULD NOT blindly provide requested data without policy checks.
- **Task enumeration**: `status_query` reveals task existence. Implementations SHOULD enforce authorization on `status_query` to prevent task enumeration by unauthorized agents.

---

## 11. Privacy Considerations

- **Business intent exposure**: Task descriptions and input/output data may reveal business strategies, confidential information, or user intent. Implementations SHOULD apply least-privilege principles to task data access.
- **Data minimization**: Minimize retention of task input and output data after task completion. Terminal task records SHOULD be prunable according to implementation-specific retention policies.
- **Logging**: Logs SHOULD prefer task IDs and status codes over full descriptions and payloads where possible. Sensitive fields (input data, result data) SHOULD be redacted or omitted from logs.
- **Sub-task visibility**: Sub-task delegation reveals the worker's decomposition strategy to sub-task workers. Workers SHOULD minimize the information disclosed in sub-task descriptions.
- **Payment and terms linkage**: The presence of `payment_id` or `terms_id` in task bodies links task activity to economic transactions. Implementations SHOULD treat these cross-references as sensitive metadata.

---

## 12. Implementation Checklist

- [ ] Implement task dispatch guard (Section 5.1): validate `typ`, `body.profile`, `action`, `task_id`.
- [ ] Implement dual-accept for `typ = 0x80` and `typ = 0xF0` with `body.profile = "xyz.agentries.task"`.
- [ ] Implement HELLO profile declaration for `xyz.agentries.task`.
- [ ] Implement action direction matrix (Section 5.8a): validate sender role and `reply_to` correlation.
- [ ] Implement all Section 5.12 action body schemas with CDDL validation.
- [ ] Enforce lifecycle state transitions (Section 7): reject invalid transitions with `4202`.
- [ ] Enforce deterministic error mapping (Section 8): apply precedence order.
- [ ] Implement idempotency rules (Section 5.13): `op_key` deduplication and deterministic replay.
- [ ] Implement `status_query` / `status` flow for active and terminal tasks.
- [ ] Optional: Implement CAP interop profile (Section 5.11) with `xyz.agentries.task.workflow:1.0.0`.
- [ ] Optional: Implement sub-task coordination (Section 5.10) with cascading cancellation.
- [ ] Optional: Implement `payment_id` and `terms_id` cross-reference fields.
- [ ] Add conformance tests from Appendix A.

---

## 13. References

### 13.1 Normative References

- RFC 001: Agent Messaging Protocol (Core)
- RFC 2119: Key words for use in RFCs to Indicate Requirement Levels
- RFC 8174: Ambiguity of Uppercase vs Lowercase in RFC 2119 Key Words
- RFC 8949: Concise Binary Object Representation (CBOR)
- RFC 8610: Concise Data Definition Language (CDDL)

### 13.2 Informative References

- RFC 002: Transport Bindings
- RFC 003: Relay and Store-and-Forward
- RFC 004: Capability Schema Registry and Compatibility
- RFC 005: Delegation Credentials and Authorization
- RFC 006: Session Protocol
- RFC 007: Agent Payment Protocol
- RFC 013: Negotiation Protocol
- RFC 014: Handoff Protocol

---

## Appendix A. Test Vectors

The following test vectors define minimal conformance scenarios. Implementations MUST pass all positive vectors and MUST produce the specified error codes for negative vectors.

### A.1 Create to Complete Positive

Input:
- Client sends `action: "create"` with valid `task_id`, `worker`, and `description`.
- Worker sends `action: "accept"` with `reply_to` referencing the `create` message `id`.
- Worker sends `action: "progress"` with `percent: 50` and `message: "Processing"`.
- Worker sends `action: "complete"` with `result` payload.

Expected:
- State transitions: `requested -> accepted -> working -> completed`.
- Client receives `complete` with task result.
- Subsequent `status_query` returns `status: "completed"` with cached result.

### A.2 Create to Reject Positive

Input:
- Client sends `action: "create"` with valid `task_id` and `worker`.
- Worker sends `action: "reject"` with `reason_code: 1` and `reason: "Capacity exceeded"`.

Expected:
- State transitions: `requested -> rejected`.
- `reject.reply_to` references the `create` message `id`.
- Task is terminal. Subsequent operations return cached rejection.

### A.3 Progress Reporting Positive

Input:
- Task is in `working` state.
- Worker sends three `action: "progress"` messages with `percent: 25`, `percent: 50`, `percent: 75`.
- Worker sends `action: "progress"` with `percent: 100` and `milestone: {"name": "analysis_done", "completed_at": 1740268900000}`.

Expected:
- All progress messages are accepted without error.
- No response is required from the client.
- `status_query` after last progress returns `percent: 100` and `milestones` containing the reported milestone.

### A.4 Input Request/Response Positive

Input:
- Task is in `working` state.
- Worker sends `action: "input_request"` with `input_id: "clarify-scope"`, `prompt: "Please specify the date range"`, and `schema: {"type": "object", "properties": {"start": {"type": "string"}, "end": {"type": "string"}}}`.
- Client sends `action: "input_response"` with `input_id: "clarify-scope"` and `data: {"start": "2025-01-01", "end": "2025-12-31"}`.
- `input_response.reply_to` references the `input_request` message `id`.

Expected:
- State transitions: `working -> input_required -> working`.
- Worker resumes execution with the provided data.

### A.5 Cancel Active Task Positive

Input:
- Task is in `working` state.
- Client sends `action: "cancel"` with `reason: "No longer needed"`.
- Worker sends `action: "cancel_result"` with `status: "canceled"`.
- `cancel_result.reply_to` references the `cancel` message `id`.

Expected:
- State transitions: `working -> canceled`.
- Task is terminal. Subsequent `status_query` returns `status: "canceled"`.

### A.6 Cancel Already Completed Negative (4202)

Input:
- Task is in `completed` terminal state.
- Client sends `action: "cancel"`.

Expected:
- Reject with `4202 INVALID_STATE_TRANSITION`.
- Task state remains `completed`.

### A.7 Invalid State Transition Negative (4202)

Input:
- Task is in `requested` state (not yet accepted).
- Worker sends `action: "complete"`.

Expected:
- Reject with `4202 INVALID_STATE_TRANSITION`.
- `complete` is only valid from `working` state.

### A.8 Task Not Found Negative (4201)

Input:
- Client sends `action: "status_query"` with a `task_id` that has no known lifecycle record.

Expected:
- Reject with `4201 TASK_NOT_FOUND`.

### A.9 Duplicate Create Idempotent Positive

Input:
- Client sends `action: "create"` with `task_id = X`.
- Task reaches `working` state.
- Client replays `action: "create"` with the same `task_id = X` and identical body.

Expected:
- No duplicate task is created.
- Response returns current task state (`working`).
- Idempotent: no duplicate state transitions.

### A.10 Unknown Action Negative (4205)

Input:
- Client sends a task message with `profile: "xyz.agentries.task"`, `profile_v: "1.0.0"`, and `action: "restart"` (not a defined action).

Expected:
- Reject with `4205 UNKNOWN_TASK_ACTION`.

### A.11 Sub-task Create and Complete Positive

Input:
- Parent task T1 is in `working` state, worker W1.
- W1 sends `action: "subtask_create"` with new `task_id = T2`, `parent_task_id = T1`, `worker = W2`.
- W2 sends `action: "accept"` for T2.
- W2 sends `action: "complete"` for T2 with result.
- W2 sends `action: "subtask_result"` to W1 with `parent_task_id = T1`, `status: "completed"`, and `result`.

Expected:
- T2 lifecycle: `requested -> accepted -> working -> completed`.
- W1 receives `subtask_result` and incorporates sub-task output.
- T1 lifecycle is unaffected by T2 transitions (parent decides independently).

### A.12 Sub-task Cascading Cancel Positive

Input:
- Parent task T1 is in `working` state with active sub-task T2 (also `working`).
- Client cancels T1.
- Worker W1 receives `cancel` for T1.
- W1 sends `cancel` for T2 to W2 (cascading).
- W2 sends `cancel_result` with `status: "canceled"` for T2.
- W1 sends `cancel_result` with `status: "canceled"` for T1.

Expected:
- T2 transitions to `canceled` before T1 finalization.
- T1 transitions to `canceled`.
- Both tasks are terminal.

### A.13 Input Request Expired Negative (4204)

Input:
- Worker sends `action: "input_request"` with `deadline: 1740268800000`.
- Client sends `action: "input_response"` after the deadline has passed (current time > deadline).

Expected:
- Worker rejects with `4204 INPUT_REQUEST_EXPIRED`.
- Worker MAY transition the task to `failed` with `error_code: 4204`.

### A.14 Unauthorized Cancel Negative (3001)

Input:
- Task T1 was created by client C1.
- Agent C2 (not C1, not an authorized delegate) sends `action: "cancel"` for T1.

Expected:
- Reject with `3001 UNAUTHORIZED`.
- Task state is unchanged.

### A.15 Malformed Task Body Negative (1001)

Input:
- Message with `typ = 0x80` and `body.profile = "xyz.agentries.task"` but missing `task_id` field.

Expected:
- Reject with `1001 INVALID_MESSAGE`.

### A.16 Profile Version Unsupported Negative (4006)

Input:
- Message with `typ = 0x80`, `body.profile = "xyz.agentries.task"`, and `body.profile_v = "2.0.0"` (unsupported).

Expected:
- Reject with `4006 PROFILE_VERSION_UNSUPPORTED`.
- Responder MAY include supported versions in error `details`.

### A.17 Status Query on Active Task Positive

Input:
- Task is in `working` state with `percent: 60` and two milestones completed.
- Client sends `action: "status_query"`.
- Worker sends `action: "status"` with `status: "working"`, `updated_at`, `percent: 60`, and `milestones` array.

Expected:
- `status.reply_to` references the `status_query` message `id`.
- Response contains current state, progress percentage, and milestone history.
- No state transition occurs (read-only operation).

---

## Appendix B. Open Questions

1. **Task priority semantics**: Should `priority` in `create-body` be normative (workers MUST respect ordering) or advisory (workers MAY use it for scheduling hints)? Current draft treats it as advisory.
2. **Maximum sub-task depth**: Should there be a normative maximum sub-task nesting depth to prevent unbounded delegation chains? Current draft leaves this implementation-defined via `4203 SUBTASK_LIMIT_EXCEEDED`.
3. **Deadline-triggered transitions**: Should task deadlines (`deadline` field) trigger automatic state transitions (for example, auto-fail on expiry), or should they remain advisory hints that workers MAY enforce? Current draft leaves enforcement to the worker.
4. **Task transfer**: Should a mechanism exist for transferring a task from one worker to another without canceling and re-creating? This may be addressed by RFC 014 context handoff.
5. **Batch task creation**: Should there be a batch variant of `create` for submitting multiple related tasks atomically? Current draft requires individual `create` per task.

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2026-02-22 | 0.1 | Nowa | Initial draft: task lifecycle semantics using profile-body format (typ=0x80), state machine, CDDL schemas for all 14 actions, sub-task coordination, CAP interop profile (xyz.agentries.task.workflow:1.0.0), error codes 42xx, 17 test vectors, dual-accept dispatch, idempotency rules, boundary contracts with RFC 001-007/013/014 |
