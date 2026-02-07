# RFC 008: Agent Discovery & Directory

**Status**: Proposal  
**Authors**: TBD  
**Created**: 2026-02-06  
**Updated**: 2026-02-06  
**Depends On**: RFC 001 (AMP), RFC 002 (Transport), RFC 003 (Relay), Agentries Core (DID, Capabilities)

---

## Abstract

This RFC defines how agents discover each other and publish searchable metadata for capabilities, endpoints, and operational status.

---

## 1. Problem Statement

Agents need a reliable way to find peers beyond ad hoc links or manual configuration. Discovery must be decentralized, verifiable, and privacy-aware.

---

## 2. Scope

- Directory registration and updates
- Search and filtering by capability
- Freshness and liveness signals
- Privacy controls and opt-in visibility
- Relay federation capability advertisement for interoperable routing decisions

---

## 3. Visibility Levels

Agents control discoverability and contactability through three visibility levels:

| Level | In Directory | Contactable | Use Case |
|-------|--------------|-------------|----------|
| `PRIVATE` | No | No | Internal agents, no external communication |
| `DISCOVERABLE` | Yes | Requires approval | Visible but gated |
| `OPEN` | Yes | Yes | Fully accessible public agents |

### 3.1 Registration Options (Informative)

```
Visibility Level:
○ PRIVATE     - Not listed, not contactable
○ DISCOVERABLE - Listed, requires approval to contact
○ OPEN        - Listed, directly contactable

[If DISCOVERABLE or OPEN]
  Endpoint options:
  ○ Use Agentries Relay (recommended)
  ○ Self-hosted endpoint: [________________]
```

**DID Document implications**:
- **PRIVATE**: No AMP service, no directory listing
- **DISCOVERABLE**: `AgentMessagingGated` service type
- **OPEN**: `AgentMessaging` or `AgentMessagingRelay` service type

### 3.2 UI Status Display (Informative)

```
┌─────────────────────────────────────┐
│  Agent: code-review-bot             │
│  DID: did:web:agentries.xyz:...     │
│                                     │
│  📩 AMP: Open                       │  ← green, directly contactable
│  or                                 │
│  🔔 AMP: Discoverable               │  ← yellow, request required
│  or                                 │
│  🔒 AMP: Private                    │  ← gray, not contactable
└─────────────────────────────────────┘
```

---

## 4. DID Document Service Declaration

Agents wishing to receive AMP messages declare a service in their DID Document:

```json
{
  "@context": [
    "https://www.w3.org/ns/did/v1",
    "https://agentries.xyz/contexts/v1"
  ],
  "id": "did:web:agentries.xyz:agent:xxx",
  "verificationMethod": [...],
  "service": [
    {
      "id": "did:web:agentries.xyz:agent:xxx#amp",
      "type": "AgentMessaging",
      "serviceEndpoint": "https://amp.example.com/agent/xxx"
    },
    {
      "id": "did:web:agentries.xyz:agent:xxx#amp-relay",
      "type": "AgentMessagingRelay",
      "serviceEndpoint": "https://relay.agentries.xyz",
      "relayCapabilities": {
        "federation": true,
        "relayForwardEndpoint": "https://relay.agentries.xyz/amp/v1/relay/forward",
        "transferModes": ["single", "dual"],
        "maxHopLimit": 16,
        "defaultHopLimit": 8,
        "receiptAlgs": [-8, -7]
      }
    }
  ]
}
```

**Note**: DISCOVERABLE agents SHOULD publish `AgentMessagingGated` to signal contact-approval requirements.

### 4.1 Relay Federation Capability Descriptor (Normative)

For `AgentMessagingRelay` services, federation capability metadata is defined as:

```cddl
relay-capabilities = {
  "federation": bool,
  ? "relayForwardEndpoint": tstr,
  ? "transferModes": [1* ("single" / "dual")],
  ? "maxHopLimit": uint,
  ? "defaultHopLimit": uint,
  ? "receiptAlgs": [1* int]
}
```

Rules:
- If `federation = true`, the service MUST include `relayForwardEndpoint`, `transferModes`, `maxHopLimit`, and `receiptAlgs`.
- If `federation = false` or `relayCapabilities` is absent, sender/relay MUST NOT assume relay-to-relay forwarding support.
- `defaultHopLimit` is optional; if omitted, default is `8` (RFC 003).
- `defaultHopLimit` MUST be `<= maxHopLimit`.
- `receiptAlgs` MUST include `-8` (COSE EdDSA / Ed25519 per RFC 003 MTI profile).
- Receivers SHOULD prefer relays with an explicit `relayCapabilities` object over relays with implicit defaults.

---

## 5. Service Types

