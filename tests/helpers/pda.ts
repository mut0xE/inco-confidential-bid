import { PublicKey } from "@solana/web3.js";
import { INCO_LIGHTNING_PROGRAM_ID } from "../confidential-bid";
import { Program } from "@coral-xyz/anchor";

// Helper: derive allowance PDA from handle
export function getAllowancePda(
  handle: bigint,
  allowedAddress: PublicKey
): [PublicKey, number] {
  const handleBuffer = Buffer.alloc(16);
  let h = handle;
  for (let i = 0; i < 16; i++) {
    handleBuffer[i] = Number(h & BigInt(0xff));
    h = h >> BigInt(8);
  }
  return PublicKey.findProgramAddressSync(
    [handleBuffer, allowedAddress.toBuffer()],
    INCO_LIGHTNING_PROGRAM_ID
  );
}

export function extractHandleFromAnchor(anchorHandle: any): bigint {
  if (anchorHandle && anchorHandle._bn) {
    return BigInt(anchorHandle._bn.toString(10));
  }
  if (typeof anchorHandle === "object" && anchorHandle["0"]) {
    const nested = anchorHandle["0"];
    if (nested && nested._bn) return BigInt(nested._bn.toString(10));
    if (nested && nested.toString && nested.constructor?.name === "BN") {
      return BigInt(nested.toString(10));
    }
  }
  if (anchorHandle instanceof Uint8Array || Array.isArray(anchorHandle)) {
    const buffer = Buffer.from(anchorHandle);
    let result = BigInt(0);
    for (let i = buffer.length - 1; i >= 0; i--) {
      result = result * BigInt(256) + BigInt(buffer[i]);
    }
    return result;
  }
  if (typeof anchorHandle === "number" || typeof anchorHandle === "bigint") {
    return BigInt(anchorHandle);
  }
  return BigInt(0);
}

export function formatBalance(plaintext: string): string {
  return (Number(plaintext) / 1e6).toFixed(6);
}

export function getIncoAta(
  program: Program,
  wallet: PublicKey,
  mint: PublicKey
): PublicKey {
  const [ata] = PublicKey.findProgramAddressSync(
    [wallet.toBuffer(), program.programId.toBuffer(), mint.toBuffer()],
    program.programId
  );
  return ata;
}
