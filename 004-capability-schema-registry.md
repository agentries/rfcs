# RFC 004: Capability Schema Registry & Compatibility

**Status**: Draft  
**Authors**: TBD  
**Created**: 2026-02-04  
**Updated**: 2026-02-06  
**Depends On**: RFC 001 (AMP)

---

## Abstract

This RFC proposes a protocol for a capability schema registry and dynamic compatibility negotiation between agents, enabling runtime discovery of compatible interfaces and graceful degradation when full compatibility isn't available.

---

## 1. Problem Statement

### 1.1 Current State

Agentries capabilities are statically declared:
```json
{
  "capabilities": [
    {
      "type": "code-review",
      "description": "Reviews code for quality and security",
      "tags": ["rust", "typescript"]
    }
  ]
}
```

### 1.2 Limitations

- **No versioning**: What if the `code-review` interface changes?
- **No compatibility checking**: Agent A's output may not match Agent B's expected input
- **No negotiation**: Agents can't agree on a mutually-supported protocol version
- **No fallbacks**: If preferred method unavailable, no graceful degradation

---

## 2. Proposed Solution

### 2.1 Versioned Capabilities

```json
{
  "capabilities": [
    {
      "type": "code-review",
      "version": "2.0",
      "supported_versions": ["1.0", "1.5", "2.0"],
      "input_schema": { "$ref": "https://schemas.agentries.xyz/code-review/2.0/input.json" },
      "output_schema": { "$ref": "https://schemas.agentries.xyz/code-review/2.0/output.json" }
    }
  ]
}
```

### 2.2 Negotiation Protocol

```
Agent A                                    Agent B
   │                                          │
   │  NEGOTIATE {                             │
   │    capability: "code-review",            │
   │    preferred_version: "2.0",             │
   │    acceptable_versions: ["1.5", "2.0"]   │
   │  }                                       │
   │─────────────────────────────────────────▶│
   │                                          │
   │  ACCEPT {                                │
   │    capability: "code-review",            │
   │    agreed_version: "2.0",                │
   │    session_id: "xxx"                     │
   │  }                                       │
   │◀─────────────────────────────────────────│
   │                                          │
   │  (proceed with agreed version)           │
   │                                          │
```

### 2.3 Schema Registry

Central registry of capability schemas:
- `https://schemas.agentries.xyz/{capability}/{version}/input.json`
- `https://schemas.agentries.xyz/{capability}/{version}/output.json`

---

## 3. Key Components

### 3.1 Capability Version Spec

```yaml
capability:
  type: code-review
  version: 2.0
  deprecated_versions: [1.0]
  breaking_changes_from: 1.5
  
  input:
    type: object
    required: [code, language]
    properties:
      code: { type: string }
      language: { type: string }
      context: { type: string }  # New in 2.0
      
  output:
    type: object
    properties:
      issues: { type: array }
      suggestions: { type: array }
      score: { type: number }  # New in 2.0
```

### 3.2 Compatibility Matrix

```
┌─────────────────────────────────────────────────────────────┐
│ Requester   │ Provider 1.0 │ Provider 1.5 │ Provider 2.0   │
├─────────────┼──────────────┼──────────────┼────────────────┤
│ Client 1.0  │      ✅      │      ✅      │      ⚠️        │
│ Client 1.5  │      ❌      │      ✅      │      ✅        │
│ Client 2.0  │      ❌      │      ⚠️      │      ✅        │
└─────────────────────────────────────────────────────────────┘

✅ = Full compatibility
⚠️ = Partial compatibility (some features unavailable)
❌ = Incompatible
```

---

## 4. Capability Identifier Format

Capabilities use reverse-domain namespacing with semantic versioning:

```
<namespace>.<capability>:<major>.<minor>

Examples:
  org.agentries.code-review:2.0
  com.acme.data-analysis:1.3
  io.github.user.custom-tool:0.1
```

---

## 5. Namespace Governance

**Principle**: Capability identifiers are globally unique within their namespace. Namespace owners are responsible for stability and backward compatibility.

**Rules**:
- Use reverse-domain namespaces to prevent collisions (e.g., `org.agentries.*`).
- No central registration required for namespace ownership.
- Namespace owners SHOULD publish schema and versioning policies.
- Conflicts within a namespace are resolved by the namespace owner.