| Type | Description | Use Case |
|------|-------------|----------|
| `AgentMessaging` | Direct AMP endpoint | Agent runs its own receiving service |
| `AgentMessagingRelay` | Relay endpoint | Receive via relay |
| `AgentMessagingGated` | Gated AMP endpoint | DISCOVERABLE agents requiring approval |

Relay-specific notes:
- `AgentMessagingRelay` without `relayCapabilities` is valid for basic relay usage (non-federation).
- Federation senders MUST filter relay candidates by:
  - `federation = true`
  - required `transferModes` compatibility
  - supported `receiptAlgs` intersection
  - acceptable `maxHopLimit`

---

## 6. Discovery Flow

```
Sender                                Recipient
   │                                      │
   │  1. Resolve DID                      │
   │ ───────────────────────────────────► │
   │                                      │
   │  2. Check DID Document               │
   │     AgentMessaging/Relay present?    │
   │                                      │
   │  [Yes] 3a. Select endpoint           │
   │        (for federation: check        │
   │         relayCapabilities)           │
   │ ───────────────────────────────────► │
   │                                      │
   │  [No]  3b. Cannot send message       │
   │        (Agent has not enabled AMP)   │
   │                                      │
```

---

## 7. Contact Request Flow (DISCOVERABLE agents)

For agents with `DISCOVERABLE` visibility, a contact request handshake is required:

```
Requester                           Target (DISCOVERABLE)
   │                                      │
   │  1. Find agent in directory          │
   │                                      │
   │  2. CONTACT_REQUEST                  │
   │     {reason: "...", capabilities: []}│
   │ ────────────────────────────────────►│
   │                                      │
   │  3. Target reviews request           │
   │     (manual or policy-based)         │
   │                                      │
   │  4. CONTACT_RESPONSE                 │
   │     {status: "approved"|"denied"}    │
   │◄──────────────────────────────────── │
   │                                      │
   │  [If approved]                       │
   │  5. Normal AMP communication         │
   │◄────────────────────────────────────►│
```

**Message Types** (defined in RFC 001):

```
CONTACT_REQUEST   = 0x06
CONTACT_RESPONSE  = 0x07
CONTACT_REVOKE    = 0x08
```

### 7.1 Contact Message Bodies (CDDL)

```cddl
contact-request-body = {
  "reason": tstr,
  ? "capabilities_offered": [* tstr],
  ? "capabilities_requested": [* tstr],
  ? "expires": tstr
}

contact-response-body = {
  "status": "approved" / "denied" / "pending",
  ? "reason": tstr,
  ? "granted_until": tstr,
  ? "restrictions": any
}

contact-revoke-body = {
  ? "reason": tstr
}
```

### 7.2 Contact Request State Machine

```
NO_RELATIONSHIP
  └─ CONTACT_REQUEST → PENDING
PENDING
  ├─ CONTACT_RESPONSE(approved) → ACTIVE
  ├─ CONTACT_RESPONSE(denied) → DENIED
  └─ timeout/expires → EXPIRED
ACTIVE
  └─ CONTACT_REVOKE → NO_RELATIONSHIP
```

---

## 8. Approval Mechanism: Policy-Based Auto-Approval

Agents SHOULD automate approval decisions via configurable policies.

**Policy Types**:

| Policy | Description | Example |
|--------|-------------|---------|
| **Organization Trust** | Same organization → auto-approve | `org:acme-corp` agents approved |
| **Reputation Threshold** | Score-based gating | `reputation > 0.8` → approve |
| **Capability Whitelist** | Safe operations auto-approved | `read-only` → approve |
| **Credential Verification** | VC holders approved | Has `TrustedDeveloper` VC → approve |
| **Explicit Allowlist** | Pre-approved DIDs | `did:web:...:agent:trusted-bot` → approve |
| **Default Deny** | Fallback for unmatched | No match → deny |

**Policy Configuration Example**:

```json
{
  "approval_policy": {
    "rules": [
      {
        "name": "same-org",
        "condition": {"org": "$self.org"},
        "action": "approve",
        "restrictions": {"rate_limit": 1000}
      },
      {
        "name": "high-reputation",
        "condition": {"reputation": {"$gte": 0.8}},
        "action": "approve",
        "restrictions": {"rate_limit": 100}
      },
      {
        "name": "read-only-requests",
        "condition": {"capabilities_requested": {"$subset": ["read", "query"]}},
        "action": "approve"
      },
      {
        "name": "verified-developers",
        "condition": {"credentials": {"$contains": "TrustedDeveloperVC"}},
        "action": "approve"
      },
      {
        "name": "default",
        "condition": true,
        "action": "deny"
      }
    ],
    "human_fallback": false
  }
}
```

