# indy-did-resolver
Proof of Concept resolver for the [did:indy method](https://hyperledger.github.io/indy-did-method/).

The code depends on [indy-vdr](https://github.com/hyperledger/indy-vdr) and could be merged ideally in the longterm.

This software could be used as a driver for the [Universal Resolver](https://github.com/decentralized-identity/universal-resolver).

# Usage

The project can be built using standard rust tooling: `cargo build` or exectued via `cargo run` 

Logging can be enabled using the environment variable`RUST_LOG`, e.g. `RUST_LOG=debug ./indy-did-driver`

### Default configuration
- default port is 8080
- default network registry is https://github.com/IDunion/indy-did-networks

The driver can be reached via HTTP, e.g.  curl http://localhost:8080/1.0/identifiers/<did>

### CLI options
```
    -f, --genesis-filename <GENESIS_FILENAME>
            Pool transaction genesis filename [default: pool_transactions_genesis.json]

    -h, --help
            Print help information

    -n, --github-network <GITHUB_NETWORKS>
            github repository for registered networks [default: https://github.com/IDunion/indy-did-
            networks]

    -p, --port <PORT>
            Port to expose [default: 8080]

    -s, --source <SOURCE>
            source to use, allowed values are path or github [default: ]

    -V, --version
            Print version information
```
### Local development

The resolver can also be configured to resolve local/custom indy networks for development purposes, e.g. [von-network](https://github.com/bcgov/von-network).
Create a folder structure like the following:

```
networks/
   └──local/
         └──pool_transactions_genesis.json

```
Start the indy-did-driver with the option `-s <path/to/networks>` and resolve via `did:indy:local:<DID>`
