import { ComputeBudgetProgram, Connection, PublicKey, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import MerkleDistributorAPI, { EligibilityResp } from "../src/index";
import { Wallet } from "@coral-xyz/anchor";
import { loadKeypair } from "./utils";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

const distributorProgramId = new PublicKey("E7HtfkEMhmn9uwL7EFNydcXBWy5WCYN1vFmKKjipEH1x");
const userWithoutClaim = new PublicKey("2Xz15NfayPmLmhjVuXTzXSF2GgDirJ7jrwFonHFTUTPj");
const rpc = "https://api.devnet.solana.com";

const claimantKeypairPath = "/path/to/claimant/keypair.json";

const distributorApiUrl = "http://127.0.0.1:7001";

async function main() {

    const claimantWallet = new Wallet(loadKeypair(claimantKeypairPath));

    // user with airdrop
    const eligibility = (await MerkleDistributorAPI.getEligibility(distributorApiUrl, claimantWallet.publicKey) as EligibilityResp);
    console.log('');
    console.log("Eligibility with claim:", eligibility);
    console.log(`Amount claimable now: ${MerkleDistributorAPI.calculateClaimableAmount(eligibility)}`);

    // user with no airdrop
    const eligibilityWithoutClaim = await MerkleDistributorAPI.getEligibility(distributorApiUrl, userWithoutClaim);
    console.log('');
    console.log("Eligibility without claim:", eligibilityWithoutClaim);

    // can pass in an (AnchorProvider) or (Connection + Wallet)
    console.log('');
    console.log("Claiming for user:", eligibility.claimant);
    const connection = new Connection(rpc);

    // Get the claim ixs first
    const claimIxs = await MerkleDistributorAPI.getNewClaimIxs({
        connection,
        distributorProgramId,
        claimantWallet,
        userEligibility: eligibility,
    });
    // then add locked amount for user
    const claimLockedIxs = await MerkleDistributorAPI.getClaimLockedIxs({
        connection,
        distributorProgramId,
        claimantWallet,
        userEligibility: eligibility,
    });

    // build tx to send
    const ixs = [
        ComputeBudgetProgram.setComputeUnitLimit({
            units: 200000,
        }),
        ComputeBudgetProgram.setComputeUnitPrice({
            microLamports: 100,
        }),
        ...claimIxs,
        ...claimLockedIxs,
    ];

    const message = new TransactionMessage({
        payerKey: claimantWallet.publicKey,
        recentBlockhash: (await connection.getLatestBlockhash('finalized')).blockhash,
        instructions: ixs,
    }).compileToV0Message();

    const tx = await claimantWallet.signTransaction(
        new VersionedTransaction(message)
    );

    console.log(`Sending claim tx: ${bs58.encode(tx.signatures[0])}`);
    const txid = await connection.sendTransaction(tx);
    return txid;
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
