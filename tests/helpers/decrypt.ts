import decrypt from "@inco/solana-sdk/attested-decrypt";
import { Keypair } from "@solana/web3.js";
import nacl from "tweetnacl";

export async function decryptHandleWithSigner(handle: string, signer: Keypair) {
  try {
    const res = await decrypt([handle], {
      address: signer.publicKey,
      signMessage: async (msg: Uint8Array) =>
        nacl.sign.detached(msg, signer.secretKey),
    });

    return { success: true, plaintext: res.plaintexts[0] };
  } catch (e: any) {
    const msg = e.message.toLowerCase();
    if (msg.includes("not allowed"))
      return { success: false, error: "not_allowed" };
    if (msg.includes("ciphertext"))
      return { success: false, error: "ciphertext_not_found" };
    return { success: false, error: msg };
  }
}
