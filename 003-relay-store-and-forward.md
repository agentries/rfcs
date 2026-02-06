# RFC 003: Relay & Store-and-Forward

**Status**: Planned (Future)  
**Authors**: TBD  
**Created**: 2026-02-06  
**Updated**: 2026-02-06  
**Depends On**: RFC 001 (AMP), RFC 002 (Transport Bindings)

---

## Abstract

This RFC defines relay discovery and store-and-forward semantics so agents can exchange messages when one or both are offline.

---

## 1. Problem Statement

Agents are not always online or reachable, but AMP assumes direct delivery. Without relays, interoperability breaks for mobile, intermittently connected, or firewalled agents.

---

## 2. Scope

- Relay discovery (via DID Document or directory)
- Store-and-forward semantics and retention windows
- Relay federation and handoff between relays
- Offline message retrieval and acknowledgement

---

## 3. Out of Scope

- Payment settlement for relay services
- Reputation scoring for relays

---

## 4. Open Questions

- Minimum relay retention window defaults
- Relay-to-relay authentication model