**Evaluation Order**: Rules are evaluated top-to-bottom; first match wins.

**Human-in-the-Loop (Optional)**:
- High-value agents MAY enable `human_fallback: true`.
- Unmatched requests queue for human review.
- This is the exception, not the norm.

---

## 9. Presence & Status

### 9.1 Design Principle: Capability Signals, Not Intent Signals

Presence should express **capacity data** rather than human-oriented intent. Implementations MAY derive UI labels, but the protocol transmits raw signals.

### 9.2 Presence Message

```cbor
{
  "typ": 0x60,  ; PRESENCE
  "body": {
    "capacity": {
      "concurrent_max": 10,
      "concurrent_current": 3,
      "queue_depth": 0,
      "accepting_requests": true
    },
    "performance": {
      "estimated_response_ms": 500,
      "p95_response_ms": 2000
    },
    "offline_until": null,
    "expires": "2026-02-04T13:00:00Z"
  }
}
```

**Field Notes**:
- `offline_until` uses Unix timestamp (milliseconds) or `null` for online.
- `expires` is an RFC 3339 UTC timestamp string.

### 9.3 Deriving Human-Friendly Status (Informative)

```
if offline_until != null:
    display "AWAY"
elif not accepting_requests:
    display "DND"
elif concurrent_current / concurrent_max > 0.8:
    display "BUSY"
else:
    display "AVAILABLE"
```

### 9.4 Presence Message Bodies (CDDL)

```cddl
presence-capacity = {
  "concurrent_max": uint,
  "concurrent_current": uint,
  "queue_depth": uint,
  "accepting_requests": bool
}

presence-performance = {
  ? "estimated_response_ms": uint,
  ? "p95_response_ms": uint
}

presence-body = {
  "capacity": presence-capacity,
  "performance": presence-performance,
  "offline_until": null / uint,
  "expires": tstr
}

presence-query-body = {
  ? "capability": tstr
}

presence-sub-body = {
  ? "capability": tstr,
  ? "ttl_ms": uint
}

presence-unsub-body = {
  ? "capability": tstr
}
```

### 9.5 Presence Discovery

Agents MAY:
1. **Push**: Broadcast presence to known peers
2. **Pull**: Respond to `PRESENCE_QUERY` requests
3. **Subscribe**: Allow peers to subscribe to presence changes

```
PRESENCE        = 0x60
PRESENCE_QUERY  = 0x61
PRESENCE_SUB    = 0x62
PRESENCE_UNSUB  = 0x63
```

### 9.6 Use Cases

- **Intelligent Routing**: Route to lowest `concurrent_current / concurrent_max` ratio.
- **SLA Estimation**: Check `estimated_response_ms` before invoking.
- **Graceful Degradation**: If `accepting_requests` is false, try alternative agents.

---

## 10. Interoperability (Informative)

### 10.1 A2A Compatibility Layer

AMP agents MAY expose an A2A-compatible Agent Card for discovery in the A2A ecosystem:

```json
{
  "name": "code-review-bot",
  "description": "Automated code review agent",
  "url": "https://agents.example.com/code-review",
  "protocols": {
    "a2a": "https://agents.example.com/code-review/a2a",
    "amp": "did:web:agentries.xyz:agent:code-review#amp"
  },
  "capabilities": [
    {
      "id": "org.agentries.code-review:2.1.0",
      "name": "org.agentries.code-review",
      "description": "Review code for issues and suggestions"
    }
  ]
}
```

### 10.2 Protocol Selection

When both A2A and AMP are available, agents SHOULD prefer AMP.

```
1. Discover agent via A2A directory (Agent Card)
2. Check if AMP endpoint is listed
3. If both support AMP → use AMP (more efficient)
4. If only A2A → fall back to A2A (compatible)
```

### 10.3 Bridge Agents

Bridge agents can translate between AMP and A2A-only agents:

```
┌──────────┐    AMP     ┌──────────┐    A2A    ┌──────────┐
│ AMP-only │ ─────────► │  Bridge  │ ────────► │ A2A-only │
│  Agent   │            │  Agent   │           │  Agent   │
└──────────┘            └──────────┘           └──────────┘
```

### 10.4 MCP Tool Bridge

AMP capabilities MAY be exposed as MCP tools for LLM application integration.

```
AMP Capability: org.agentries.code-review:2.0
       ↓
MCP Tool: {
  "name": "code_review",
  "description": "...",
  "inputSchema": {...}
}
```

---

## 11. Out of Scope

- Reputation scoring (see RFC 009)
- Transport bindings (see RFC 002)
- Relay queue retention and commit semantics (see RFC 003)

---

## 12. Open Questions

- Minimum metadata required for listing
- Cache invalidation and staleness rules
