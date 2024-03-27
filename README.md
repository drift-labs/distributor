# Merkle-distributor

A program and toolsets for distributing tokens efficiently via uploading a [Merkle root](https://en.wikipedia.org/wiki/Merkle_tree).

## Sharding merkle tree

Thanks Jito for excellent [Merkle-distributor project](https://github.com/jito-foundation/distributor). In Jupiter, We fork the project and add some extra steps to make it works for a large set of addresses.

There are issues if the number of airdrop addresses increases:
- The size of the proof increases, that may be over solana transaction size limit.
- Too many write locked accounts duration hot claming event, so only some transactions are get through.

In order to tackle it, we break the large set of addresses to smaller merkle trees, like 12000 addresses for each merkle tree. Therefore, when user claim, that would write lock on different accounts as well as reduces proof size.

Before are follow toolset to build sharding merkle trees

## CLI
Build and deploy sharding merkle trees:

```
cd cli
cargo build
../target/debug/cli create-merkle-tree --csv-path [PATH_TO_LARGE_SET_OF_ADDRESS] --merkle-tree-path [PATH_TO_FOLDER_STORE_ALL_MERKLE_TREES] --max-nodes-per-tree 12000
../target/debug/cli --mint [TOKEN_MINT] --keypair-path [KEY_PAIR] --rpc-url [RPC] new-distributor --start-vesting-ts [START_VESTING] --end-vesting-ts [END_VESTING] --merkle-tree-path [PATH_TO_FOLDER_STORE_ALL_MERKLE_TREES] --clawback-start-ts [CLAWBACK_START] --enable-slot [ENABLE_SLOT]
../target/debug/cli --mint [TOKEN_MINT] --keypair-path [KEY_PAIR] --rpc-url [RPC] fund-all --merkle-tree-path [PATH_TO_FOLDER_STORE_ALL_MERKLE_TREES]
```

Anyone can verify the whole setup after that:

```
../target/debug/cli --mint [TOKEN_MINT] --keypair-path [KEY_PAIR] --rpc-url [RPC] verify --merkle-tree-path [PATH_TO_FOLDER_STORE_ALL_MERKLE_TREES] --clawback-start-ts [CLAWBACK_START] --enable-slot [ENABLE_SLOT] --admin [ADMIN]
```

## API
Axum server under `/api` serves merkle proof for users. We add some more caching functionality and some helpers
to make it eaier to determine how much a user is eligible to claim.

* API server tries to cache account data locally
    * calls `getProgramAccounts` for each `Distributor` on startup and saves all `ClaimStatus` accounts
    * does `programSubscribe` on `ClaimStatus` accounts to update cache in real time
        * _maybe we should do some smart cache updating based on calls to `/claim`_
    * cache is backed `dashmap`, each key-value pair is about 180 bytes (excl overhead of structs and DashMap)
        * 10_000 accounts: 1.8MB
        * 100_000 accounts: 18MB
        * 1_000_000 accounts: 180MB
* endpoints
    * `GET /distributors` returns `MerkleDistributor` account info
    * `GET /user/:user_pubkey`: returns merkle proof and amounts for a user
        * `404` if user doesn't exist in the tree.
        * `500` if user exists in tree but has no proof
    * `GET /claim/:user_pubkey`: returns `ClaimStatus` info for user.
        * `404` if `ClaimStatus` account doesn't exist (user did not claim yet or does not exist in tree)
    * `GET /eligibility/:user_pubkey`: returns a combination of `/user` and `/claim` info
        * `200` with claim info if user is in tree (`claimed_amount` is 0 if pending)
        * `404` if user has no claim

```
cd api
cargo build
../target/debug/drift-airdrop-api --merkle-tree-path [PATH_TO_FOLDER_STORE_ALL_MERKLE_TREES] \
--program-id E7HtfkEMhmn9uwL7EFNydcXBWy5WCYN1vFmKKjipEH1x \
--mint 4m9qg7yfJQcCLobNFgHCw6a9ZEBAYyxyy5YGef3ijknU \
--rpc-url https://your.rpc.com \
--ws-url wss://your.rpc.com

```