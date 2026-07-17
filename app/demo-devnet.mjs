// Live devnet demo: drive the deployed native-SOL program with raw web3.js
// (no IDL needed). Creates a market and places a real 0.1 SOL bet, so an
// actual on-chain escrow transaction shows up in Solana Explorer.
//
//   node app/demo-devnet.mjs
import {
  Connection, Keypair, PublicKey, SystemProgram,
  Transaction, TransactionInstruction, sendAndConfirmTransaction, LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import os from "node:os";

const PROGRAM_ID = new PublicKey("iZvZwSKPvRZpEqxyXSiRGos9pGuuzygmKdcAB6biffQ");
const RPC = "https://api.devnet.solana.com";
const conn = new Connection(RPC, "confirmed");
const wallet = Keypair.fromSecretKey(
  new Uint8Array(JSON.parse(readFileSync(os.homedir() + "/.config/solana/id.json", "utf8"))),
);

// ---- Anchor instruction discriminator + LE encoders ----
const disc = (name) => createHash("sha256").update("global:" + name).digest().subarray(0, 8);
const enc = (s) => Buffer.from(s, "utf8");
const u8 = (n) => Buffer.from([n & 0xff]);
const u32 = (n) => { const b = Buffer.alloc(4); b.writeUInt32LE(n >>> 0); return b; };
const i32 = (n) => { const b = Buffer.alloc(4); b.writeInt32LE(n); return b; };
const i64 = (n) => { const b = Buffer.alloc(8); b.writeBigInt64LE(BigInt(n)); return b; };
const u64 = (n) => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const pda = (seeds) => PublicKey.findProgramAddressSync(seeds, PROGRAM_ID)[0];
const ak = (pubkey, isSigner, isWritable) => ({ pubkey, isSigner, isWritable });
const SYS = SystemProgram.programId;
const link = (sig) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;

// ---- market params (fresh window each run so the market PDA is unique) ----
const fixtureId = 778899, statKey = 1, period = 0, threshold = 1, comparison = 0; // "> 1 goal?"
const now = Math.floor(Date.now() / 1000);
const windowStart = now - 30, windowEnd = now + 3600; // open right now

const rewardPool = pda([enc("reward_pool")]);
const rewardVault = pda([enc("reward_vault")]);
const market = pda([enc("market"), i64(fixtureId), u32(statKey), i64(windowStart)]);
const vault = pda([enc("vault"), market.toBuffer()]);
const position = pda([enc("position"), market.toBuffer(), wallet.publicKey.toBuffer()]);

const sol = (lamports) => (lamports / LAMPORTS_PER_SOL).toFixed(4);

async function main() {
  console.log("Program :", PROGRAM_ID.toBase58());
  console.log("Wallet  :", wallet.publicKey.toBase58(), "(" + sol(await conn.getBalance(wallet.publicKey)) + " SOL)");
  console.log("Market  :", market.toBase58());
  console.log("Vault   :", vault.toBase58(), "\n");

  // 1) reward pool (global, once) + market — one tx
  const setup = new Transaction();
  const poolInfo = await conn.getAccountInfo(rewardPool);
  if (!poolInfo) {
    setup.add(new TransactionInstruction({
      programId: PROGRAM_ID,
      keys: [ak(rewardPool, false, true), ak(wallet.publicKey, true, true), ak(SYS, false, false)],
      data: disc("initialize_reward_pool"),
    }));
    console.log("• reward pool: creating");
  } else {
    console.log("• reward pool: already exists, skipping");
  }
  setup.add(new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [ak(market, false, true), ak(wallet.publicKey, true, true), ak(SYS, false, false)],
    data: Buffer.concat([disc("initialize_market"), i64(fixtureId), u32(statKey), i32(period), i32(threshold), u8(comparison), i64(windowStart), i64(windowEnd)]),
  }));
  const sig1 = await sendAndConfirmTransaction(conn, setup, [wallet], { commitment: "confirmed" });
  console.log("✓ market initialized  →", link(sig1), "\n");

  // 2) place a real 0.1 SOL bet on YES
  const stake = 0.1 * LAMPORTS_PER_SOL;
  const bet = new Transaction().add(new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      ak(market, false, true), ak(position, false, true), ak(vault, false, true),
      ak(rewardPool, false, true), ak(rewardVault, false, true),
      ak(wallet.publicKey, true, true), ak(SYS, false, false),
    ],
    data: Buffer.concat([disc("place_bet"), u8(1) /* YES */, u64(stake), u32(5000)]),
  }));
  const sig2 = await sendAndConfirmTransaction(conn, bet, [wallet], { commitment: "confirmed" });
  console.log("✓ placed 0.1 SOL on YES  →", link(sig2));
  console.log("    market vault:", sol(await conn.getBalance(vault)), "SOL (98% net)   reward vault:", sol(await conn.getBalance(rewardVault)), "SOL (2% fee)\n");

  // 3) settle the market as YES (mock keeper; the real path CPIs the TxOracle Merkle proof)
  const settle = new Transaction().add(new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [ak(market, false, true), ak(wallet.publicKey, true, false)],
    data: Buffer.concat([disc("settle_market_mock"), u8(1) /* YES wins */]),
  }));
  const sig3 = await sendAndConfirmTransaction(conn, settle, [wallet], { commitment: "confirmed" });
  console.log("✓ settled YES  →", link(sig3), "\n");

  // 4) winner claims the pot — lamports leave the vault PDA back to the user
  const before = await conn.getBalance(wallet.publicKey);
  const claim = new Transaction().add(new TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      ak(market, false, false), ak(position, false, true), ak(vault, false, true),
      ak(wallet.publicKey, true, true), ak(SYS, false, false),
    ],
    data: disc("claim_payout"),
  }));
  const sig4 = await sendAndConfirmTransaction(conn, claim, [wallet], { commitment: "confirmed" });
  const gained = (await conn.getBalance(wallet.publicKey)) - before;
  console.log("✓ claimed payout  →", link(sig4));
  console.log("    wallet received ~", sol(gained), "SOL back (whole pot, minus tx fee)\n");

  console.log("Full loop verified on-chain: bet → escrow → settle → payout.");
  console.log("  market vault now:", sol(await conn.getBalance(vault)), "SOL (drained to winner)");
  console.log("Explorer (program):", `https://explorer.solana.com/address/${PROGRAM_ID.toBase58()}?cluster=devnet`);
}

main().catch((e) => { console.error("DEMO FAILED:", e.message || e); process.exit(1); });
