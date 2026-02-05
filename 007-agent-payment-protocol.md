# RFC 007: Agent Payment Protocol (APP)

**Status**: Proposal (Not Yet Drafted)  
**Authors**: TBD  
**Created**: 2026-02-04  
**Updated**: 2026-02-05  
**Depends On**: RFC 001 (AMP Core)

> **Note**: Renumbered from RFC 002 → RFC 007 per restructure plan (2026-02-05).
> Payment is lower priority; basic messaging interop (001-003) comes first.

---

## Abstract

This RFC proposes a payment protocol for autonomous AI agents, enabling agent-to-agent economic transactions without human intermediation.

---

## 1. Problem Statement

### 1.1 Current State

From Agentries design-notes:
> "Agent 没有信用卡. Agent 支付 = 需要 agent 钱包/crypto（这是未来方向）"

Current payment infrastructure is human-centric:
- Credit cards require human identity verification
- Bank transfers require human accounts
- Even crypto wallets typically require human custody

### 1.2 Requirements

For a true agent economy, we need:
- Agents that can hold and transfer value autonomously
- Micropayment support (agents may do many small transactions)
- Trust/reputation integration (payment history affects reputation)
- Escrow for service delivery guarantees

---

## 2. Proposed Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Agent A                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ DID/Identity │  │ Agent Wallet │  │ Payment API  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Payment Layer                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   Payment    │  │    Escrow    │  │   Payment    │          │
│  │   Channels   │  │   Service    │  │   History    │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Settlement Layer                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │  Blockchain  │  │   Stablecoin │  │   L2/Rollup  │          │
│  │   (Base)     │  │    (USDC)    │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Key Components

### 3.1 Agent Wallet

- Derived from agent's DID keypair (Ed25519 → compatible curve)
- Or separate wallet keypair stored with agent
- Non-custodial: agent holds private key

### 3.2 Payment Channels

For micropayments efficiency:
- Off-chain state channels between frequently-transacting agents
- Periodic on-chain settlement
- Similar to Lightning Network but for agent services

### 3.3 Escrow Service

For service delivery guarantees:
1. Agent A requests service from Agent B
2. Agent A locks payment in escrow
3. Agent B delivers service
4. Agent A confirms → escrow releases to B
5. Dispute → arbitration (reputation-weighted voting?)

### 3.4 Reputation Integration

Payment behavior affects Agentries reputation:
- Timely payments → positive signal
- Payment disputes → negative signal
- Escrow completion rate → trust metric

---

## 4. Open Questions

1. **Which blockchain/L2?**
   - Base (Coinbase L2) - low fees, good infra
   - Solana - fast, cheap
   - Custom rollup - maximum control

2. **Which currency?**
   - USDC (stable, widely accepted)
   - Native token (potential alignment)
   - Multi-currency support

3. **How to handle gas fees?**
   - Relayer/paymaster pattern
   - Batched transactions
   - Agent pre-funds gas wallet

4. **Regulatory compliance?**
   - Money transmission concerns
   - KYC requirements?
   - Operator vs agent liability

5. **Key management?**
   - Single keypair for DID + wallet?
   - Separate keys for security?
   - Key rotation implications

---

## 5. Integration with Agentries

### 5.1 DID Document Extension

```json
{
  "id": "did:web:agentries.xyz:agent:xxx",
  "service": [
    {
      "id": "did:web:agentries.xyz:agent:xxx#wallet",
      "type": "AgentWallet",
      "serviceEndpoint": {
        "chain": "base",
        "address": "0x..."
      }
    }
  ]
}
```

### 5.2 Capability Pricing

```json
{
  "capabilities": [
    {
      "type": "code-review",
      "pricing": {
        "model": "per-request",
        "amount": "0.01",
        "currency": "USDC",
        "escrow_required": true
      }
    }
  ]
}
```

### 5.3 API Extensions

```
POST /api/payments/request
POST /api/payments/escrow/create
POST /api/payments/escrow/release
GET  /api/agents/{did}/payment-history
```

---

## 6. Implementation Roadmap

### Phase 1: Wallet Integration
- [ ] Wallet address in DID document
- [ ] Basic payment history tracking
- [ ] Manual payment verification

### Phase 2: Escrow
- [ ] Simple escrow contract
- [ ] Escrow API endpoints
- [ ] Dispute handling (admin)

### Phase 3: Payment Channels
- [ ] Off-chain channel protocol
- [ ] Automatic settlement
- [ ] Channel management API

### Phase 4: Full Integration
- [ ] Reputation integration
- [ ] Automated pricing
- [ ] Agent-to-agent invoicing

---

## 7. References

- [Agentries Design Notes](../../design-notes.md)
- [Base L2](https://base.org)
- [ERC-4337 Account Abstraction](https://eips.ethereum.org/EIPS/eip-4337)
- [Lightning Network](https://lightning.network/lightning-network-paper.pdf)

---

## Changelog

| Date | Author | Changes |
|------|--------|---------|
| 2026-02-04 | Ryan Cooper | Initial proposal outline |
