# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This repository contains **RFC specifications** for the Agentries ecosystem, focusing on the **Agent Messaging Protocol (AMP)** - a native communication protocol for AI agent-to-agent communication. This is a pure specification/documentation repository with no code to build or test.

## Repository Structure

```
README.md                         # RFC process overview and index
AMP-FIRST-PRINCIPLES.md           # Design rationale (why these decisions)
DECISION-LOG.md                   # Architectural decision records
001-agent-messaging-protocol.md   # Core AMP spec (Draft v3.1) - foundational
002-transport-bindings.md         # WebSocket/HTTP/TCP bindings (Draft v0.3)
003-capability-negotiation.md     # Capability versioning (Proposal)
004-session-protocol.md           # Multi-turn conversations (Proposal)
005-delegation-authorization.md   # Delegation credentials (Proposal)
007-agent-payment-protocol.md     # Agent economics (Proposal)
```

## Key Design Decisions

- **DID-Native**: Agents identified by Decentralized Identifiers
- **CBOR Encoding**: Binary protocol (RFC 8949), not JSON
- **Signature-Based**: All messages cryptographically signed (Ed25519); encryption optional
- **Three-Layer Architecture**: Transport → Security → Application
- **AMP is independent**: Not a DIDComm profile; designed specifically for AI agents

## RFC Maturity Workflow

Proposal → Draft → Review → Accepted → Implemented → Rejected/Withdrawn

Current state: RFC 001-002 in Draft; RFC 003-007 in Proposal (outlines only).

## Document Conventions

- Uses RFC 2119 language: MUST, SHOULD, MAY, etc.
- Requirements labeled R1, R2, etc.
- ASCII diagrams for message flows and architecture
- JSON/YAML examples for message structures
- Open Questions sections indicate areas needing input

## Reading Order for Context

1. `README.md` - Overview and RFC index
2. `AMP-FIRST-PRINCIPLES.md` - Understand the "why" behind design choices
3. `001-agent-messaging-protocol.md` - Core protocol (all other RFCs depend on this)
4. `DECISION-LOG.md` - Key architectural decisions with rationale
5. Extensions (003-007) as needed

## Dependencies Between RFCs

- RFC 001 (AMP Core) is foundational
- RFC 002 (Transport) depends on RFC 001
- RFC 003+ depend on RFC 001 and/or Agentries Core
