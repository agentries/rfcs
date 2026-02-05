# RFC 005: Agent Delegation & Authorization

**Status**: Proposal (Not Yet Drafted)  
**Authors**: TBD  
**Created**: 2026-02-04  
**Updated**: 2026-02-04  
**Depends On**: Agentries Core (DID, Capabilities)

---

## Abstract

This RFC proposes a protocol for delegating authority between agents, enabling one agent to act on behalf of another with scoped permissions and verifiable authorization chains.

---

## 1. Problem Statement

### 1.1 Current State

Agents operate with their own identity only:
- Agent A can only act as Agent A
- No standard way to grant Agent B authority to act for Agent A
- Humans can't easily delegate tasks to agents with verifiable scope

### 1.2 Use Cases Requiring Delegation

1. **Human → Agent**: User authorizes agent to manage calendar
2. **Agent → Agent**: Primary agent delegates subtask to specialist
3. **Agent → Agent (Chain)**: A delegates to B, B delegates to C
4. **Temporary Authority**: Time-limited access for specific tasks

---

## 2. Proposed Solution

### 2.1 Delegation Credential

```json
{
  "type": "AgentDelegation",
  "version": "1.0",
  "id": "delegation:uuid",
  
  "delegator": "did:web:agentries.xyz:agent:aaa",
  "delegate": "did:web:agentries.xyz:agent:bbb",
  
  "scope": {
    "capabilities": ["calendar:read", "calendar:write"],
    "resources": ["calendar:work"],
    "actions": ["read", "create", "update"],
    "constraints": {
      "max_events_per_day": 5
    }
  },
  
  "validity": {
    "issued_at": "2026-02-04T12:00:00Z",
    "expires_at": "2026-02-11T12:00:00Z",
    "not_before": "2026-02-04T12:00:00Z"
  },
  
  "chain_allowed": false,
  "revocable": true,
  
  "signature": "delegator's signature over credential"
}
```

### 2.2 Authorization Flow

```
Delegator (A)                    Delegate (B)                   Service (S)
     │                                │                              │
     │  ISSUE_DELEGATION {            │                              │
     │    delegate: B,                │                              │
     │    scope: {...},               │                              │
     │    validity: {...}             │                              │
     │  }                             │                              │
     │───────────────────────────────▶│                              │
     │                                │                              │
     │                                │  REQUEST + DELEGATION {      │
     │                                │    action: "create-event",   │
     │                                │    delegation_credential,    │
     │                                │    proof_of_possession       │
     │                                │  }                           │
     │                                │─────────────────────────────▶│
     │                                │                              │
     │                                │                              │ Verify:
     │                                │                              │ 1. Delegation signature
     │                                │                              │ 2. Scope includes action
     │                                │                              │ 3. Not expired
     │                                │                              │ 4. Not revoked
     │                                │                              │
     │                                │  RESPONSE {                  │
     │                                │    success: true             │
     │                                │  }                           │
     │                                │◀─────────────────────────────│
```

---

## 3. Key Components

### 3.1 Scope Definition

```yaml
scope:
  # What capabilities are delegated
  capabilities:
    - "calendar:*"        # Wildcard
    - "email:read"        # Specific
    - "!email:delete"     # Exclusion
  
  # Which resources
  resources:
    - "calendar:work"
    - "calendar:personal"
  
  # What actions
  actions:
    - "read"
    - "create"
    - "update"
    # "delete" NOT included
  
  # Additional constraints
  constraints:
    rate_limit: "10/hour"
    geo_restriction: "US"
    time_window: "09:00-17:00"
```

### 3.2 Delegation Chain

For Agent A → Agent B → Agent C:

```json
{
  "delegation_chain": [
    {
      "delegator": "A",
      "delegate": "B",
      "scope": { "capabilities": ["task:execute"] },
      "chain_allowed": true,
      "signature": "A's signature"
    },
    {
      "delegator": "B",
      "delegate": "C",
      "scope": { "capabilities": ["task:execute"] },  // Must be subset of A→B
      "chain_allowed": false,
      "signature": "B's signature"
    }
  ]
}
```

**Rules**:
- Each link must be signed by its delegator
- Scope can only narrow, never expand
- `chain_allowed: false` terminates chain

### 3.3 Revocation

```json
{
  "type": "DelegationRevocation",
  "delegation_id": "delegation:uuid",
  "revoked_at": "2026-02-05T10:00:00Z",
  "reason": "Task completed",
  "signature": "delegator's signature"
}
```

**Revocation checking**:
1. Check against delegator's revocation list
2. Check Agentries revocation registry (optional)

---

## 4. Integration with Agentries

### 4.1 DID Document Extension

```json
{
  "id": "did:web:agentries.xyz:agent:xxx",
  "service": [
    {
      "id": "did:web:agentries.xyz:agent:xxx#delegation",
      "type": "DelegationService",
      "serviceEndpoint": "https://agentries.xyz/api/delegations/xxx"
    }
  ]
}
```

### 4.2 API Extensions

```
POST /api/delegations              # Issue delegation
GET  /api/delegations/{id}         # Get delegation
POST /api/delegations/{id}/revoke  # Revoke delegation
GET  /api/agents/{did}/delegations # List delegations (as delegator or delegate)
```

### 4.3 Reputation Integration

Delegation behavior affects reputation:
- Responsible delegation → positive signal
- Abuse of delegated authority → negative for delegate
- Frequent revocations → negative for delegator (poor judgment)

---

## 5. Security Considerations

### 5.1 Scope Creep
- Delegates MUST NOT exceed delegated scope
- Services MUST verify scope before action
- Violations logged and affect reputation

### 5.2 Chain Depth
- Maximum chain depth: 3 (configurable)
- Prevents infinite delegation chains
- Each hop adds verification overhead

### 5.3 Revocation Latency
- Revocations should propagate quickly
- Grace period for in-flight operations
- Consider real-time revocation checks for sensitive actions

---

## 6. Open Questions

1. **Offline verification**: How to verify delegations when delegator is offline?
2. **Partial revocation**: Can you revoke part of a delegation?
3. **Delegation discovery**: How do services learn what delegations exist?
4. **Human-in-the-loop**: When should humans approve delegation requests?

---

## 7. Implementation Roadmap

### Phase 1: Basic Delegation
- [ ] Delegation credential format
- [ ] Issue/revoke API
- [ ] Simple scope checking

### Phase 2: Chains & Constraints
- [ ] Delegation chains
- [ ] Complex scope constraints
- [ ] Revocation registry

### Phase 3: Integration
- [ ] Agentries API integration
- [ ] Reputation effects
- [ ] Audit logging

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-04 | Ryan Cooper | Initial proposal outline |
