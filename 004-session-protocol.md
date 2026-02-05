# RFC 004: Agent Session Protocol

**Status**: Proposal (Not Yet Drafted)  
**Authors**: TBD  
**Created**: 2026-02-04  
**Updated**: 2026-02-04  
**Depends On**: RFC 001 (AMP), RFC 003 (Capability Negotiation)

---

## Abstract

This RFC proposes a protocol for establishing, maintaining, and terminating stateful sessions between agents, enabling multi-turn interactions and complex workflows.

---

## 1. Problem Statement

### 1.1 Current State

Agent interactions are typically stateless:
- Single request → single response
- No context carried between calls
- Complex workflows require external orchestration

### 1.2 Limitations

- **Multi-turn conversations**: Agents can't maintain dialogue context
- **Long-running tasks**: No way to track progress across multiple exchanges
- **Workflow state**: Complex agent collaborations require external state management
- **Resumption**: If connection breaks, context is lost

---

## 2. Proposed Solution

### 2.1 Session Lifecycle

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   INIT      │────▶│   ACTIVE    │────▶│  CLOSING    │────▶│   CLOSED    │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
                           │                                       ▲
                           │          ┌─────────────┐              │
                           └─────────▶│  SUSPENDED  │──────────────┘
                                      └─────────────┘
```

### 2.2 Session Establishment

```
Agent A                                    Agent B
   │                                          │
   │  SESSION_INIT {                          │
   │    session_id: "uuid",                   │
   │    purpose: "code-review-workflow",      │
   │    ttl: 3600,                            │
   │    capabilities_required: [...]          │
   │  }                                       │
   │─────────────────────────────────────────▶│
   │                                          │
   │  SESSION_ACCEPT {                        │
   │    session_id: "uuid",                   │
   │    expires_at: "...",                    │
   │    state_endpoint: "..."                 │
   │  }                                       │
   │◀─────────────────────────────────────────│
   │                                          │
```

### 2.3 Session Context

Each session maintains:
- **Session ID**: Unique identifier
- **Participants**: DIDs of involved agents
- **State**: Shared context data
- **History**: Message sequence
- **TTL**: Expiration time

---

## 3. Key Components

### 3.1 Session State Format

```json
{
  "session_id": "uuid",
  "created_at": "2026-02-04T12:00:00Z",
  "expires_at": "2026-02-04T13:00:00Z",
  "status": "active",
  "participants": [
    "did:web:agentries.xyz:agent:aaa",
    "did:web:agentries.xyz:agent:bbb"
  ],
  "context": {
    "purpose": "code-review-workflow",
    "current_step": 2,
    "total_steps": 5,
    "shared_data": { ... }
  },
  "history": [
    { "seq": 1, "from": "did:...", "type": "request", ... },
    { "seq": 2, "from": "did:...", "type": "response", ... }
  ]
}
```

### 3.2 Session Messages

| Message Type | Description |
|-------------|-------------|
| `SESSION_INIT` | Initiate new session |
| `SESSION_ACCEPT` | Accept session invitation |
| `SESSION_REJECT` | Decline session |
| `SESSION_MSG` | Message within session |
| `SESSION_STATE` | State update |
| `SESSION_SUSPEND` | Pause session |
| `SESSION_RESUME` | Resume suspended session |
| `SESSION_CLOSE` | Terminate session |

### 3.3 State Persistence

Options for storing session state:
1. **Initiator-hosted**: Agent A maintains state
2. **Relay-hosted**: Message relay stores state
3. **Distributed**: State replicated across participants

---

## 4. Use Cases

### 4.1 Multi-Turn Conversation

```
Session: "tech-support-123"
├── Turn 1: User describes problem
├── Turn 2: Agent asks clarifying question
├── Turn 3: User provides details
├── Turn 4: Agent proposes solution
└── Turn 5: User confirms, session closes
```

### 4.2 Complex Workflow

```
Session: "code-review-workflow-456"
├── Step 1: Agent A sends code
├── Step 2: Agent B performs security scan
├── Step 3: Agent C performs style check
├── Step 4: Agent B reports security findings
├── Step 5: Agent C reports style findings
└── Step 6: Agent A aggregates and responds
```

---

## 5. Open Questions

1. **State storage**: Who is responsible for persisting session state?
2. **Concurrent sessions**: How many simultaneous sessions can an agent handle?
3. **Session recovery**: How to handle partial failures mid-session?
4. **Privacy**: Should session history be encrypted?

---

## 6. Implementation Roadmap

### Phase 1: Basic Sessions
- [ ] Session init/accept/close
- [ ] Simple state storage
- [ ] Session timeout handling

### Phase 2: Advanced Features
- [ ] Suspend/resume
- [ ] Multi-party sessions
- [ ] State checkpointing

### Phase 3: Integration
- [ ] AMP integration
- [ ] Agentries session registry
- [ ] Session analytics

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-04 | Ryan Cooper | Initial proposal outline |
