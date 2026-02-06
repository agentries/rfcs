# RFC 008: Agent Discovery & Directory

**Status**: Planned (Future)  
**Authors**: TBD  
**Created**: 2026-02-06  
**Updated**: 2026-02-06  
**Depends On**: Agentries Core (DID, Capabilities)

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

---

## 3. Out of Scope

- Reputation scoring (see RFC 009)
- Transport bindings (see RFC 002)

---

## 4. Open Questions

- Minimum metadata required for listing
- Cache invalidation and staleness rules
