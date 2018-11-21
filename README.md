# edge_bridge
This module contains the federated bridge implementation from Edgeware to eligible blockchains. The federation is currently managed by at least the active set of validators in the underlying consensus protocol. There may be more stakeholders that can be involved in the federated process, all of which are denoted as bridge authorities. The bridge authorizes a two-way peg against eligible blockchains using at least 2/3 of the stake to process incoming and outgoing requests.

# Setup
Install rust or update to the latest versions.
```
curl https://sh.rustup.rs -sSf | sh
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup update stable
cargo install --git https://github.com/alexcrichton/wasm-gc
```

You will also need to install the following packages:

Linux:
```
sudo apt install cmake pkg-config libssl-dev git
```

Mac:
```
brew install cmake pkg-config openssl git
```

# Overview
The bridge enables users on one blockchain to exchange Edgeware-compliant, non-native tokens (such as an Edgeware ERC20 token) for native Edgeware tokens and vice versa.

### Deposit
Interested individuals will deposit compliant tokens in a mechanism run by the active set of bridge authorities. Each bridge authority should be incentivized to sign new deposit messages in a timely manner, enabling the interested individual the ability to present an aggregate signature or list of signatures to this module to process the creation of native tokens.

### Withdraw
Interested individuals will request to withdraw native Edgeware tokens using this module. Bridge authorities should be incentivized to sign new withdraw messages in a timely manner, enabling the interested individual the ability to present an aggregate signature or list of signatures to a mechanism on the target blockchain for minting of Edgeware-compliant, non-native tokens.