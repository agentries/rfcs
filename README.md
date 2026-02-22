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

## Current Conformance Target

Current program target is `Draft` interoperability readiness.
Design-requirement evaluation for RFC `001-011` in this cycle is based on `Draft` gate completion.
Current-cycle design-requirement verdict for RFC `001-011`: `SATISFIED` (see `conformance/2026-02-07-draft-v1/GATE-STATUS.md`).
In this cycle, a failing `Accepted` gate check is expected and does not block RFC `001-011` design-requirement closure.
Current-cycle completion check command: `conformance/2026-02-07-draft-v1/validate-design-requirements.sh conformance/2026-02-07-draft-v1/reports`.
`Accepted/Implementation-Ready` is deferred until the conformance suite includes at least two independent `full-stack-pass` reports under `conformance/2026-02-07-draft-v1/reports`.

## Current RFCs

| RFC | Title | Status | Author | Last Updated |
|-----|-------|--------|--------|--------------|
| 001 | [Agent Messaging Protocol (AMP Full Entry + Core Subset)](001-agent-messaging-protocol.md) | Draft v0.46 | Ryan Cooper, Jason Apple Huang | 2026-02-22 |
| 002 | [Transport Bindings (TCP-first, HTTP/WS mappings)](002-transport-bindings.md) | Draft v0.16 | Ryan Cooper, Nowa | 2026-02-07 |
| 003 | [Relay & Store-and-Forward](003-relay-store-and-forward.md) | Draft v0.61 | Nowa | 2026-02-07 |
| 004 | [Capability Schema Registry & Compatibility](004-capability-schema-registry.md) | Draft v0.12 | Ryan Cooper, Nowa | 2026-02-07 |
| 005 | [Delegation Credentials & Authorization](005-delegation-authorization.md) | Draft v0.34 | Ryan Cooper, Nowa | 2026-02-07 |
| 006 | [Session Protocol (State + Recovery)](006-session-protocol.md) | Draft v0.9 (coupled MTI + optional independent thread profile + explicit session_scope marker) | Ryan Cooper, Nowa | 2026-02-07 |
| 007 | [Agent Payment Protocol](007-agent-payment-protocol.md) | Draft v0.34 (CAP precedence + session source-of-truth + split descriptor failure vectors + byte checks) | Ryan Cooper, Nowa | 2026-02-07 |
| 008 | [Agent Discovery & Directory](008-agent-discovery-directory.md) | Draft v0.4 | Nowa | 2026-02-10 |
| 009 | [Reputation & Trust Signals](009-reputation-trust-signals.md) | Draft v0.11 (session/CAP negative vectors + 3004 byte-level checks) | Nowa | 2026-02-10 |
| 010 | [Observability & Evaluation Telemetry](010-observability-evaluation-telemetry.md) | Draft v0.5 | Nowa | 2026-02-10 |
| 011 | [Multi-Agent Coordination & Group Messaging](011-multi-agent-coordination.md) | Draft v0.6 | Nowa | 2026-02-10 |
| 012 | [Task Protocol](012-task-protocol.md) | Draft v0.1 | Nowa | 2026-02-22 |
| 013 | [Negotiation Protocol](013-negotiation-protocol.md) | Draft v0.1 | Nowa | 2026-02-22 |
| 014 | [Handoff Protocol](014-handoff-protocol.md) | Draft v0.1 | Nowa | 2026-02-22 |

## Supporting Documents

| Document | Type | Description |
|----------|------|-------------|
| [DECISION-LOG.md](DECISION-LOG.md) | Decision Log | Architectural decisions with rationale |
| [AMP-FIRST-PRINCIPLES.md](AMP-FIRST-PRINCIPLES.md) | Research | Design rationale for AMP |
| [conformance/2026-02-07-draft-v1/README.md](conformance/2026-02-07-draft-v1/README.md) | Conformance | Draft interoperability suite manifest + report schema/template |
| [conformance/2026-02-07-draft-v1/GATE-STATUS.md](conformance/2026-02-07-draft-v1/GATE-STATUS.md) | Conformance | Accepted-gate readiness snapshot and remaining gaps |
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

*Note: Draft v0.9 keeps `coupled` (`thread_id==session_id`) as MTI baseline, adds optional `independent` thread mode, and requires explicit `session_scope=true` marker for unambiguous session-scoped non-control dispatch.*

### RFC 007: Agent Payment Protocol
**Problem**: Agents need to pay each other for services.

