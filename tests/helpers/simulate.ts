import { Connection, Keypair, PublicKey, Transaction } from "@solana/web3.js";

// Helper: simulate transaction and extract handle from account data
export async function simulateAndGetHandle(
  connection: Connection,
  tx: Transaction,
  accountPubkey: PublicKey,
  walletKeypair: Keypair
): Promise<bigint | null> {
  const { blockhash } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.feePayer = walletKeypair.publicKey;
  tx.sign(walletKeypair);

  const simulation = await connection.simulateTransaction(tx, undefined, [
    accountPubkey,
  ]);

  if (simulation.value.err) {
    console.log("Simulation error:", simulation.value.err);
    return null;
  }

  if (simulation.value.accounts?.[0]?.data) {
    const data = Buffer.from(simulation.value.accounts[0].data[0], "base64");
    // Extract handle from account data (offset depends on your account struct)
    const amountBytes = data.slice(72, 88); // Adjust offset as needed
    let handle = BigInt(0);
    for (let i = 15; i >= 0; i--) {
      handle = handle * BigInt(256) + BigInt(amountBytes[i]);
    }
    return handle;
  }
  return null;
}
