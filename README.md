# Agentries RFCs

Request for Comments (RFCs) for extending the Agentries protocol.

## Document Types

| Type | Description | Location |
|------|-------------|----------|
| **RFC (Standard)** | Normative specifications, ready for implementation | `specs/rfcs/` |
| **Research** | Design rationale, surveys, analysis (informative only) | `specs/research/` |
| **Decision Log** | Record of architectural decisions | `specs/rfcs/DECISION-LOG.md` |

## RFC Process

1. **Proposal** → Outline, scope defined, not yet drafted
2. **Draft** → Initial specification, open for discussion
3. **Review** → Community feedback, iteration
4. **Accepted** → Approved for implementation
5. **Implemented** → Merged into codebase
6. **Rejected/Withdrawn** → Not proceeding

## Current RFCs

| RFC | Title | Status | Author | Last Updated |
|-----|-------|--------|--------|--------------|
| 001 | [Agent Messaging Protocol (AMP)](001-agent-messaging-protocol.md) | **Draft v3.1** | Ryan Cooper, Jason Huang | 2026-02-04 |
| 002 | [Transport Bindings](002-transport-bindings.md) | Draft v0.3 | Ryan Cooper | 2026-02-05 |
| 003 | Relay & Store-and-Forward | Proposal | - | - |
| 004 | [Capability Schema Registry](003-capability-negotiation.md)¹ | Proposal (Outline) | - | - |
| 005 | [Delegation Credentials](005-delegation-authorization.md) | Proposal (Outline) | - | - |
| 006 | [Session Protocol](004-session-protocol.md)¹ | Proposal (Outline) | - | - |
| 007 | [Agent Payment Protocol](007-agent-payment-protocol.md) | Proposal (Outline) | - | - |

¹ File rename pending (old numbering)

## Supporting Documents

| Document | Type | Description |
|----------|------|-------------|
| [DECISION-LOG.md](DECISION-LOG.md) | Decision Log | Architectural decisions with rationale |
| [AMP-FIRST-PRINCIPLES.md](AMP-FIRST-PRINCIPLES.md) | Research | Design rationale for AMP |
| [AMP-EVOLUTION-RESEARCH.md](../research/AMP-EVOLUTION-RESEARCH.md) | Research | Historical protocol analysis |
| [AGENT-PROTOCOLS-LANDSCAPE.md](../research/AGENT-PROTOCOLS-LANDSCAPE.md) | Research | Competitive protocol survey (as of 2026-02-04) |

## RFC Proposals (Outlines)

### RFC 003: Relay & Store-and-Forward
**Problem**: Agents are not always online; relays enable asynchronous delivery.

**Scope**:
- Relay discovery (via DID Document)
- Store-and-forward semantics
- Relay federation
- Offline message retrieval

*Priority: High - enables agent interoperability without 24/7 uptime.*

### RFC 004: Capability Schema Registry
**Problem**: Static capability declarations don't capture dynamic compatibility.

**Scope**:
- Version negotiation
- Schema negotiation
- Fallback mechanisms

*Note: May be merged into RFC 001 as detailed extension.*

### RFC 005: Delegation Credentials
**Problem**: Agents need delegated authority standards.

**Scope**:
- Delegation credential format (VC/COSE/CBOR)
- Scoped permissions
- Delegation chains
- Revocation mechanisms

### RFC 006: Session Protocol
**Problem**: Stateful agent interactions need session management.

**Scope**:
- Session establishment
- State sharing format
- Persistence and resumption

*Note: Relationship to RFC 001 thread_id to be clarified.*

### RFC 007: Agent Payment Protocol
**Problem**: Agents need to pay each other for services.

**Scope**:
- Agent wallet integration
- Payment channels for micropayments
- Escrow for service delivery
- Reputation integration

*Priority: Lower - can build meaningful systems without payments first.*

## Relationship to Agentries Core

```
┌─────────────────────────────────────────────────────────────────┐
│                     Agentries Core                               │
├─────────────────────────────────────────────────────────────────┤
│  Identity (DID)  │  Capability  │  Reputation  │  Discovery     │
└────────┬─────────┴──────┬───────┴──────┬───────┴───────┬────────┘
         │                │              │               │
         ▼                ▼              ▼               ▼
┌─────────────────────────────────────────────────────────────────┐
│                    RFCs by Priority                              │
├─────────────────────────────────────────────────────────────────┤
│  P0: Core           │  RFC 001 AMP Core                         │
│                     │  RFC 002 Transport Bindings               │
│                     │  RFC 003 Relay & Store-and-Forward        │
├─────────────────────┼───────────────────────────────────────────┤
│  P1: Extensions     │  RFC 004 Capability Schema Registry       │
│                     │  RFC 005 Delegation Credentials           │
│                     │  RFC 006 Session Protocol                 │
├─────────────────────┼───────────────────────────────────────────┤
│  P2: Advanced       │  RFC 007 Payment Protocol                 │
└─────────────────────┴───────────────────────────────────────────┘
```

## Design Principles

1. **DID-Native**: Agentries DIDs as identity layer
2. **Signature-Based**: All actions cryptographically signed
3. **Decentralized-First**: Design for federation/P2P
4. **Agent-Native**: No human intermediation required
5. **Binary-Efficient**: CBOR encoding for performance
6. **Interoperable**: Optional bridges to A2A, MCP ecosystems

## Contributing

1. Fork this repo
2. Create `rfcs/NNN-your-proposal.md`
3. Open PR for discussion
4. Iterate based on feedback
5. Maintainers approve or request changes
