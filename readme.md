<div align="center">
  <a href="https://www.normalfinance.io/">
    <img src="https://cdn.prod.website-files.com/6595b2282ea917577755d3a5/6595bb9290625dfff5df3f7e_Logo%20-%20Color.svg" alt="Normal logo" width="340"/>
  </a>
</div>

<div>
  <a href="https://discord.gg/hayb9pafjZ"><img src="https://img.shields.io/discord/928701482319101952"/></a>
  <a  href="https://github.com/normalfinance/normal-stellar-amm/releases"><img src="https://img.shields.io/github/release-pre/normalfinance/normal-stellar-amm.svg"/></a>
  <a  href="https://github.com/normalfinance/normal-stellar-amm/pulse"><img src="https://img.shields.io/github/contributors/normalfinance/normal-stellar-amm.svg"/></a>
  <a href="https://opensource.org/license/apache-2-0"><img src="https://img.shields.io/github/license/normalfinance/normal-stellar-amm"/></a>
  <a href="https://github.com/normalfinance/normal-stellar-amm/pulse"><img src="https://img.shields.io/github/last-commit/normalfinance/normal-stellar-amm.svg"/></a>
  <a href="https://github.com/normalfinance/normal-stellar-amm/pulls"><img src="https://img.shields.io/github/issues-pr/normalfinance/normal-stellar-amm.svg"/></a>
 
  <a href="https://github.com/normalfinance/normal-stellar-amm/issues"><img src="https://img.shields.io/github/issues/normalfinance/normal-stellar-amm.svg"/></a>
  <a href="https://github.com/normalfinance/normal-stellar-amm/issues"><img src="https://img.shields.io/github/issues-closed/normalfinance/normal-stellar-amm.svg"/></a>
</div>

# ✨ Normal Stellar v1 🦄

Normal is a synthetic asset protocol enabling investors to trade any crypto asset or real-world asset (RWA) from a single DEX.

## Features

- Earn $NORM by providing $XLM as liquidity to pools
- Buy and sell Normal Tokens (i.e. nBTC - Normal Bitcoin)
- Permissionlessly create synthetic tokens (coming soon...)

## Smart Contracts

- **liquidity_pool** - Exchange liquidity pool based on constant product formula (xy=k)
- **token** - [SEP-0041](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) compatible token smart contract designed for liquidity pool share management
- **liquidity_pool_router** - entry point and catalogue of liquidity pools which is capable to deploy new pools if necessary
- **liquidity_pool_plane** - contract designed to store minimum information about any liquidity pool: type, parameters, reserves. being updated on every action with the pool (deposit, swap, withdraw, parameters update, etc)

![Normal AMM Swap Flow](./docs/images/swap.png "Swap Flow")

## Built With

- [Rust](https://www.rust-lang.org/)
- [Soroban](https://soroban.stellar.org/)
- [Rust Soroban SDK](https://github.com/stellar/rs-soroban-sdk)

## Getting Started

### Prerequisites

- [Task](https://taskfile.dev/) as task runner
- installed latest Rust version
- [soroban cli](https://github.com/stellar/soroban-tools)

### Development setup

#### Clone project

`git clone git@github.com:normalfinance/normal-stellar-amm.git`

#### Build contracts

`task build`

#### Run tests

`task test`

#### (Optionally) Deploy & invoke contracts via soroban-cli

check the Soroban documentation: https://soroban.stellar.org/docs/reference/rpc

## Authors

- [@AquaToken](https://github.com/AquaToken)
- [@jblewnormal](https://github.com/jblewnormal)

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## Contact

- 📧 Email: [hello@normalfinance.io](mailto:hello@normalfinance.io)
- Telegram: [@aquarius_HOME](https://t.me/aquarius_HOME)
- 🐣 Twitter: [@normalfi](https://twitter.com/normalfi)
- 🥷🏼 GitHub: [@normalfinance](https://github.com/normalfinance)
- 👾 Discord: [@Normal](https://discord.gg/sgzFscHp4C)
- 📚 Docs: [@aquarius-aqua](https://docs.normalfinance.io/)
- 🤓 Blog: [@aquarius-aqua](https://blog.normalfinance.io/)

## License

[Apache-2.0](https://choosealicense.com/licenses/apache-2.0/)