**Scope**:
- Quote/authorize/capture/cancel/refund/status workflow semantics
- Settlement-proof abstraction and deterministic verification/mapping
- CAP interoperability profile (`xyz.agentries.payment.workflow:1.0.0`)
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

### RFC 012: Task Protocol
**Problem**: Agents need persistent, queryable long-running task lifecycle management independent of sessions.

**Scope**:
- Task creation, acceptance, progress tracking, input collection, completion, failure, cancellation
- Sub-task delegation and coordination
- Persistent task state queryable across sessions
- CAP interoperability profile (`xyz.agentries.task.workflow:1.0.0`)
- Optional loose coupling with RFC 007 (payment) and RFC 013 (negotiation) via `payment_id`/`terms_id`

*Note: First AMP Standard Profile RFC. Uses `typ=0x80` with profile-body dispatch (`xyz.agentries.*` namespace). Error codes in 42xx range.*

### RFC 013: Negotiation Protocol
**Problem**: Agents need structured multi-round agreement before committing to tasks, payments, or other interactions.

**Scope**:
- Multi-round propose/counter/accept/reject/withdraw semantics
- Structured terms with expiry and round numbering
- Stable `negotiation_id` referenceable from other protocols (RFC 007, RFC 012)
- CAP interoperability profile (`xyz.agentries.negotiation.workflow:1.0.0`)

*Note: AMP Standard Profile. Uses `typ=0x81` with profile-body dispatch. Error codes in 43xx range.*

### RFC 014: Handoff Protocol
**Problem**: Agents need structured responsibility transfer with context packaging, client notification, and optional veto.

**Scope**:
- Agent-to-agent handoff with three transfer modes (full, assist, escalate)
- Context packaging with optional task snapshot (RFC 012) and conversation history
- Client notification and optional veto mechanism
- CAP interoperability profile (`xyz.agentries.handoff.workflow:1.0.0`)

*Note: AMP Standard Profile. Uses `typ=0x82` with profile-body dispatch. Error codes in 44xx range.*

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
│              AMP Infrastructure (reusable)                       │
├─────────────────────┬───────────────────────────────────────────┤
│  AMP Core           │  RFC 001 AMP Core (envelope/signing)      │
│                     │  RFC 002 Transport Bindings               │
│                     │  RFC 003 Relay & Store-and-Forward        │
├─────────────────────┴───────────────────────────────────────────┤
│        AMP Standard Profiles (xyz.agentries.*)                    │
├─────────────────────┬───────────────────────────────────────────┤
│  Universal          │  RFC 012 Task Protocol (typ=0x80)         │
│  Interactions       │  RFC 013 Negotiation Protocol (typ=0x81)  │
│                     │  RFC 014 Handoff Protocol (typ=0x82)      │
├─────────────────────┴───────────────────────────────────────────┤
│        Agentries Application Profile (AMP Full, xyz.agentries.*) │
├─────────────────────┬───────────────────────────────────────────┤
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

**AMP Infrastructure** (RFC 001-003) is general-purpose agent communication infrastructure. Any project MAY adopt it and define domain-specific Standard or Private Profiles on top (see RFC 001 §1.5 and §17). **AMP Standard Profiles** (RFC 012-014) define universal interaction patterns (task, negotiation, handoff) using the `xyz.agentries.*` namespace and registered type codes. **Agentries Application Profile** (RFC 004-011) is the built-in application profile for the Agentries ecosystem using the `xyz.agentries.*` namespace.

## Design Principles

1. **DID-Native**: Agentries DIDs as identity layer
2. **Signature-Based**: All actions cryptographically signed
3. **Decentralized-First**: Design for federation/P2P
4. **Agent-Native**: No human intermediation required
5. **Binary-Efficient**: CBOR encoding for performance
6. **Interoperable**: Optional bridges to A2A, MCP ecosystems
7. **Extensible**: Domain-specific application protocols build on AMP Core via registered type codes and application profiles

## License

This repository is licensed under the [Apache License 2.0](LICENSE). This applies to all specifications, code examples, and conformance artifacts. The Apache 2.0 license includes an explicit patent grant, ensuring that anyone implementing AMP has clear legal standing.

## Contributing

1. Fork this repo
2. Create `NNN-your-proposal.md` using `RFC-TEMPLATE.md` as a starting point
3. Open PR for discussion
4. Iterate based on feedback
5. Maintainers approve or request changes
