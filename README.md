# Agentries RFCs

Request for Comments (RFCs) for extending the Agentries protocol.

## Document Types

| Type | Description | Location |
|------|-------------|----------|
| **RFC (Standard)** | Normative specifications, ready for implementation | Repository root (`./`) |
| **Research** | Design rationale and analysis (informative only) | Repository root (`./`) |
| **Decision Log** | Record of architectural decisions | `DECISION-LOG.md` |

## RFC Process

1. **Proposal** → Outline, scope defined, not yet drafted
2. **Draft** → Initial specification, open for discussion
3. **Review** → Community feedback, iteration
4. **Accepted** → Approved for implementation
5. **Implemented** → Merged into codebase
6. **Rejected/Withdrawn** → Not proceeding

Note: **Accepted** indicates implementation-ready specifications (byte-accurate and testable).

## Current RFCs

| RFC | Title | Status | Author | Last Updated |
|-----|-------|--------|--------|--------------|
| 001 | [Agent Messaging Protocol (AMP Core)](001-agent-messaging-protocol.md) | Draft v0.42 | Ryan Cooper, Jason Apple Huang | 2026-02-07 |
| 002 | [Transport Bindings (TCP-first, HTTP/WS mappings)](002-transport-bindings.md) | Draft v0.15 | Ryan Cooper, Nowa | 2026-02-07 |
| 003 | [Relay & Store-and-Forward](003-relay-store-and-forward.md) | Draft v0.61 | Nowa | 2026-02-07 |
| 004 | [Capability Schema Registry & Compatibility](004-capability-schema-registry.md) | Draft v0.12 | Ryan Cooper, Nowa | 2026-02-07 |
| 005 | [Delegation Credentials & Authorization](005-delegation-authorization.md) | Draft v0.34 | Ryan Cooper, Nowa | 2026-02-07 |
| 006 | [Session Protocol (State + Recovery)](006-session-protocol.md) | Draft v0.8 (coupled MTI + optional independent thread profile + explicit session_scope marker) | Ryan Cooper, Nowa | 2026-02-07 |
| 007 | [Agent Payment Protocol](007-agent-payment-protocol.md) | Draft v0.34 (CAP precedence + session source-of-truth + split descriptor failure vectors + byte checks) | Ryan Cooper, Nowa | 2026-02-07 |
| 008 | [Agent Discovery & Directory](008-agent-discovery-directory.md) | Proposal (relay federation capability descriptor added) | - | 2026-02-06 |
| 009 | [Reputation & Trust Signals](009-reputation-trust-signals.md) | Planned (Future) | - | 2026-02-06 |
| 010 | [Observability & Evaluation Telemetry](010-observability-evaluation-telemetry.md) | Planned (Future) | - | 2026-02-06 |
| 011 | [Multi-Agent Coordination & Group Messaging](011-multi-agent-coordination.md) | Planned (Future) | - | 2026-02-06 |

## Supporting Documents

| Document | Type | Description |
|----------|------|-------------|
| [DECISION-LOG.md](DECISION-LOG.md) | Decision Log | Architectural decisions with rationale |
| [AMP-FIRST-PRINCIPLES.md](AMP-FIRST-PRINCIPLES.md) | Research | Design rationale for AMP |
| [conformance/2026-02-07-draft-v1/README.md](conformance/2026-02-07-draft-v1/README.md) | Conformance | Draft interoperability suite manifest + report schema/template |
| [examples/rust-amp001/README.md](examples/rust-amp001/README.md) | Example | RFC 001 end-to-end demo (server/client) |
| [examples/rust-amp002-004/README.md](examples/rust-amp002-004/README.md) | Example | RFC 002-004 transport and capability interop demo/tests |
| [examples/rust-amp005/README.md](examples/rust-amp005/README.md) | Example | RFC 003 relay/store-and-forward E2E demo/tests |

## RFC Proposals (Outlines)

