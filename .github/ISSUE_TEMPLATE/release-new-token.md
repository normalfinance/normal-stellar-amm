---
name: 🚀 Release New Normal Token (Synthetic Asset)
about: Track the release of a new synthetic asset and liquidity pool using deploy_pool.sh
title: "Release: New Normal Token - <SYMBOL>"
labels: ["release", "new-token", "deployment"]
---

## 🔖 Token Info

**Token Symbol:**  
e.g., `nBTC`

**Base Asset Tracked:**  
e.g., `BTC`, `ETH`, `SOL`

**Oracle Source:**  
e.g., `Pyth BTC/USD`, contract ID, etc.

## ⚙️ Pool Deployment Parameters

Provide important pool configuration:

- Initial Liquidity: e.g., `10,000 nBTC / 1,000 SOL`
- Admin Address: e.g., `GABC...`
- Fees: e.g., `0.3%`
- Any custom init parameters?

## 👤 Deployment Responsibility

**Deployer (GitHub handle or name):**  
e.g., `@your-username`

---

## ✅ Deployment Checklist

- [ ] Oracle is deployed and verified
- [ ] Token contract is deployed and verified
- [ ] Liquidity pool deployed via `deploy_pool.sh`
- [ ] Token icon and metadata added to frontend
- [ ] Indexer/subgraph integration complete
