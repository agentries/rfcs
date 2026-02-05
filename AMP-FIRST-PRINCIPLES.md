# AMP First Principles Analysis

**Document Type**: Research & Design Rationale  
**Date**: 2026-02-04  
**Authors**: Ryan Cooper, Jason Apple Huang

---

## 1. What Is An Agent?

从第一性原理出发，先定义"Agent"：

```
Agent = Autonomous Software Entity + Identity + Capabilities + Communication
```

| 属性 | 人类 | Agent |
|------|------|-------|
| 身份 | 政府颁发 (护照) | 自主生成 (DID) |
| 在线状态 | 间歇性 | 可能 24/7 或完全离线 |
| 消息处理 | 阅读、忽略、延迟 | 确定性处理或失败 |
| 决策 | 模糊、情感 | 逻辑、可验证 |
| 代理 | 可以授权他人 | 可以接受委托 |

**Agent 不是人类的模拟，而是一种新型实体。**

---

## 2. Agent 通信的核心需求

### 2.1 必须满足的需求 (MUST)

| 需求 | 理由 |
|------|------|
| **身份验证** | 必须知道消息来自谁 |
| **消息完整性** | 必须知道消息未被篡改 |
| **送达确认** | 必须知道消息到达了 |
| **重放保护** | 必须拒绝重复/过期消息 |
| **传输无关** | 不能绑定单一传输方式 |

### 2.2 应该满足的需求 (SHOULD)

| 需求 | 理由 |
|------|------|
| **保密性 (E2E)** | 中间节点不应读取内容 |
| **处理确认** | 知道消息被处理了（vs 仅送达） |
| **离线支持** | Agent 不总是在线 |
| **批量处理** | 高频场景需要效率 |
| **可扩展** | 支持新消息类型 |

### 2.3 可选需求 (MAY)

| 需求 | 理由 |
|------|------|
| **匿名发送** | 某些场景需要 |
| **不可否认性** | 法律/合规场景 |
| **前向安全** | 密钥泄露后保护历史消息 |

---

## 3. 现有协议分析

### 3.1 协议对比

| 协议 | 身份 | 加密 | 传输 | 异步 | 去中心化 | Agent 适用性 |
|------|------|------|------|------|----------|-------------|
| **SMTP/IMAP** | 域名 | 可选 | TCP | ✅ | 联邦 | ⭐⭐ 太老 |
| **XMPP** | JID | TLS/E2E | TCP | ✅ | 联邦 | ⭐⭐⭐ 可用 |
| **Matrix** | @user:server | E2E可选 | HTTP | ✅ | 联邦 | ⭐⭐⭐ 可用 |
| **DIDComm** | DID | ✅ 必需 | Any | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| **MTProto** | auth_key | ✅ 必需 | Any | ✅ | 中心化 | ⭐⭐⭐⭐ |
| **AMQP** | 连接级 | TLS | TCP | ✅ | Broker | ⭐⭐⭐ |

### 3.2 DIDComm 的关键洞察

DIDComm 是目前最接近 Agent 需求的协议：

**核心设计原则：**
1. **消息式、异步、单工** — 不是请求-响应！
2. **传输无关** — HTTP, WebSocket, Bluetooth, NFC, 邮件, 纸条...
3. **可路由** — Alice 可以不直连 Bob 就能通信
4. **无会话** — 协议层无状态（可在上层构建会话）

**三种消息格式：**
```
plaintext          → 明文（构建块，很少直接传输）
signed(plaintext)  → 签名（不可否认）
encrypted(plaintext) → 加密（保密 + 完整性）
```

**加密组合：**
```
authcrypt(plaintext)         → 默认选择，证明发送者身份
anoncrypt(plaintext)         → 匿名发送
anoncrypt(sign(plaintext))   → 不可否认 + 保密
```

### 3.3 MTProto 的关键洞察

Telegram 的 MTProto 有工程上的精妙设计：

**三层架构：**
```
High-Level (RPC/API)      ← 业务逻辑
Cryptographic Layer       ← 加密/签名
Transport (TCP/WS/HTTP)   ← 传输
```

**Message ID 设计：**
```
msg_id ≈ unix_time × 2³²
```
- 时间相关 → 自然有序
- 拒绝 >300s 过期的消息 → 防重放
- 客户端/服务端 msg_id 不同 → 区分方向

**Server Salt：**
- 64-bit 随机数，每 30 分钟更换
- 防止重放攻击

**Content-Related 区分：**
- 需要 ACK 的消息 vs 不需要的（ACK 本身不需要 ACK）

---

## 4. Agent 特有场景

### 4.1 场景分类

| 场景 | 特点 | 例子 |
|------|------|------|
| **P2P 通信** | 两个自主 agent | Agent A 问 Agent B 问题 |
| **服务调用** | 请求-响应 | Agent 调用 API 服务 |
| **人类委托** | 代表人类行动 | 助理 agent 发邮件 |
| **多方协作** | 工作流 | 3+ agents 协作完成任务 |
| **通知/事件** | 单向推送 | 监控 agent 发警报 |
| **长任务更新** | 进度报告 | 构建 agent 报告进度 |

