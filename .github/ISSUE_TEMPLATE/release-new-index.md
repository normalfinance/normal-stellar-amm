---
name: 📊 Create New Index Fund
about: Proposal to create a new index fund composed of Normal synthetic tokens
title: "[Index Fund] NAME_OF_FUND"
labels: fund, new-index
assignees: ""
---

## 🏷 Fund Name

_What is the name of the fund?_

> Example: Normal ETH-BTC Index Fund

## 🎯 Fund Mission

_What is the purpose or theme of this index fund?_

> Example: Track the combined performance of ETH and BTC as a blue-chip crypto index.

## 💼 Component Assets

_List the synthetic Normal tokens included in the fund, their symbols, and weightings._

| Token      | Symbol | Weight (%) |
| ---------- | ------ | ---------- |
| Normal ETH | nETH   | 50%        |
| Normal BTC | nBTC   | 50%        |

> Add more rows as needed.

## ⚖️ Rebalancing Strategy

_Describe the rebalancing frequency and method (if any)._

> Example: Rebalance monthly to maintain target weights using oracle prices.

## 👤 Permissions & Admin

_List any specific roles, multisig accounts, or contracts allowed to manage or update the fund._

> Example: normalfund.admin multisig

## 🛠 Deployment Details

_Any scripts or CLI commands used to initialize the index fund contract or mint tokens._

> Example: `deploy_index_fund.sh` with above parameters

## 📅 Proposed Launch Date

_When do you plan to deploy or activate the fund?_

> Example: 2025-08-01

---

### ✅ Checklist

- [ ] All component assets are deployed and liquid
- [ ] Oracle price feeds confirmed and stable
- [ ] Weighting sums to 100%
- [ ] Rebalancing logic approved
- [ ] Fund contract reviewed by at least one team member
