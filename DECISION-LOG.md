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

---

## DL-005: RFC Renumbering by Priority

**Date**: 2026-02-06  
**RFC**: All  
**Round**: 0 (Indexing)  
**Raised by**: Nowa  

**Decision**: 按优先级重新编号 RFC，并用新编号对齐文件名与索引。

**Context**: 现有 RFC 编号与优先级不一致，部分计划中的主题缺少占位文档，导致导航和沟通成本增加。

**Rationale**:
- 编号即优先级，降低讨论与查找成本
- 通过占位文档明确路线图
- 索引与文件名一致，避免链接漂移

**Implication**:
- 更新 `README.md` 索引与优先级图
- 重命名 RFC 003 -> RFC 004，RFC 004 -> RFC 006
- 新增 RFC 003、008-011 占位文档

---

## DL-006: Three-Tier Conformance Model and Profile Extensibility

**Date**: 2026-02-22
**RFC**: 001
**Round**: 5 (v0.45)
**Raised by**: Nowa

**Decision**: 定义三层一致性模型：AMP Core / AMP Standard Profiles / AMP Private Profiles。Standard Profile 使用 `0x80-0xEF` 中一 profile 一 type code，通过 `body.action` 分发细粒度操作。Private Profile 共享 `0xF0`，通过 `body.profile` 区分。

**Context**: AMP 需要可扩展的应用协议机制，但不能耗尽 type code 空间也不能让 relay 解析 body。

**Rationale**:
- 一 profile 一 code 在信封层高效分发，无需解码 body
- `body.action` 承载细粒度动作，保持 type code 空间紧凑
- Relay 保持中立（不解析 body），路由基于 `typ` 即可
- Private Profile 共享 `0xF0` 允许无需注册的快速原型
- HELLO `profiles` 字段实现连接时兼容性发现

**Implication**:
- Standard Profile 注册需通过 §17 流程，包含 CDDL、安全要求等
- RFC 012-014 作为首批 Standard Profile 验证该模式

---

## DL-007: AMP Full 是 Application Profile，非 Standard Profile

**Date**: 2026-02-22
**RFC**: 001
**Round**: 5 (v0.46)
**Raised by**: Nowa

**Decision**: 明确 AMP Full (RFC 004-011) 是使用 AMP Core type codes (`0x00-0x7F`) 的 Application Profile 套件，不是 Standard Profile。Standard Profile (`0x80-0xEF`) 是独立的扩展机制。

**Context**: v0.45 将 AMP Full 描述为 "内建 Standard Profile"，但 AMP Full 使用 CAP_*/DOC_*/CRED_*/DELEG_*/REQUEST/RESPONSE 等多个 core type codes，不符合 Standard Profile 的 "一 profile 一 code" 规则。

**Rationale**:
- AMP Full 的 type codes 在 `0x00-0x7F` core 范围内，不在 `0x80-0xEF` 扩展范围内
- Standard Profile 的定义是 "一个注册 type code + body.action 分发"，AMP Full 不符合
- 混淆这两个概念会导致注册、协商、实现边界都不清晰

**Implication**:
- RFC 001 §1.5 修正术语，将 AMP Full 定位为 Application Profile
- Standard Profile 和 Application Profile 是两种不同的扩展路径

---

## DL-008: 命名空间迁移 org.agentries → xyz.agentries

**Date**: 2026-02-22
**RFC**: 004-014
**Round**: 0
**Raised by**: Nowa

**Decision**: 将所有 profile 和 capability 命名空间从 `org.agentries.*` 迁移到 `xyz.agentries.*`，基于实际拥有的域名 `agentries.xyz`。

**Context**: 之前使用 `org.agentries.*` 但 `agentries.org` 可能非项目所有。Reverse-domain 命名的目的是避免冲突，应基于实际拥有的域名。

**Rationale**:
- `agentries.xyz` 是项目实际拥有的域名
- `xyz.agentries.*` 符合 reverse-domain 命名惯例
- 消除潜在的命名冲突风险

**Implication**:
- RFC 004-014 所有 capability ID 和 profile name 统一使用 `xyz.agentries.*`
- 如 `xyz.agentries.payment.workflow:1.0.0`、`xyz.agentries.task.workflow:1.0.0` 等

---

## DL-009: Standard Profile typ 发送规则遵循 HELLO 协商

**Date**: 2026-02-22
**RFC**: 012, 013, 014
**Round**: 0
**Raised by**: Nowa

**Decision**: Standard Profile 消息的 `typ` 发送必须遵循 RFC 001 §13.3 的 typ selection rule：仅在 peer 在 HELLO 中声明了 `typ` 时使用注册 type code，否则回退 `0xF0`。接收方必须 dual-accept。

**Context**: RFC 012/013 初版无条件要求 "MUST use typ=0x80/0x81"，与 RFC 001 §13.3 冲突。若 peer 未经 HELLO 协商就收到未知 type code，会直接返回 `1005 UNKNOWN_TYPE`。

**Rationale**:
- RFC 001 §13.3 是 typ 选择的权威来源
- 无条件使用注册 typ 会导致未 HELLO 协商的 peer 互通失败
- 0xF0 回退是 Private-to-Standard 迁移路径的核心设计

**Implication**:
- RFC 012/013/014 §5.1 统一为条件式 typ 选择
- Dual-accept 是接收方的 MUST 要求
