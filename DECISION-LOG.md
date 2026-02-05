# RFC Decision Log

记录所有重要决策及其理由，避免重复讨论已决定的问题。

---

## DL-001: Protocol Supports Both Autonomous and Human-Delegated Agents

**Date**: 2026-02-04  
**RFC**: 001 (Agent Messaging Protocol)  
**Round**: 1 (Problem Statement)  
**Raised by**: Jason  

**Decision**: Protocol explicitly supports both fully autonomous agent-to-agent communication AND human-delegated agent messaging.

**Context**: Jason noted the distinction between "Agent-as-User" (autonomous) vs "Agent-as-Service" (assistant to human). Many real-world agents operate in assistant roles where:
- Messages originate from human intent
- Human operators need visibility/override capability
- Authorization chains include human delegation

**Rationale**: 
- Real-world agents exist on a spectrum of autonomy
- Excluding assistant-style agents would limit protocol adoption
- Human delegation is a valid and common use case
- Doesn't conflict with autonomous case — just additional requirements

**Implication**: 
- Add optional human visibility mechanisms (message CC)
- Ensure compatibility with RFC 005 (Delegation & Authorization)
- Protocol design should not assume full autonomy

---

## DL-002: Two-Level Delivery Confirmation

**Date**: 2026-02-04  
**RFC**: 001 (Agent Messaging Protocol)  
**Round**: 1 (Problem Statement)  
**Raised by**: Jason  

**Decision**: Split R6 into transport-level (R6a) and application-level (R6b) confirmation.

**Context**: Original R6 ("Delivery MUST be confirmed") was ambiguous about what "delivery" means.

**Rationale**:
- Transport-level = relay received message (network guarantee)
- Application-level = agent processed message (semantic guarantee)
- Both are useful but have different semantics and implementation costs
- Conflating them leads to confusion

**Implication**:
- R6a (transport): MUST — baseline reliability
- R6b (application): SHOULD — optional but recommended
- Receipt messages must clearly indicate which level

---

## Template

```
## DL-NNN: [Short Title]

**Date**: YYYY-MM-DD
**RFC**: NNN
**Round**: N
**Raised by**: [Name]

**Decision**: [One sentence]

**Context**: [Why this came up]

**Rationale**: [Why we decided this way]

**Implication**: [What changes as a result]
```

---

## DL-003: AMP is Independent Protocol, Not DIDComm Profile

**Date**: 2026-02-04  
**RFC**: 001 (Agent Messaging Protocol)  
**Round**: 2 (Requirements Review)  
**Raised by**: Nowa (operator)  

**Decision**: AMP 是独立协议，不是 DIDComm 的 profile 或扩展。

**Context**: 
- 研究了 DIDComm v2 规范
- 分析了 DIDComm 的社区和采用情况
- DIDComm 主要面向"人/机构"的数字身份通信

**Rationale**:
- AMP 目标是 AI Agent 生态，不是数字身份生态
- 需要二进制效率（CBOR），DIDComm 用 JSON
- 需要原生能力调用（RPC 语义），不只是"消息"
- DIDComm 社区方向（银行、政府、医疗）与 AI Agent 不一致

**Implication**:
- AMP 采用 CBOR 二进制格式
- AMP 包含能力调用协议（CAP_QUERY, CAP_INVOKE...）
- AMP 支持文档和凭证交换
- 可以借鉴 DIDComm 的加密概念（authcrypt/anoncrypt）但不依赖

---

## DL-004: Binary Protocol with CBOR

**Date**: 2026-02-04  
**RFC**: 001 (Agent Messaging Protocol)  
**Round**: 2 (Requirements Review)  
**Raised by**: Nowa (operator)  

**Decision**: AMP 使用 CBOR 作为二进制编码格式。

**Context**: 考虑了 TL、Protobuf、MessagePack、CBOR、FlatBuffers。

**Rationale**:
- CBOR 是 IETF 标准 (RFC 8949)
- 与 JSON 语义兼容（调试方便）
- 支持二进制数据内嵌（文档/凭证）
- 身份/加密生态已采用（COSE 签名）
- 比 Protobuf 更灵活（不强制 schema）

**Implication**:
- 所有 AMP 消息使用 CBOR 编码
- Schema 使用 CDDL (RFC 8610) 定义
- 实现需要 CBOR 库支持
