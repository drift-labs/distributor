import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import { Connection, Keypair, PublicKey, TransactionInstruction, TransactionMessage, VersionedTransaction } from "@solana/web3.js";
import fs from 'fs';

export function loadKeypair(privateKey: string): Keypair {
    // try to load privateKey as a filepath
    let loadedKey: Uint8Array;
    if (fs.existsSync(privateKey)) {
        console.log(`loading private key from ${privateKey}`);
        privateKey = fs.readFileSync(privateKey).toString();
    }

    if (privateKey.includes('[') && privateKey.includes(']')) {
        console.log(`Trying to load private key as numbers array`);
        loadedKey = Uint8Array.from(JSON.parse(privateKey));
    } else if (privateKey.includes(',')) {
        console.log(`Trying to load private key as comma separated numbers`);
        loadedKey = Uint8Array.from(
            privateKey.split(',').map((val) => Number(val))
        );
    } else {
        console.log(`Trying to load private key as base58 string`);
        privateKey = privateKey.replace(/\s/g, '');
        loadedKey = new Uint8Array(bs58.decode(privateKey));
    }

    return Keypair.fromSecretKey(Uint8Array.from(loadedKey));
}

export async function buildVersionedTransaction(connection: Connection, ixs: Array<TransactionInstruction>, payerKey: PublicKey): Promise<VersionedTransaction> {
    const message = new TransactionMessage({
        payerKey,
        recentBlockhash: (await connection.getLatestBlockhash('finalized')).blockhash,
        instructions: ixs,
    }).compileToV0Message();

    return new VersionedTransaction(message);
}