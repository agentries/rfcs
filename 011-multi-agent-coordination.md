# RFC 011: Multi-Agent Coordination & Group Messaging

**Status**: Planned (Future)  
**Authors**: TBD  
**Created**: 2026-02-06  
**Updated**: 2026-02-06  
**Depends On**: RFC 001 (AMP), RFC 006 (Session Protocol)

---

## Abstract

This RFC proposes semantics for coordinating multiple agents within shared tasks, including group messaging, role assignment, and coordination metadata.

---

## 1. Problem Statement

Complex workflows often require multiple agents working together. Without a standard coordination layer, each system invents incompatible group messaging patterns.

---

## 2. Scope

- Group message addressing and membership
- Roles, responsibilities, and handoff patterns
- Coordination metadata and conflict resolution

---

## 3. Out of Scope

- Transport-level routing (see RFC 002)
- Payment or escrow flows (see RFC 007)

---

## 4. Open Questions

- How to represent leader or coordinator roles
- Backward compatibility with single-agent AMP flows
