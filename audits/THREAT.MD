# **What are we working on?**

Normal is a synthetic asset protocol making it possible for investors to trade any cryptocurrency from a single decentralized exchange. Normal Tokens - the synthetic assets - are each powered by their own AMM which automatically mints/burns tokens after each swap to maintain the pool’s price to an oracle price. The typical happy path is as follows:

1. User 1 deposits 100 USDC as liquidity to a Normal AMM (let’s use nBTC - Normal Bitcoin)
2. The AMM uses the live Bitcoin price via a Reflector oracle and the pool’s current x and y token reserves to mint `x` new nBTC tokens matching the USDC
3.

# What can go wrong?

# STRIDE reminders

| **Mnemonic Threat**        | **Definition**                                                                                               | **Question**                                                                                                                                                                                                                         |
| -------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **S**poofing               | The ability to impersonate another user or system component to gain unauthorized access.                     | Is the user who they say they are?                                                                                                                                                                                                   |
| **T**ampering              | Unauthorized alteration of data or code.                                                                     | Has the data or code been modified in some way?                                                                                                                                                                                      |
| **R**epudiation            | The ability for a system or user to deny having taken a certain action.                                      | Is there enough data to “prove” the user took the action if they were to deny it?                                                                                                                                                    |
| **I**nformation Disclosure | The over-sharing of data expected to be kept private.                                                        | Is there anywhere where excessive data is being shared or controls are not properly in place to protect private information?                                                                                                         |
| **D**enial of Service      | The ability for an attacker to negatively affect the availability of a system.                               | Can someone, without authorization, impact the availability of the service or business?                                                                                                                                              |
| **E**levation of Privilege | The ability for an attacker to gain additional privileges and roles beyond what they initially were granted. | Are there ways for a user, without proper authentication (verifying identity) and authorization (verifying permission) to gain access to additional privileges, either through standard (normally legitimate) or illegitimate means? |

_To complete the STRIDE table below, apply the questions above to each of the interactions in the data flow diagram between different entities. There should be at least one issue identified for each of S, T, R, I, D, and E. Make sure to uniquely identify them, so they are easier to track in the next step._

# Threat table

| **Threat**   | **Issues**                                             |
| ------------ | ------------------------------------------------------ |
| **S**poofing | **Spoof.1** - User can gain admin or privileged access |

\*\*
**Spoof.2** - User can remove another user’s liquidity

** |
| **T**ampering | **Tamper.1\*\* - User could create fraudulent or stale oracle data

**Tamper.2** - User could submit an unauthorized contract upgrade
|
| **R**epudiation | **Repudiate.1** - The user trying to double-spend/redeem their LP tokens |
| **I**nformation Disclosure | **Info.1** - An attacker is able to gain more information on a pool than necessary

|
| **D**enial of Service | **DoS.1** - User could front-run swaps

**DoS.2** - User could take out an oracle provider

DoS.3 - An attacker could remove a large amount of liquidity at once, depegging the pool |
| **E**levation of Privilege | **Elevation.1** - An attacker is able to view admin access to the protocol |

# What are we going to do about it?

_To complete the table below, design or identify mitigations that can address each of the issues identified. Alternatively, the risk may be documented and corporately accepted by the business._

| **Threat**   | **Issues**                                             |
| ------------ | ------------------------------------------------------ |
| **S**poofing | **Spoof.1 -** User can gain admin or privileged access |

**S1R.1** -

\***\*Spoof.1.R.2** - *Second remediation step for the first Spoofing issue, etc.* |
| **T**ampering | **Tamper.1** - An attacker could create fraudulent or stale oracle data
**Tamper.1.R.1** - Supported oracles undergo strict selection criteria to ensure sufficient decentralization and security procedures.

**Tamper.2** - An attacker could submit an unauthorized contract upgrade
**Tamper.2.R.1** -

\***\* |
| **R**epudiation | **Repudiate.1** - An attacker could try to double-spend/redeem LP tokens
**Repudiate.1.R.1** - Strict value checks are applied to the amount of LP tokens to redeem when passed into contract functions. LP token burn operations are placed in the appropriate order of operations to avoid data leaks. |
| **I**nformation Disclosure | **Info.1.R.1** - *Description of the first remediation for this issue. Oftentimes, there may be more than one singular remediation required to fully address an issue. Ensure sufficient detail is provided to fully mitigate or remediate the issue.* |
| **D**enial of Service | **DoS.1** - An attacker could front-run swaps
**DoS.R1\*\* - Transactions submitted to the protocol from our app(s) are directed to a private node that doesn’t disclose transaction information useful for front-running. Parameters for swaps may also be encrypted and only visible to the contract during execution.

**DoS.2** - User could take out an oracle provider
**DoS.R2** - Pools validate oracle price data before use. An emergency oracle is set at pool creation to be used in the event the primary oracle is unusable. Privileged accounts may update/reset the oracle and/or the oracle price.

**DoS.3** - An attacker could remove a large amount of liquidity at once, de-pegging the pool
**DoS.R3** - |
| **E**levation of Privilege | **Elevation.1**- An attacker is able to view admin access to the protocol
E1R1 - There is no admin dashboard maintained. Instead, all admin (or other privileged) access is strictly accessible via contract function invokations via Stellar Lab. |

# Did we do a good job?

- Has the data flow diagram been referenced since it was created?
- Did the STRIDE model uncover any new design issues or concerns that had not been previously addressed or thought of?
- Did the treatments identified in the “What are we going to do about it” section adequately address the issues identified?
- Have additional issues been found after the threat model?
- Any additional thoughts or insights on the threat modeling process that could help improve it next time?