### RFC 003: Relay & Store-and-Forward
**Problem**: Agents are not always online; relays enable asynchronous delivery.

**Scope**:
- Relay discovery (via DID Document)
- Store-and-forward semantics
- Relay federation
- Offline message retrieval

*Priority: High - enables agent interoperability without 24/7 uptime.*

### RFC 004: Capability Schema Registry & Compatibility
**Problem**: Static capability declarations don't capture dynamic compatibility.

**Scope**:
- Version negotiation
- Schema negotiation
- Fallback mechanisms

*Note: May be merged into RFC 001 as detailed extension.*

### RFC 005: Delegation Credentials & Authorization
**Problem**: Agents need delegated authority standards.

**Scope**:
- Delegation credential format (VC/COSE/CBOR)
- Scoped permissions
- Delegation chains
- Revocation mechanisms

### RFC 006: Session Protocol (State + Recovery)
**Problem**: Stateful agent interactions need session management.

**Scope**:
- Session establishment
- State sharing format
- Persistence and resumption

*Note: Draft v0.8 keeps `coupled` (`thread_id==session_id`) as MTI baseline, adds optional `independent` thread mode, and requires explicit `session_scope=true` marker for unambiguous session-scoped non-control dispatch.*

### RFC 007: Agent Payment Protocol
**Problem**: Agents need to pay each other for services.

**Scope**:
- Quote/authorize/capture/cancel/refund/status workflow semantics
- Settlement-proof abstraction and deterministic verification/mapping
- CAP interoperability profile (`org.agentries.payment.workflow:1.0.0`)
- Chain/rail-specific settlement internals remain out of scope

*Note: Draft v0.34 keeps RFC 004-aligned CAP behavior and further splits descriptor-integrity negatives into deterministic `3001`/`5002` vectors with minimal byte-level code checks.*

### RFC 008: Agent Discovery & Directory
**Problem**: Agents need a way to find peers and publish capabilities.

**Scope**:
- Directory registration and updates
- Search and filtering by capability
- Freshness and liveness signals
- Privacy controls and visibility

### RFC 009: Reputation & Trust Signals
**Problem**: Agents need trust signals for counterparties and services.

**Scope**:
- Reputation signal types and provenance
- Aggregation and decay models
- Verifiable attestations and disputes

### RFC 010: Observability & Evaluation Telemetry
**Problem**: Operators need consistent telemetry to measure reliability and quality.

**Scope**:
- Telemetry event taxonomy and schemas
- Privacy and data minimization
- Aggregation and correlation guidelines

### RFC 011: Multi-Agent Coordination & Group Messaging
**Problem**: Complex workflows require standardized coordination semantics.

**Scope**:
- Group addressing and membership
- Roles, handoffs, and conflict resolution
- Coordination metadata patterns

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
├─────────────────────┬───────────────────────────────────────────┤
│  P0: Core           │  RFC 001 AMP Core                         │
│                     │  RFC 002 Transport Bindings               │
│                     │  RFC 003 Relay & Store-and-Forward        │
├─────────────────────┼───────────────────────────────────────────┤
│  P1: Extensions     │  RFC 004 Capability Schema Registry       │
│                     │  RFC 005 Delegation Credentials           │
│                     │  RFC 006 Session Protocol                 │
├─────────────────────┼───────────────────────────────────────────┤
│  P2: Advanced       │  RFC 007 Payment Protocol                 │
├─────────────────────┼───────────────────────────────────────────┤
│  P3: Ecosystem      │  RFC 008 Discovery & Directory            │
│                     │  RFC 009 Reputation & Trust Signals       │
│                     │  RFC 010 Observability & Evaluation       │
│                     │  RFC 011 Multi-Agent Coordination         │
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
2. Create `NNN-your-proposal.md` using `RFC-TEMPLATE.md` as a starting point
3. Open PR for discussion
4. Iterate based on feedback
5. Maintainers approve or request changes
