import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ConfidentialBid } from "../target/types/confidential_bid";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createAssociatedTokenAccount,
  createMint,
  getAccount,
  getAssociatedTokenAddressSync,
  mintTo,
} from "@solana/spl-token";
import incoIdl from "../tests/idl/inco_token.json";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  extractHandleFromAnchor,
  formatBalance,
  getAllowancePda,
  getIncoAta,
} from "./helpers/pda";
import { encryptValue, hexToBuffer } from "@inco/solana-sdk";
import { simulateAndGetHandle } from "./helpers/simulate";
import { decryptHandleWithSigner } from "./helpers/decrypt";
import { expect } from "chai";
import fs from "fs";
const SYSTEM_PROGRAM_ID = SystemProgram.programId;

export const INCO_LIGHTNING_PROGRAM_ID = new PublicKey(
  "5sjEbPiqgZrYwR31ahR6Uk9wf5awoX61YGg7jExQSwaj"
);
const INPUT_TYPE = 0;
const DECIMALS = 6;
const TOKEN_MULTIPLIER = BigInt(1_000_000);

const logTransactionResult = (label: string, txSignature: string) => {
  console.log(`\n${label}:`);
  console.log(`   Txn signature: ${txSignature}`);
};

describe("confidential-bid", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const organizer = provider.wallet as anchor.Wallet;

  let auctionMint: anchor.web3.PublicKey; // NFT being auctioned
  let bidTokenMint = Keypair.generate(); // Inco token used for bidding
  let organizerAta: anchor.web3.PublicKey; // Organizer's NFT account
  let bidderAta: anchor.web3.PublicKey; // Bidder's token account
  let bidVault: anchor.web3.PublicKey; // Bid Vault token account
  let bidder1 = Keypair.generate(); // Bidder

  let auctionPda: anchor.web3.PublicKey;
  let vault: anchor.web3.PublicKey;

  const program = anchor.workspace.confidentialBid as Program<ConfidentialBid>;
  const incoTokenProgram = new anchor.Program(incoIdl as anchor.Idl, provider);

  const auctionId = new anchor.BN(Math.floor(Math.random() * 1000));

  // Creates a confidential mint using Inco token program
  async function createConfidentialMint(payer: Keypair, mintKeypair: Keypair) {
    // Build the instruction using the IDL
    const tx = await incoTokenProgram.methods
      .initializeMint(DECIMALS, payer.publicKey, payer.publicKey)
      .accounts({
        mint: mintKeypair.publicKey,
        payer: payer.publicKey,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        systemProgram: anchor.web3.SystemProgram.programId,
      } as any)
      .signers([mintKeypair])
      .rpc();
    logTransactionResult("Confidential mint created", tx);
    return mintKeypair;
  }

  // Creates a token account for Inco confidential tokens
  async function createTokenAccount(
    payer: Keypair,
    owner: PublicKey,
    mint: PublicKey
  ) {
    // Derive the associated token account for confidential tokens
    const ata = getIncoAta(incoTokenProgram, owner, mint);
    const tx = await incoTokenProgram.methods
      .createIdempotent()
      .accounts({
        payer: payer.publicKey,
        associatedToken: ata,
        wallet: owner,
        mint,
        systemProgram: anchor.web3.SystemProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .signers([payer])
      .rpc();
    logTransactionResult("Confidential Token Account created", tx);
    await new Promise((resolve) => setTimeout(resolve, 5000));
    // Mint tokens to the newly created account
    await mintTokens(mint, ata, payer.publicKey, payer);
    return ata;
  }

  // Mints confidential tokens to an account via Inco token program
  async function mintTokens(
    mint: PublicKey,
    ata: PublicKey,
    mintAuthority: PublicKey,
    payer: Keypair
  ) {
    const mintAmount = BigInt(100) * TOKEN_MULTIPLIER;

    console.log(
      `\nMinting ${mintAmount / TOKEN_MULTIPLIER} tokens to ${ata.toBase58()}`
    );

    const encryptedHex = await encryptValue(mintAmount);

    const txForSim = await incoTokenProgram.methods
      .mintTo(hexToBuffer(encryptedHex), INPUT_TYPE)
      .accounts({
        mint,
        account: ata,
        mintAuthority,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .transaction();

    await new Promise((resolve) => setTimeout(resolve, 3000));

    const newHandle = await simulateAndGetHandle(
      provider.connection,
      txForSim,
      ata,
      payer
    );

    if (!newHandle) {
      throw new Error("Failed to extract handle from simulation");
    }
    const [allowancePda] = getAllowancePda(newHandle!, bidder1.publicKey);

    const tx = await incoTokenProgram.methods
      .mintTo(hexToBuffer(encryptedHex), INPUT_TYPE)
      .accounts({
        mint,
        account: ata,
        mintAuthority,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      } as any)
      .remainingAccounts([
        { pubkey: allowancePda, isSigner: false, isWritable: true },
        { pubkey: bidder1.publicKey, isSigner: false, isWritable: false },
      ])
      .rpc();
    logTransactionResult("Mint tx", tx);
    await new Promise((r) => setTimeout(r, 3000));

    const account = await (incoTokenProgram.account as any).incoAccount.fetch(
      ata
    );
    const handle = extractHandleFromAnchor(account.amount);
    const result = await decryptHandleWithSigner(handle.toString(), bidder1);

    console.log(
      `Balance :`,
      result.success
        ? `${formatBalance(result.plaintext!)} tokens`
        : result.error
    );
    if (result.success) {
      expect(BigInt(result.plaintext!)).to.equal(mintAmount);
    }
  }

  before(async () => {
    // Create NFT mint (decimals=0)
    auctionMint = await createMint(
      provider.connection,
      organizer.payer,
      organizer.publicKey,
      null,
      0 // NFT decimals
    );

    // Create organizer's ATA
    organizerAta = await createAssociatedTokenAccount(
      provider.connection,
      organizer.payer,
      auctionMint,
      organizer.publicKey
    );

    // Mint NFT to organizer
    await mintTo(
      provider.connection,
      organizer.payer,
      auctionMint,
      organizerAta,
      organizer.publicKey,
      2
    );

    // Derive PDAs
    auctionPda = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("auction"),
        organizer.publicKey.toBuffer(),
        auctionId.toBuffer("le", 8),
      ],
      program.programId
    )[0];

    // Derive vault ATA to hold the NFT during auction
    vault = getAssociatedTokenAddressSync(auctionMint, auctionPda, true);

    // Create confidential bid token mint (Inco)
    bidTokenMint = await createConfidentialMint(organizer.payer, bidTokenMint);
    const bidmint = await (incoTokenProgram.account as any).incoMint.fetch(
      bidTokenMint.publicKey
    );

    // Create bidder's confidential token account and mint tokens
    bidderAta = await createTokenAccount(
      organizer.payer,
      bidder1.publicKey,
      bidTokenMint.publicKey
    );
    bidVault = getIncoAta(incoTokenProgram, auctionPda, bidTokenMint.publicKey);
  });

  it("should create first price auction successfully", async () => {
    const currentTime = Math.floor(Date.now() / 1000);
    const startTime = new anchor.BN(currentTime + 3);
    const endTime = new anchor.BN(currentTime + 3600); // 1 hour later
    const reservePrice = new anchor.BN(BigInt(10) * TOKEN_MULTIPLIER);
    const tokenAmount = new anchor.BN(1);

    const tx = await program.methods
      .createAuction(
        auctionId,
        startTime,
        endTime,
        reservePrice,
        { normal: {} }, // First-price auction type
        tokenAmount
      )
      .accounts({
        organizer: organizer.publicKey,
        mint: auctionMint, // NFT being auctioned
        bidTokenMint: bidTokenMint.publicKey,
        bidVault,
        vault,
        auction: auctionPda,
        organizerTokenAccount: organizerAta,
        systemProgram: SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .rpc();
    logTransactionResult("Auction created:", tx);

    // VERIFY AUCTION STATE
    const auctionStateAccount = await program.account.auctionState.fetch(
      auctionPda
    );
    expect(auctionStateAccount.organizer.toBase58()).to.equal(
      organizer.publicKey.toBase58()
    );
    expect(auctionStateAccount.auctionId.toNumber()).to.equal(
      auctionId.toNumber()
    );
    expect(auctionStateAccount.mint.toBase58()).to.equal(
      auctionMint.toBase58()
    );
    expect(auctionStateAccount.bidTokenMint.toBase58()).to.equal(
      bidTokenMint.publicKey.toBase58()
    );
    expect(auctionStateAccount.startTime.toNumber()).to.equal(
      startTime.toNumber()
    );
    expect(auctionStateAccount.endTime.toNumber()).to.equal(endTime.toNumber());
    expect(auctionStateAccount.reservePrice.toNumber()).to.equal(
      reservePrice.toNumber()
    );
    expect(auctionStateAccount.bidCount).to.equal(0);
    expect(auctionStateAccount.highestBid.toNumber()).to.equal(0);
    expect(auctionStateAccount.auctionStatus).to.deep.include({ open: {} });
    expect(auctionStateAccount.auctionType).to.deep.include({ normal: {} });
    expect(auctionStateAccount.bidVault.toBase58()).to.equal(
      bidVault.toBase58()
    );

    // Verify NFT transferred to vault
    const vaultAccount = await getAccount(provider.connection, vault);
    expect(vaultAccount.amount).to.equal(BigInt(1));

    console.log("First Price auction created successfully!\n");
  });

  it("should create vickrey (second-price) auction successfully", async () => {
    const currentTime = Math.floor(Date.now() / 1000);
    const startTime = new anchor.BN(currentTime + 60);
    const endTime = new anchor.BN(currentTime + 60 + 300); // 5 minutes from start
    const reservePrice = new anchor.BN(BigInt(10) * TOKEN_MULTIPLIER); // 10 cUSDC
    const tokenAmount = new anchor.BN(1);

    // Generate new auction ID for Vickrey auction
    const vickreyAuctionId = new anchor.BN(Math.floor(Math.random() * 1000));

    // Derive Vickrey auction PDA
    const vickreyAuctionPda = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("auction"),
        organizer.publicKey.toBuffer(),
        vickreyAuctionId.toBuffer("le", 8),
      ],
      program.programId
    )[0];

    // Derive Vickrey vault ATA
    const vickreyVault = getAssociatedTokenAddressSync(
      auctionMint,
      vickreyAuctionPda,
      true
    );
    bidVault = getIncoAta(
      incoTokenProgram,
      vickreyAuctionPda,
      bidTokenMint.publicKey
    );
    const tx = await program.methods
      .createAuction(
        vickreyAuctionId,
        startTime,
        endTime,
        reservePrice,
        { vickrey: {} }, // Second-price auction type
        tokenAmount
      )
      .accounts({
        organizer: organizer.publicKey,
        mint: auctionMint, // NFT being auctioned
        bidTokenMint: bidTokenMint.publicKey,
        bidVault,
        vault: vickreyVault,
        auction: vickreyAuctionPda,
        organizerTokenAccount: organizerAta,
        systemProgram: SYSTEM_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .rpc();
    logTransactionResult("Vickrey Auction created:", tx);

    // VERIFY VICKREY AUCTION STATE
    const vickreyAuctionStateAccount = await program.account.auctionState.fetch(
      vickreyAuctionPda
    );

    // Verify organizer
    expect(vickreyAuctionStateAccount.organizer.toBase58()).to.equal(
      organizer.publicKey.toBase58()
    );

    // Verify auction ID
    expect(vickreyAuctionStateAccount.auctionId.toNumber()).to.equal(
      vickreyAuctionId.toNumber()
    );

    // Verify NFT mint
    expect(vickreyAuctionStateAccount.mint.toBase58()).to.equal(
      auctionMint.toBase58()
    );

    // Verify bid token mint
    expect(vickreyAuctionStateAccount.bidTokenMint.toBase58()).to.equal(
      bidTokenMint.publicKey.toBase58()
    );

    // Verify start time
    expect(vickreyAuctionStateAccount.startTime.toNumber()).to.equal(
      startTime.toNumber()
    );

    // Verify end time
    expect(vickreyAuctionStateAccount.endTime.toNumber()).to.equal(
      endTime.toNumber()
    );

    // Verify reserve price
    expect(vickreyAuctionStateAccount.reservePrice.toNumber()).to.equal(
      reservePrice.toNumber()
    );

    // Verify bid count is 0
    expect(vickreyAuctionStateAccount.bidCount).to.equal(0);

    // Verify highest bid is 0
    expect(vickreyAuctionStateAccount.highestBid.toNumber()).to.equal(0);

    // Verify second highest bid is None
    expect(vickreyAuctionStateAccount.secondHighestBid).to.equal(null);

    // Verify auction status is Open
    expect(vickreyAuctionStateAccount.auctionStatus).to.deep.include({
      open: {},
    });

    // Verify auction type is Vickrey (second-price)
    expect(vickreyAuctionStateAccount.auctionType).to.deep.include({
      vickrey: {},
    });

    expect(vickreyAuctionStateAccount.bidVault.toBase58()).to.equal(
      bidVault.toBase58()
    );
    // Verify NFT transferred to vault
    const vickreyVaultAccount = await getAccount(
      provider.connection,
      vickreyVault
    );
    expect(vickreyVaultAccount.amount).to.equal(BigInt(1));

    console.log("\nVickrey auction created successfully!\n");
  });

  it("should place bid successfully", async () => {
    const bidAmount = BigInt(50) * TOKEN_MULTIPLIER;
    const encryptedBid = await encryptValue(bidAmount);
    const bidBuffer = hexToBuffer(encryptedBid);

    const [bidPda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("bid"), auctionPda.toBuffer(), bidder1.publicKey.toBuffer()],
      program.programId
    );
    bidVault = getIncoAta(incoTokenProgram, auctionPda, bidTokenMint.publicKey);

    await new Promise((r) => setTimeout(r, 4000));
    const txSim = await program.methods
      .placeBid(bidBuffer, 0)
      .accounts({
        bidder: bidder1.publicKey,
        organizer: organizer.publicKey,
        bidderTokenAta: bidderAta,
        bidMint: bidTokenMint.publicKey,
        bidVault,
        bid: bidPda,
        auction: auctionPda,
        systemProgram: SYSTEM_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .transaction();

    const bidderHandle = await simulateAndGetHandle(
      provider.connection,
      txSim,
      bidPda,
      bidder1
    );
    if (!bidderHandle)
      throw new Error("Failed to get bidder Handle  from simulation");

    const bidderAllowance = getAllowancePda(bidderHandle, bidder1.publicKey);
    console.log("bidder allowance:", bidderAllowance);

    const bidHandle = await simulateAndGetHandle(
      provider.connection,
      txSim,
      bidVault,
      organizer.payer
    );
    const bidAllowance = getAllowancePda(bidHandle, organizer.publicKey);
    console.log("bid allowance:", bidAllowance);

    const tx = await program.methods
      .placeBid(bidBuffer, 0)
      .accounts({
        bidder: bidder1.publicKey,
        organizer: organizer.publicKey,
        bidderTokenAta: bidderAta,
        bidMint: bidTokenMint.publicKey,
        bidVault,
        bid: bidPda,
        auction: auctionPda,
        systemProgram: SYSTEM_PROGRAM_ID,
        incoTokenProgram: incoTokenProgram.programId,
        incoLightningProgram: INCO_LIGHTNING_PROGRAM_ID,
      })
      .remainingAccounts([
        {
          pubkey: bidderAllowance[0],
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: bidder1.publicKey,
          isSigner: false,
          isWritable: false,
        },
        {
          pubkey: bidAllowance[0],
          isSigner: false,
          isWritable: true,
        },
        {
          pubkey: organizer.publicKey,
          isSigner: false,
          isWritable: false,
        },
      ])
      .signers([bidder1])
      .rpc();

    logTransactionResult("Bid placed", tx);

    const auctionState = await program.account.auctionState.fetch(auctionPda);
    expect(auctionState.bidCount).to.equal(1);
    console.log(auctionState);

    const bidState = await program.account.bid.fetch(bidPda);
    console.log(bidState);

    const bidVaultAccount = await (
      incoTokenProgram.account as any
    ).incoAccount.fetch(bidVault);
    console.log(bidVaultAccount);

    const account = await (incoTokenProgram.account as any).incoAccount.fetch(
      bidderAta
    );
    const handle1 = extractHandleFromAnchor(account.amount);
    const result1 = await decryptHandleWithSigner(
      handle1.toString(),
      organizer.payer
    );

    console.log(
      `Balance :`,
      result1.success
        ? `${formatBalance(result1.plaintext!)} tokens`
        : result1.error
    );
    const handle = extractHandleFromAnchor(account.amount);
    const result = await decryptHandleWithSigner(handle.toString(), bidder1);

    console.log(
      `Balance :`,
      result.success
        ? `${formatBalance(result.plaintext!)} tokens`
        : result.error
    );
  });
});
