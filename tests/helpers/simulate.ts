import { Program } from "@coral-xyz/anchor";
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

export async function simulateAuctionHandles(
  connection: Connection,
  tx: Transaction,
  auctionPubkey: PublicKey,
  signer: Keypair,
  program: Program<any>
) {
  const { blockhash } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.feePayer = signer.publicKey;
  tx.sign(signer);

  const sim = await connection.simulateTransaction(tx, undefined, [
    auctionPubkey,
  ]);
  if (sim.value.err || !sim.value.accounts?.[0]?.data) {
    throw new Error(`Simulation failed: ${JSON.stringify(sim.value.err)}`);
  }

  const raw = Buffer.from(sim.value.accounts[0].data[0], "base64");
  const auction: any = program.coder.accounts.decode("auctionState", raw);
  return {
    highestBid: BigInt(auction.highestBid.toString()),
    reserveMetHandle: BigInt(auction.reserveMetHandle.toString()),
    highestTimestamp: BigInt(auction.highestTimestamp.toString()),
    secondHighestBid: auction.secondHighestBid
      ? BigInt(auction.secondHighestBid.toString())
      : null,
  };
}
export async function simulateCheckWinnerHandles(
  connection: Connection,
  tx: Transaction,
  bidPda: PublicKey,
  signer: Keypair,
  program: Program<any>
) {
  const { blockhash } = await connection.getLatestBlockhash();
  tx.recentBlockhash = blockhash;
  tx.feePayer = signer.publicKey;
  tx.sign(signer);

  const sim = await connection.simulateTransaction(tx, undefined, [bidPda]);

  if (sim.value.err) {
    throw new Error(
      `Simulation failed: ${JSON.stringify(
        sim.value.err
      )}\nLogs:\n${sim.value.logs?.join("\n")}`
    );
  }

  if (!sim.value.accounts?.[0]?.data) {
    throw new Error("No account data returned in simulation");
  }

  const raw = Buffer.from(sim.value.accounts[0].data[0], "base64");
  const bidAcc: any = program.coder.accounts.decode("bid", raw);

  return {
    isWinner: BigInt(bidAcc.isWinnerHandle.toString()),
  };
}
