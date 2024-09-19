<p align="center">
  <img src="https://github.com/webb-tools/dkg-substrate/raw/master/assets/webb_banner_light.png" alt="Gadget Logo">
</p>

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust Version](https://img.shields.io/badge/rust-1.74.0%2B-blue.svg)](https://www.rust-lang.org)

# Gadget: A Powerful distributed AVS Framework
Gadget is a comprehensive framework for building AVS services on Tangle and Eigenlayer. It provides a standardized framework for building task based systems and enables developers to clearly specify jobs, slashing reports, benchmarks, and tests for offchain and onchain service interactions. We plan to integrate with other restaking infrastructures over time, if you are a project that is interested please reach out!

## Features

- Modular and extensible architecture
- Integration with [Tangle](https://twitter.com/tangle_network) and [Eigenlayer](https://www.eigenlayer.xyz/)
- Standardized job execution and submission mechanisms
- Protocol-specific blockchain connections, networking layers, and application logic
- Comprehensive testing framework

## Getting Started

### Prerequisites

- Rust 1.74.0 or higher
- Substrate blockchain with compatible job management and submission infrastructure

### Installation

1. Clone the repository:

```bash
git clone https://github.com/webb-tools/gadget/
git submodule update --init --recursive
cd gadget
```
   
2. Build the project:

```bash
cargo build --release
```
## Troubleshooting
For troubleshooting build issues, refer to [Tangle Troubleshooting](https://github.com/webb-tools/tangle?tab=readme-ov-file#-troubleshooting-) or create an issue.

### Foundry
In order to use the incredible-squaring blueprint, you need to have the Foundry CLI installed. You can install it here: [Foundry CLI](https://ethereum-blockchain-developer.com/2022-06-nft-truffle-hardhat-foundry/14-foundry-setup/)

Then, run:
```bash
cd blueprints/incredible-squaring
yarn install
forge build --root ./contracts
```

## License
Gadget is licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

We welcome contributions to Gadget! If you have any ideas, suggestions, or bug reports, please open an issue or submit a pull request.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Acknowledgements
We would like to thank the following projects for their inspiration and contributions to this project:
* 
* [DFNS CGGMP21](https://github.com/dfns/cggmp21/)
* [Threshold BLS](https://github.com/mikelodder7/blsful)
* [LIT Protocol fork of ZCash Frost](https://github.com/LIT-Protocol/frost)
* [DKLS](https://github.com/silence-laboratories/silent-shard-dkls23-ll)

## Contact
If you have any questions or need further information, please contact the developers [here](https://webb.tools/)