### 4.2 与人类消息的关键区别

| 维度 | 人类消息 | Agent 消息 |
|------|----------|------------|
| 频率 | 低（秒/分钟级） | 可能很高（毫秒级） |
| 批量 | 很少 | 常见 |
| 格式 | 自然语言为主 | 结构化为主 |
| 处理 | 可能忽略 | 必须处理或报错 |
| 重试 | 人工决定 | 自动策略 |
| 路由 | 简单（直达） | 可能复杂（多跳） |

### 4.3 Agent 特有需求

1. **能力协商** — Agent 需要发现对方能做什么
2. **协议协商** — Agent 需要协商用什么版本/格式
3. **委托链** — Agent 可能代表另一个 agent（代表人类）
4. **状态机** — 很多交互遵循明确的协议状态机
5. **错误语义** — 需要机器可读的错误码

---

## 5. AMP 设计原则

基于以上分析，AMP 应遵循以下原则：

### 5.1 核心原则

1. **DID-Native** — 身份基于 DID，不依赖中心化身份提供者
2. **签名优先** — 所有消息默认签名，加密可选
3. **传输无关** — 不绑定特定传输协议
4. **异步优先** — 不假设同步请求-响应
5. **确定性送达** — 明确的送达和处理确认机制

### 5.2 借鉴的设计

**从 DIDComm 借鉴：**
- 消息格式（plaintext / signed / encrypted）
- DID 解析获取端点和公钥
- 路由机制
- 传输无关设计

**从 MTProto 借鉴：**
- 时间相关的 Message ID
- Server Salt 防重放
- Content-Related 区分（需要 ACK vs 不需要）
- 三层架构分离

**从 XMPP 借鉴：**
- 可扩展的消息类型
- 存在/状态机制（可选）

### 5.3 AMP 独特设计

**针对 Agent 场景的增强：**

1. **能力声明** — 消息可携带发送者的能力摘要
2. **批量消息** — 原生支持消息容器
3. **协议头** — 支持声明遵循的应用协议
4. **委托证明** — 支持携带委托凭证
5. **处理语义** — 明确区分 "收到" vs "处理成功" vs "处理失败"

---

## 6. AMP 与其他协议的关系

> **Decision**: AMP is an independent protocol, not a DIDComm profile or extension. See [DL-003](DECISION-LOG.md#dl-003-amp-is-independent-protocol-not-didcomm-profile).

### 6.1 为什么独立而非扩展 DIDComm？

| 考量 | DIDComm | AMP |
|------|---------|-----|
| 目标场景 | 人/机构的数字身份通信 | AI Agent 生态 |
| 社区方向 | 银行、政府、医疗 | AI、自动化、开发者 |
| 消息格式 | JSON | CBOR (二进制效率) |
| 核心功能 | 通用消息 | 能力调用 (RPC 语义) |

DIDComm 设计优秀，但目标场景不同。硬塞 Agent 需求会导致两边都不满意。

### 6.2 从 DIDComm 借鉴的概念

尽管独立，AMP 借鉴了 DIDComm 的优秀设计：

- **加密模式**: `authcrypt` / `anoncrypt` 概念
- **DID 解析**: 通过 DID Document 获取端点和公钥
- **传输无关**: 协议不绑定特定传输
- **路由机制**: 中介/转发模型

### 6.3 互操作策略

AMP 提供可选的互操作层，而非协议依赖：

```
┌─────────────────────────────────────────┐
│              AMP Core                    │
│  (独立协议, DID-native, CBOR)           │
├─────────────────────────────────────────┤
│        Optional Bridges                  │
│  • A2A Agent Card export                │
│  • MCP tool exposure                    │
│  • DIDComm message translation          │
└─────────────────────────────────────────┘
```

这意味着：
- AMP agent 可以被 A2A 生态发现
- AMP agent 可以暴露为 MCP tools
- AMP 消息可以桥接到 DIDComm（需要转换层）
- **但 AMP 不依赖这些协议**

---

## 7. 开放问题

1. ~~**是否直接采用 DIDComm v2？**~~ → **已决策**: AMP 是独立协议 (DL-003)
2. **Relay 架构** — 中心化 vs 联邦 vs P2P？
3. **消息持久化** — Relay 存多久？
4. **与 Agentries 的集成深度** — Agentries 只做 DID，还是也做 Relay？
5. **向后兼容** — 如何与现有系统（Email、HTTP API）桥接？

---

## 8. 参考资料

- [DIDComm Messaging Spec v2](https://identity.foundation/didcomm-messaging/spec/)
- [DIDComm Guidebook](https://didcomm.org/book/v2/)
- [MTProto Protocol](https://core.telegram.org/mtproto)
- [XMPP RFCs](https://xmpp.org/rfcs/)
- [Matrix Spec](https://spec.matrix.org/)
- [AMQP 1.0](https://www.amqp.org/specification/1.0/amqp-org-download)

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-04 | Ryan Cooper | Initial analysis based on DIDComm and MTProto study |
| 2026-02-04 | Ryan Cooper | Rewrote Section 6: AMP is independent protocol (aligns with DL-003), DIDComm concepts borrowed but not dependent |
