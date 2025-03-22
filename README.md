# Terp Network Core Cosmwasm Libraries

## TODO: 
- prep for us of v019 cw721 libraries
- fix bugs
- add cw-orch scripts 
- add agreement based contracts
- update earlybird contract with latest features 
- 
This repository contains the core contracts and libraries that are shared among all Terp Network protocols.
- replcae asset type used in market (removed terr import)
- factory utils query response to use cw721 v19 collection info metadata


| NFT Contracts                                                     | Description                                                                                      |
|-------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
 | [Earlybird](./contracts/nft/earlybirds/earlybird/README.md)      |  Support to set a list of addresses that meet a requirement                                                   |
| [Earlybird Flex](./contracts/nft/earlybirds/earlybird-flex/README.md)      |                                                     |
| [Earlybird Immutable](./contracts/nft/earlybirds/earlybird-immutable/README.md)      | Once deployed, address in eb contract cannot be updated                                                    |
| [Base Factory](./contracts/nft/factories/base-factory/README.md)      |                                                     |
| [Open Edition Factory](./contracts/nft/factories/open-edition-factory/README.md)      |                                                     |
| [Vending Factory](./contracts/nft/factories/vending-factory/README.md)      |                                                     |
| [Base Minter](./contracts/nft/minters/base-minter/README.md)      |                                                     |
| [Open Edition Minter](./contracts/nft/minters/open-edition-minter/README.md)      |                                                     |
| [Vending Minter](./contracts/nft/minters/vending-minter/README.md)      |                                                     |
| [Vending Minter Earlybird Flex](./contracts/nft/minters/vending-minter-eb-flex/README.md)      |                                                     |

| Revenue Contracts                                                     | Description                                                                                      |
|-------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| [Terp Fair Burn](./contracts/revenue/fair-burn/README.md)      |      
| [Terp Residual Registry](./contracts/revenue/residual-registry/README.md)      | Contract for fees and Developer Residual.       
| [Terp Splits](./contracts/revenue/splits/README.md)      |                                                     |

___
| Core Packages                                                     | Description                                                                                      |
|-------------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| [Controllers](./packages/actions/controllers/README.md)      |      
| [Ethereum Signature Verification](./packages/actions/ethereum-verify/README.md)      |      
| [Mint Hooks](./packages/actions/mint-hooks/README.md)      |      
| [Terp Index Query](./packages/actions/terp-index-query/README.md)      |     
| [Factory Utils](./packages/nft/factory-utils/README.md)      |      
| [Minter Utils](./packages/nft/minter-utils/README.md)      |      
| [Terp Metadata](./packages/nft/terp-metadata/README.md)      |      
| [cw721](./packages/nft/cw721/README.md)      |      
| [Terp Fee](./packages/revenue/terp-fee/README.md)      |      
| [Terp Multi Test](./packages/utils/terp-multi-test/README.md)      |      
| [Terp SDK](./packages/utils/terp-sdk/README.md)      |      
| [Test Suite](./packages/utils/test-suite/README.md)      |      
| [Unit Test](./packages/utils/unit-tests/README.md)      |      


> *Heavily modified fork of Stargaze [core](https://github.com/public-awesome/core) & [launchpad contract](https://github.com/public-awesome/launchpad). Massive respect to its contributors.*


```
```

### NOTICE
Please use at your own risk and do not trust, but verify the functionality of these contracts.