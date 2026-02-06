# RFC 004: Capability Schema Registry & Compatibility

**Status**: Proposal (Not Yet Drafted)  
**Authors**: TBD  
**Created**: 2026-02-04  
**Updated**: 2026-02-06  

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

## 4. Open Questions

1. **Schema enforcement**: Strict validation or best-effort?
2. **Backward compatibility policy**: How many versions to support?
3. **Custom vs standard capabilities**: Can agents define their own?
4. **Discovery integration**: How to find agents with compatible versions?

---

## 5. Implementation Roadmap

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