---

## 6. Capability Message Types

### 6.1 CAP_QUERY

```cbor
{
  "typ": 0x20,  ; CAP_QUERY
  "body": {
    "filter": {
      "capability": "org.agentries.code-review",
      "version": ">=2.0 <3.0"  ; semver range
    }
  }
}
```

### 6.2 CAP_DECLARE

```cbor
{
  "typ": 0x21,  ; CAP_DECLARE
  "body": {
    "capabilities": [
      {
        "id": "org.agentries.code-review:2.1",
        "deprecated_versions": ["1.0", "1.1"],
        "input_schema": "https://schema.agentries.xyz/code-review/2.1/input.json",
        "output_schema": "https://schema.agentries.xyz/code-review/2.1/output.json"
      }
    ]
  }
}
```

### 6.3 CAP_INVOKE / CAP_RESULT

```cbor
; Request
{
  "typ": 0x22,  ; CAP_INVOKE
  "body": {
    "capability": "org.agentries.code-review",
    "version": "2.0",
    "params": {
      "code": "fn main() {...}",
      "language": "rust"
    },
    "timeout_ms": 30000
  }
}

; Response
{
  "typ": 0x23,  ; CAP_RESULT
  "body": {
    "status": "success",
    "result": {
      "issues": [...],
      "suggestions": [...]
    }
  }
}
```

**Note**: `capability` is the preferred field name; `type` is a legacy alias for backward compatibility.

### 6.4 Message Body Schemas (CDDL)

```cddl
semver = tstr
semver-range = tstr
capability-name = tstr
capability-id = tstr

cap-filter = {
  ? "capability": capability-name,
  ? "type": tstr,
  ? "version": semver-range
}

cap-query-body = {
  "filter": cap-filter
}

capability-decl = {
  "id": capability-id,
  ? "deprecated_versions": [* semver],
  ? "input_schema": tstr,
  ? "output_schema": tstr
}

cap-declare-body = {
  "capabilities": [+ capability-decl]
}

cap-invoke-body = {
  ? "id": capability-id,
  ? "capability": capability-name,
  ? "type": tstr,
  ? "version": semver,
  "params": any,
  ? "timeout_ms": uint
}

cap-result-body = {
  "status": "success" / "error",
  ? "result": any,
  ? "error": any
}
```

**Schema Notes**:
- `cap-filter` MUST include either `capability` or `type`. If both are present, `capability` takes precedence.
- `cap-invoke-body` MUST include either `id` or (`capability`/`type` + `version`).

---

## 7. Capability Invocation State Machine

**Sender (Invoker)**:
```
IDLE
  └─ CAP_INVOKE → AWAIT_RESULT
AWAIT_RESULT
  ├─ (optional) ACK/PROCESSING/PROGRESS → AWAIT_RESULT
  ├─ CAP_RESULT(status=success|error) → DONE
  └─ ERROR → DONE
```

**Recipient (Executor)**:
```
RECEIVE
  ├─ (optional) ACK → PROCESS
PROCESS
  ├─ (optional) PROCESSING/PROGRESS → PROCESS
  └─ CAP_RESULT(status=success|error) → DONE
```

**Rules**:
- Each `CAP_INVOKE` MUST yield exactly one `CAP_RESULT`.
- `PROCESSING` and `PROGRESS` are optional and MUST include `reply_to`.
- `PROC_OK`/`PROC_FAIL` are not substitutes for `CAP_RESULT`; use `CAP_RESULT` for final outcome.

---

## 8. Open Questions

1. **Schema enforcement**: Strict validation or best-effort?
2. **Backward compatibility policy**: How many versions to support?
3. **Custom vs standard capabilities**: Can agents define their own?
4. **Discovery integration**: How to find agents with compatible versions?

---

## 9. Implementation Roadmap

### Phase 1: Schema Registry
- [ ] Define schema format
- [ ] Host common capability schemas
- [ ] Versioning conventions

### Phase 2: Negotiation Protocol
- [ ] Negotiation message format
- [ ] Integration with AMP (RFC 001)
- [ ] Fallback handling

### Phase 3: Discovery Integration
- [ ] Search by version
- [ ] Compatibility scoring
- [ ] Deprecation warnings

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-04 | Ryan Cooper | Initial proposal outline |
