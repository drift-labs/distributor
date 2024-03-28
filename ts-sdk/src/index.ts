import { AnchorProvider, BN, Program, Wallet } from '@coral-xyz/anchor';
import { ASSOCIATED_TOKEN_PROGRAM_ID, Token, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { Connection, PublicKey, SystemProgram, TransactionInstruction } from '@solana/web3.js';
import { MerkleDistributor, IDL } from '../../target/types/merkle_distributor';

export interface UserProof {
  merkleTree: string;
  amount: number;
  proof: number[][];
}

export interface ClaimStatusResp {
  claimant: PublicKey;
  lockedAmount: number;
  lockedAmountWithdrawn: number;
  unlockedAmount: number;
  unlockedAmountClaimed: number;
  closable: boolean;
  distributor: PublicKey;
}

export interface EligibilityResp {
  claimant: string;
  merkle_tree: string;
  mint: string;
  start_ts: number;
  end_ts: number;
  proof: number[][];
  start_amount: number;
  end_amount: number;
  claimed_amount: number;
}

export interface UserNotFoundResp {
  error: string;
}

export interface MerkleDistributorResp {
  pubkey: string;
  version: number;
  mint: string;
  tokenVault: string;
  maxTotalClaim: number;
  maxNumNodes: number;
  totalAmountClaimed: number;
  totalAmountForgone: number;
  numNodesClaimed: number;
  startTs: number;
  endTs: number;
  clawbackStartTs: number;
  clawbackReceiver: string;
  admin: string;
  clawedBack: boolean;
  enableSlot: number;
  closable: boolean;
}

export const getOrCreateATAInstruction = async (
  tokenMint: PublicKey,
  owner: PublicKey,
  connection: Connection,
  allowOwnerOffCurve = true,
  payer = owner,
): Promise<[PublicKey, TransactionInstruction?]> => {
  let toAccount;
  try {
    toAccount = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      tokenMint,
      owner,
      allowOwnerOffCurve,
    );
    const account = await connection.getAccountInfo(toAccount);
    if (!account) {
      const ix = Token.createAssociatedTokenAccountInstruction(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        tokenMint,
        toAccount,
        owner,
        payer,
      );
      return [toAccount, ix];
    }
    return [toAccount, undefined];
  } catch (e) {
    /* handle error */
    console.error('Error::getOrCreateATAInstruction', e);
    throw e;
  }
};

export interface ClaimIxConfig {
  connection?: Connection;
  claimantWallet?: Wallet;
  provider?: AnchorProvider;

  distributorProgramId: PublicKey;
  userEligibility: EligibilityResp;
}

export default class MerkleDistributorAPI {
  static async getUserProof(baseUrl: string, userPubkey: PublicKey): Promise<UserProof> {
    const url = `${baseUrl}/user/${userPubkey.toBase58()}`;
    const response = await fetch(url);
    return (await response.json()) as UserProof;
  }

  static async getClaimStatus(baseUrl: string, userPubkey: PublicKey): Promise<ClaimStatusResp> {
    const url = `${baseUrl}/claim/${userPubkey.toBase58()}`;
    const response = await fetch(url);
    return (await response.json()) as ClaimStatusResp;
  }

  static async getEligibility(baseUrl: string, userPubkey: PublicKey): Promise<EligibilityResp | UserNotFoundResp> {
    const url = `${baseUrl}/eligibility/${userPubkey.toBase58()}`;
    const response = await fetch(url);
    if (response.status === 200) {
      return (await response.json()) as EligibilityResp;
    } else if (response.status === 404) {
      return (await response.json()) as UserNotFoundResp;
    } else {
      return await response.json();
    }
  }

  static async getDistributors(baseUrl: string): Promise<MerkleDistributorResp[]> {
    const url = `${baseUrl}/distributors`;
    const response = await fetch(url);
    return (await response.json()) as MerkleDistributorResp[];
  }

  /**
   * Calculate the amount claimable for a user based on their eligibility and the current time.
   * @param u The user's eligibility data
   * @param nowTs The current time in seconds
   * @returns The amount claimable for the user in raw values without decimals applied
   */
  static calculateClaimableAmount(u: EligibilityResp, nowTs = Date.now() / 1000): number {
    if (nowTs < u.start_ts) {
      return 0;
    }
    if (nowTs > u.end_ts) {
      return 0;
    }
    return Math.floor(((u.end_amount - u.start_amount) * (nowTs - u.start_ts)) / (u.end_ts - u.start_ts) + u.start_amount);
  }

  static deriveClaimStatus(claimant: PublicKey, distributor: PublicKey, programId: PublicKey) {
    return PublicKey.findProgramAddressSync(
      [Buffer.from('ClaimStatus'), claimant.toBytes(), distributor.toBytes()],
      programId,
    );
  }

  static async getNewClaimIxs(config: ClaimIxConfig): Promise<TransactionInstruction[]> {
    let provider = config.provider;
    if (!provider && config.connection && config.claimantWallet) {
      provider = new AnchorProvider(config.connection, config.claimantWallet, {});
    } else if (!provider) {
      throw new Error('Must provide either an AnchorProvider or Connection and Wallet');
    }

    const program = new Program<MerkleDistributor>(IDL, config.distributorProgramId, provider);

    const user = config.userEligibility;
    const claimant = new PublicKey(user.claimant);
    const distributor = new PublicKey(user.merkle_tree);
    const mint = new PublicKey(user.mint);

    const [claimStatusPubKey, _] = MerkleDistributorAPI.deriveClaimStatus(
      claimant,
      distributor,
      config.distributorProgramId,
    );

    const ixs: TransactionInstruction[] = [];

    const [toATA, toATAIx] = await getOrCreateATAInstruction(mint, claimant, provider.connection, true, claimant);
    toATAIx && ixs.push(toATAIx);

    const [mdATA, mdATAIx] = await getOrCreateATAInstruction(mint, distributor, provider.connection, true, claimant);
    mdATAIx && ixs.push(mdATAIx);

    return [
      ...ixs,
      await program.methods
        .newClaim(new BN(user.end_amount), new BN(0), user.proof as any)
        .accounts({
          claimant,
          claimStatus: claimStatusPubKey,
          distributor,
          from: mdATA,
          to: toATA,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .instruction(),
    ];
  }
}
