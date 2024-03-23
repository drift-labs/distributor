export type MerkleDistributor = {
  "version": "0.0.1",
  "name": "merkle_distributor",
  "instructions": [
    {
      "name": "newDistributor",
      "docs": [
        "READ THE FOLLOWING:",
        "",
        "This instruction is susceptible to frontrunning that could result in loss of funds if not handled properly.",
        "",
        "An attack could look like:",
        "- A legitimate user opens a new distributor.",
        "- Someone observes the call to this instruction.",
        "- They replace the clawback_receiver, admin, or time parameters with their own.",
        "",
        "One situation that could happen here is the attacker replaces the admin and clawback_receiver with their own",
        "and sets the clawback_start_ts with the minimal time allowed. After clawback_start_ts has elapsed,",
        "the attacker can steal all funds from the distributor to their own clawback_receiver account.",
        "",
        "HOW TO AVOID:",
        "- When you call into this instruction, ensure your transaction succeeds.",
        "- To be extra safe, after your transaction succeeds, read back the state of the created MerkleDistributor account and",
        "assert the parameters are what you expect, most importantly the clawback_receiver and admin.",
        "- If your transaction fails, double check the value on-chain matches what you expect."
      ],
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MerkleDistributor"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "version"
              }
            ]
          }
        },
        {
          "name": "clawbackReceiver",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Clawback receiver token account"
          ]
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The mint to distribute."
          ]
        },
        {
          "name": "tokenVault",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Token vault",
            "Should create previously"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin wallet, responsible for creating the distributor and paying for the transaction.",
            "Also has the authority to set the clawback receiver and change itself."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        },
        {
          "name": "associatedTokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Associated Token] program."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Token] program."
          ]
        }
      ],
      "args": [
        {
          "name": "version",
          "type": "u64"
        },
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "maxTotalClaim",
          "type": "u64"
        },
        {
          "name": "maxNumNodes",
          "type": "u64"
        },
        {
          "name": "startVestingTs",
          "type": "i64"
        },
        {
          "name": "endVestingTs",
          "type": "i64"
        },
        {
          "name": "clawbackStartTs",
          "type": "i64"
        },
        {
          "name": "enableSlot",
          "type": "u64"
        },
        {
          "name": "closable",
          "type": "bool"
        }
      ]
    },
    {
      "name": "closeDistributor",
      "docs": [
        "only available in test phase"
      ],
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "relations": [
            "admin",
            "token_vault"
          ]
        },
        {
          "name": "tokenVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Clawback receiver token account"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin wallet, responsible for creating the distributor and paying for the transaction.",
            "Also has the authority to set the clawback receiver and change itself."
          ]
        },
        {
          "name": "destinationTokenAccount",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "account receive token back"
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "closeClaimStatus",
      "docs": [
        "only available in test phase"
      ],
      "accounts": [
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "claimant",
            "admin"
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
    {
      "name": "setEnableSlot",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Payer to create the distributor."
          ]
        }
      ],
      "args": [
        {
          "name": "enableSlot",
          "type": "u64"
        }
      ]
    },
    {
      "name": "newClaim",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Claim status PDA"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "ClaimStatus"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "claimant"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "MerkleDistributor",
                "path": "distributor"
              }
            ]
          }
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Account to send the claimed tokens to."
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Who is claiming the tokens."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        }
      ],
      "args": [
        {
          "name": "amountUnlocked",
          "type": "u64"
        },
        {
          "name": "amountLocked",
          "type": "u64"
        },
        {
          "name": "proof",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "claimLocked",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Claim Status PDA"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "ClaimStatus"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "claimant"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "MerkleDistributor",
                "path": "distributor"
              }
            ]
          }
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Account to send the claimed tokens to.",
            "Claimant must sign the transaction and can only claim on behalf of themself"
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Who is claiming the tokens."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "clawback",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The Clawback token account."
          ]
        },
        {
          "name": "claimant",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Claimant account",
            "Anyone can claw back the funds"
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "setClawbackReceiver",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "newClawbackAccount",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "New clawback account"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin signer"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "setAdmin",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin signer"
          ]
        },
        {
          "name": "newAdmin",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "New admin account"
          ]
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "claimStatus",
      "docs": [
        "Holds whether or not a claimant has claimed tokens."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "claimant",
            "docs": [
              "Authority that claimed the tokens."
            ],
            "type": "publicKey"
          },
          {
            "name": "lockedAmount",
            "docs": [
              "Locked amount"
            ],
            "type": "u64"
          },
          {
            "name": "lockedAmountWithdrawn",
            "docs": [
              "Locked amount withdrawn"
            ],
            "type": "u64"
          },
          {
            "name": "unlockedAmount",
            "docs": [
              "Unlocked amount"
            ],
            "type": "u64"
          },
          {
            "name": "unlockedAmountClaimed",
            "docs": [
              "Unlocked amount claimed"
            ],
            "type": "u64"
          },
          {
            "name": "closable",
            "docs": [
              "indicate that whether admin can close this account, for testing purpose"
            ],
            "type": "bool"
          },
          {
            "name": "admin",
            "docs": [
              "admin of merkle tree, store for for testing purpose"
            ],
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "merkleDistributor",
      "docs": [
        "State for the account which distributes tokens."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bump",
            "docs": [
              "Bump seed."
            ],
            "type": "u8"
          },
          {
            "name": "version",
            "docs": [
              "Version of the airdrop"
            ],
            "type": "u64"
          },
          {
            "name": "root",
            "docs": [
              "The 256-bit merkle root."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "mint",
            "docs": [
              "[Mint] of the token to be distributed."
            ],
            "type": "publicKey"
          },
          {
            "name": "tokenVault",
            "docs": [
              "Token Address of the vault"
            ],
            "type": "publicKey"
          },
          {
            "name": "maxTotalClaim",
            "docs": [
              "Maximum number of tokens that can ever be claimed from this [MerkleDistributor]."
            ],
            "type": "u64"
          },
          {
            "name": "maxNumNodes",
            "docs": [
              "Maximum number of nodes in [MerkleDistributor]."
            ],
            "type": "u64"
          },
          {
            "name": "totalAmountClaimed",
            "docs": [
              "Total amount of tokens that have been claimed."
            ],
            "type": "u64"
          },
          {
            "name": "totalAmountForgone",
            "docs": [
              "Total amount of tokens that have been forgone."
            ],
            "type": "u64"
          },
          {
            "name": "numNodesClaimed",
            "docs": [
              "Number of nodes that have been claimed."
            ],
            "type": "u64"
          },
          {
            "name": "startTs",
            "docs": [
              "Lockup time start (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "endTs",
            "docs": [
              "Lockup time end (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "clawbackStartTs",
            "docs": [
              "Clawback start (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "clawbackReceiver",
            "docs": [
              "Clawback receiver"
            ],
            "type": "publicKey"
          },
          {
            "name": "admin",
            "docs": [
              "Admin wallet"
            ],
            "type": "publicKey"
          },
          {
            "name": "clawedBack",
            "docs": [
              "Whether or not the distributor has been clawed back"
            ],
            "type": "bool"
          },
          {
            "name": "enableSlot",
            "docs": [
              "this merkle tree is enable from this slot"
            ],
            "type": "u64"
          },
          {
            "name": "closable",
            "docs": [
              "indicate that whether admin can close this pool, for testing purpose"
            ],
            "type": "bool"
          },
          {
            "name": "buffer0",
            "docs": [
              "Buffer 0"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "buffer1",
            "docs": [
              "Buffer 1"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "buffer2",
            "docs": [
              "Buffer 2"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "NewClaimEvent",
      "fields": [
        {
          "name": "claimant",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "timestamp",
          "type": "i64",
          "index": false
        },
        {
          "name": "amountClaimed",
          "type": "u64",
          "index": false
        },
        {
          "name": "amountForgone",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "ClaimedEvent",
      "fields": [
        {
          "name": "claimant",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "amount",
          "type": "u64",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InsufficientUnlockedTokens",
      "msg": "Insufficient unlocked tokens"
    },
    {
      "code": 6001,
      "name": "StartTooFarInFuture",
      "msg": "Deposit Start too far in future"
    },
    {
      "code": 6002,
      "name": "InvalidProof",
      "msg": "Invalid Merkle proof."
    },
    {
      "code": 6003,
      "name": "ExceededMaxClaim",
      "msg": "Exceeded maximum claim amount"
    },
    {
      "code": 6004,
      "name": "MaxNodesExceeded",
      "msg": "Exceeded maximum node count"
    },
    {
      "code": 6005,
      "name": "Unauthorized",
      "msg": "Account is not authorized to execute this instruction"
    },
    {
      "code": 6006,
      "name": "OwnerMismatch",
      "msg": "Token account owner did not match intended owner"
    },
    {
      "code": 6007,
      "name": "ClawbackDuringVesting",
      "msg": "Clawback cannot be before vesting ends"
    },
    {
      "code": 6008,
      "name": "ClawbackBeforeStart",
      "msg": "Attempted clawback before start"
    },
    {
      "code": 6009,
      "name": "ClawbackAlreadyClaimed",
      "msg": "Clawback already claimed"
    },
    {
      "code": 6010,
      "name": "InsufficientClawbackDelay",
      "msg": "Clawback start must be at least one day after vesting end"
    },
    {
      "code": 6011,
      "name": "SameClawbackReceiver",
      "msg": "New and old Clawback receivers are identical"
    },
    {
      "code": 6012,
      "name": "SameAdmin",
      "msg": "New and old admin are identical"
    },
    {
      "code": 6013,
      "name": "ClaimExpired",
      "msg": "Claim window expired"
    },
    {
      "code": 6014,
      "name": "ArithmeticError",
      "msg": "Arithmetic Error (overflow/underflow)"
    },
    {
      "code": 6015,
      "name": "StartTimestampAfterEnd",
      "msg": "Start Timestamp cannot be after end Timestamp"
    },
    {
      "code": 6016,
      "name": "TimestampsNotInFuture",
      "msg": "Timestamps cannot be in the past"
    },
    {
      "code": 6017,
      "name": "InvalidVersion",
      "msg": "Airdrop Version Mismatch"
    },
    {
      "code": 6018,
      "name": "ClaimingIsNotStarted",
      "msg": "Claiming is not started"
    },
    {
      "code": 6019,
      "name": "CannotCloseDistributor",
      "msg": "Cannot close distributor"
    },
    {
      "code": 6020,
      "name": "CannotCloseClaimStatus",
      "msg": "Cannot close claim status"
    }
  ]
};

export const IDL: MerkleDistributor = {
  "version": "0.0.1",
  "name": "merkle_distributor",
  "instructions": [
    {
      "name": "newDistributor",
      "docs": [
        "READ THE FOLLOWING:",
        "",
        "This instruction is susceptible to frontrunning that could result in loss of funds if not handled properly.",
        "",
        "An attack could look like:",
        "- A legitimate user opens a new distributor.",
        "- Someone observes the call to this instruction.",
        "- They replace the clawback_receiver, admin, or time parameters with their own.",
        "",
        "One situation that could happen here is the attacker replaces the admin and clawback_receiver with their own",
        "and sets the clawback_start_ts with the minimal time allowed. After clawback_start_ts has elapsed,",
        "the attacker can steal all funds from the distributor to their own clawback_receiver account.",
        "",
        "HOW TO AVOID:",
        "- When you call into this instruction, ensure your transaction succeeds.",
        "- To be extra safe, after your transaction succeeds, read back the state of the created MerkleDistributor account and",
        "assert the parameters are what you expect, most importantly the clawback_receiver and admin.",
        "- If your transaction fails, double check the value on-chain matches what you expect."
      ],
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "MerkleDistributor"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "Mint",
                "path": "mint"
              },
              {
                "kind": "arg",
                "type": "u64",
                "path": "version"
              }
            ]
          }
        },
        {
          "name": "clawbackReceiver",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Clawback receiver token account"
          ]
        },
        {
          "name": "mint",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The mint to distribute."
          ]
        },
        {
          "name": "tokenVault",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "Token vault",
            "Should create previously"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin wallet, responsible for creating the distributor and paying for the transaction.",
            "Also has the authority to set the clawback receiver and change itself."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        },
        {
          "name": "associatedTokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Associated Token] program."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Token] program."
          ]
        }
      ],
      "args": [
        {
          "name": "version",
          "type": "u64"
        },
        {
          "name": "root",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "maxTotalClaim",
          "type": "u64"
        },
        {
          "name": "maxNumNodes",
          "type": "u64"
        },
        {
          "name": "startVestingTs",
          "type": "i64"
        },
        {
          "name": "endVestingTs",
          "type": "i64"
        },
        {
          "name": "clawbackStartTs",
          "type": "i64"
        },
        {
          "name": "enableSlot",
          "type": "u64"
        },
        {
          "name": "closable",
          "type": "bool"
        }
      ]
    },
    {
      "name": "closeDistributor",
      "docs": [
        "only available in test phase"
      ],
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "relations": [
            "admin",
            "token_vault"
          ]
        },
        {
          "name": "tokenVault",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Clawback receiver token account"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin wallet, responsible for creating the distributor and paying for the transaction.",
            "Also has the authority to set the clawback receiver and change itself."
          ]
        },
        {
          "name": "destinationTokenAccount",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "account receive token back"
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "closeClaimStatus",
      "docs": [
        "only available in test phase"
      ],
      "accounts": [
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "relations": [
            "claimant",
            "admin"
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "admin",
          "isMut": false,
          "isSigner": true
        }
      ],
      "args": []
    },
    {
      "name": "setEnableSlot",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "[MerkleDistributor]."
          ],
          "relations": [
            "admin"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Payer to create the distributor."
          ]
        }
      ],
      "args": [
        {
          "name": "enableSlot",
          "type": "u64"
        }
      ]
    },
    {
      "name": "newClaim",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Claim status PDA"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "ClaimStatus"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "claimant"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "MerkleDistributor",
                "path": "distributor"
              }
            ]
          }
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Account to send the claimed tokens to."
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Who is claiming the tokens."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        }
      ],
      "args": [
        {
          "name": "amountUnlocked",
          "type": "u64"
        },
        {
          "name": "amountLocked",
          "type": "u64"
        },
        {
          "name": "proof",
          "type": {
            "vec": {
              "array": [
                "u8",
                32
              ]
            }
          }
        }
      ]
    },
    {
      "name": "claimLocked",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "claimStatus",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Claim Status PDA"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "type": "string",
                "value": "ClaimStatus"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "path": "claimant"
              },
              {
                "kind": "account",
                "type": "publicKey",
                "account": "MerkleDistributor",
                "path": "distributor"
              }
            ]
          }
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Account to send the claimed tokens to.",
            "Claimant must sign the transaction and can only claim on behalf of themself"
          ]
        },
        {
          "name": "claimant",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Who is claiming the tokens."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "clawback",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "from",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Distributor ATA containing the tokens to distribute."
          ]
        },
        {
          "name": "to",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The Clawback token account."
          ]
        },
        {
          "name": "claimant",
          "isMut": false,
          "isSigner": true,
          "docs": [
            "Claimant account",
            "Anyone can claw back the funds"
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "The [System] program."
          ]
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "SPL [Token] program."
          ]
        }
      ],
      "args": []
    },
    {
      "name": "setClawbackReceiver",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "newClawbackAccount",
          "isMut": false,
          "isSigner": false,
          "docs": [
            "New clawback account"
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin signer"
          ]
        }
      ],
      "args": []
    },
    {
      "name": "setAdmin",
      "accounts": [
        {
          "name": "distributor",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "The [MerkleDistributor]."
          ]
        },
        {
          "name": "admin",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "Admin signer"
          ]
        },
        {
          "name": "newAdmin",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "New admin account"
          ]
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "claimStatus",
      "docs": [
        "Holds whether or not a claimant has claimed tokens."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "claimant",
            "docs": [
              "Authority that claimed the tokens."
            ],
            "type": "publicKey"
          },
          {
            "name": "lockedAmount",
            "docs": [
              "Locked amount"
            ],
            "type": "u64"
          },
          {
            "name": "lockedAmountWithdrawn",
            "docs": [
              "Locked amount withdrawn"
            ],
            "type": "u64"
          },
          {
            "name": "unlockedAmount",
            "docs": [
              "Unlocked amount"
            ],
            "type": "u64"
          },
          {
            "name": "unlockedAmountClaimed",
            "docs": [
              "Unlocked amount claimed"
            ],
            "type": "u64"
          },
          {
            "name": "closable",
            "docs": [
              "indicate that whether admin can close this account, for testing purpose"
            ],
            "type": "bool"
          },
          {
            "name": "admin",
            "docs": [
              "admin of merkle tree, store for for testing purpose"
            ],
            "type": "publicKey"
          }
        ]
      }
    },
    {
      "name": "merkleDistributor",
      "docs": [
        "State for the account which distributes tokens."
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "bump",
            "docs": [
              "Bump seed."
            ],
            "type": "u8"
          },
          {
            "name": "version",
            "docs": [
              "Version of the airdrop"
            ],
            "type": "u64"
          },
          {
            "name": "root",
            "docs": [
              "The 256-bit merkle root."
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "mint",
            "docs": [
              "[Mint] of the token to be distributed."
            ],
            "type": "publicKey"
          },
          {
            "name": "tokenVault",
            "docs": [
              "Token Address of the vault"
            ],
            "type": "publicKey"
          },
          {
            "name": "maxTotalClaim",
            "docs": [
              "Maximum number of tokens that can ever be claimed from this [MerkleDistributor]."
            ],
            "type": "u64"
          },
          {
            "name": "maxNumNodes",
            "docs": [
              "Maximum number of nodes in [MerkleDistributor]."
            ],
            "type": "u64"
          },
          {
            "name": "totalAmountClaimed",
            "docs": [
              "Total amount of tokens that have been claimed."
            ],
            "type": "u64"
          },
          {
            "name": "totalAmountForgone",
            "docs": [
              "Total amount of tokens that have been forgone."
            ],
            "type": "u64"
          },
          {
            "name": "numNodesClaimed",
            "docs": [
              "Number of nodes that have been claimed."
            ],
            "type": "u64"
          },
          {
            "name": "startTs",
            "docs": [
              "Lockup time start (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "endTs",
            "docs": [
              "Lockup time end (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "clawbackStartTs",
            "docs": [
              "Clawback start (Unix Timestamp)"
            ],
            "type": "i64"
          },
          {
            "name": "clawbackReceiver",
            "docs": [
              "Clawback receiver"
            ],
            "type": "publicKey"
          },
          {
            "name": "admin",
            "docs": [
              "Admin wallet"
            ],
            "type": "publicKey"
          },
          {
            "name": "clawedBack",
            "docs": [
              "Whether or not the distributor has been clawed back"
            ],
            "type": "bool"
          },
          {
            "name": "enableSlot",
            "docs": [
              "this merkle tree is enable from this slot"
            ],
            "type": "u64"
          },
          {
            "name": "closable",
            "docs": [
              "indicate that whether admin can close this pool, for testing purpose"
            ],
            "type": "bool"
          },
          {
            "name": "buffer0",
            "docs": [
              "Buffer 0"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "buffer1",
            "docs": [
              "Buffer 1"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "buffer2",
            "docs": [
              "Buffer 2"
            ],
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    }
  ],
  "events": [
    {
      "name": "NewClaimEvent",
      "fields": [
        {
          "name": "claimant",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "timestamp",
          "type": "i64",
          "index": false
        },
        {
          "name": "amountClaimed",
          "type": "u64",
          "index": false
        },
        {
          "name": "amountForgone",
          "type": "u64",
          "index": false
        }
      ]
    },
    {
      "name": "ClaimedEvent",
      "fields": [
        {
          "name": "claimant",
          "type": "publicKey",
          "index": false
        },
        {
          "name": "amount",
          "type": "u64",
          "index": false
        }
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "InsufficientUnlockedTokens",
      "msg": "Insufficient unlocked tokens"
    },
    {
      "code": 6001,
      "name": "StartTooFarInFuture",
      "msg": "Deposit Start too far in future"
    },
    {
      "code": 6002,
      "name": "InvalidProof",
      "msg": "Invalid Merkle proof."
    },
    {
      "code": 6003,
      "name": "ExceededMaxClaim",
      "msg": "Exceeded maximum claim amount"
    },
    {
      "code": 6004,
      "name": "MaxNodesExceeded",
      "msg": "Exceeded maximum node count"
    },
    {
      "code": 6005,
      "name": "Unauthorized",
      "msg": "Account is not authorized to execute this instruction"
    },
    {
      "code": 6006,
      "name": "OwnerMismatch",
      "msg": "Token account owner did not match intended owner"
    },
    {
      "code": 6007,
      "name": "ClawbackDuringVesting",
      "msg": "Clawback cannot be before vesting ends"
    },
    {
      "code": 6008,
      "name": "ClawbackBeforeStart",
      "msg": "Attempted clawback before start"
    },
    {
      "code": 6009,
      "name": "ClawbackAlreadyClaimed",
      "msg": "Clawback already claimed"
    },
    {
      "code": 6010,
      "name": "InsufficientClawbackDelay",
      "msg": "Clawback start must be at least one day after vesting end"
    },
    {
      "code": 6011,
      "name": "SameClawbackReceiver",
      "msg": "New and old Clawback receivers are identical"
    },
    {
      "code": 6012,
      "name": "SameAdmin",
      "msg": "New and old admin are identical"
    },
    {
      "code": 6013,
      "name": "ClaimExpired",
      "msg": "Claim window expired"
    },
    {
      "code": 6014,
      "name": "ArithmeticError",
      "msg": "Arithmetic Error (overflow/underflow)"
    },
    {
      "code": 6015,
      "name": "StartTimestampAfterEnd",
      "msg": "Start Timestamp cannot be after end Timestamp"
    },
    {
      "code": 6016,
      "name": "TimestampsNotInFuture",
      "msg": "Timestamps cannot be in the past"
    },
    {
      "code": 6017,
      "name": "InvalidVersion",
      "msg": "Airdrop Version Mismatch"
    },
    {
      "code": 6018,
      "name": "ClaimingIsNotStarted",
      "msg": "Claiming is not started"
    },
    {
      "code": 6019,
      "name": "CannotCloseDistributor",
      "msg": "Cannot close distributor"
    },
    {
      "code": 6020,
      "name": "CannotCloseClaimStatus",
      "msg": "Cannot close claim status"
    }
  ]
};
