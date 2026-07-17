// Swipe n Rekt — on-chain client SDK.
//
// The reusable bridge between any caller (frontend + Phantom, or the NestJS
// backend keeper) and the deployed `swipe_n_rekt` Anchor program. It only covers
// the USER-signed instructions the frontend needs — place_bet, claim_payout,
// mint_card — plus PDA derivation and account reads. Market creation and
// settlement are backend-authority instructions and live in the backend keeper.
//
// REQUIRES `anchor build` first: this imports the generated IDL + types from
// ../target. The program id is read from the IDL `address` field, so it stays
// correct automatically after `anchor keys sync` / deploy — nothing to hardcode.
//
//   import idl        from "../target/idl/swipe_n_rekt.json"
//   import type { SwipeNRekt } from "../target/types/swipe_n_rekt"

import { AnchorProvider, Program, BN } from "@coral-xyz/anchor";
import {
  Connection,
  PublicKey,
  SystemProgram,
  Keypair,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import idl from "../target/idl/swipe_n_rekt.json";
import type { SwipeNRekt } from "../target/types/swipe_n_rekt";

// ---- constants mirrored from programs/.../constants.rs -----------------------
export const SIDE_NO = 0;
export const SIDE_YES = 1;
export const COMPARISON = { GreaterThan: 0, LessThan: 1, EqualTo: 2 } as const;
export const RARITY = { Common: 0, Uncommon: 1, Rare: 2, Epic: 3, Legendary: 4 } as const;
export const FEE_BPS = 200; // 2% — taken on every bet, informational for UI
/** USDC devnet mint used by the program (override via env / config). */
export const USDC_DEVNET_MINT = new PublicKey("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU");

const enc = (s: string) => Buffer.from(s, "utf8");
const i64le = (v: number | bigint | BN) => new BN(v.toString()).toArrayLike(Buffer, "le", 8);
const u32le = (v: number | BN) => new BN(v.toString()).toArrayLike(Buffer, "le", 4);

// ---- PDA derivation (pure — no Program instance needed) ----------------------
// Backend and frontend derive identical addresses from these.
export function marketPda(
  programId: PublicKey,
  fixtureId: number | bigint | BN,
  statKey: number | BN,
  windowStart: number | bigint | BN,
): PublicKey {
  return PublicKey.findProgramAddressSync(
    [enc("market"), i64le(fixtureId), u32le(statKey), i64le(windowStart)],
    programId,
  )[0];
}
export function positionPda(programId: PublicKey, market: PublicKey, user: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([enc("position"), market.toBuffer(), user.toBuffer()], programId)[0];
}
export function vaultPda(programId: PublicKey, market: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([enc("vault"), market.toBuffer()], programId)[0];
}
export function rewardPoolPda(programId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([enc("reward_pool")], programId)[0];
}
export function rewardVaultPda(programId: PublicKey): PublicKey {
  return PublicKey.findProgramAddressSync([enc("reward_vault")], programId)[0];
}
export function cardSupplyPda(programId: PublicKey, catalogId: number | BN): PublicKey {
  return PublicKey.findProgramAddressSync([enc("card_supply"), u32le(catalogId)], programId)[0];
}

/** The three params that identify a market — the backend must expose these per swipe card. */
export interface MarketRef {
  fixtureId: number | bigint | BN;
  statKey: number | BN;
  windowStart: number | bigint | BN;
}

/** Convert a human USDC amount (e.g. 20) to base units (6 decimals → 20_000_000). */
export function usdcToBaseUnits(amount: number, decimals = 6): BN {
  return new BN(Math.round(amount * 10 ** decimals));
}
/** Implied probability (0..1) → basis points u32, for the `price` field. */
export function priceToBps(prob: number): number {
  return Math.max(0, Math.min(10_000, Math.round(prob * 10_000)));
}

// -----------------------------------------------------------------------------
export class SwipeClient {
  readonly program: Program<SwipeNRekt>;

  constructor(provider: AnchorProvider) {
    // Anchor 0.30: address comes from the IDL, not a separate argument.
    this.program = new Program(idl as SwipeNRekt, provider);
  }

  get programId(): PublicKey { return this.program.programId; }
  get provider(): AnchorProvider { return this.program.provider as AnchorProvider; }
  get wallet(): PublicKey { return this.provider.wallet.publicKey; }

  // ---- reads ----------------------------------------------------------------
  deriveMarket(ref: MarketRef): PublicKey {
    return marketPda(this.programId, ref.fixtureId, ref.statKey, ref.windowStart);
  }
  fetchMarket(market: PublicKey) { return this.program.account.market.fetch(market); }
  fetchPosition(position: PublicKey) { return this.program.account.position.fetchNullable(position); }

  /** The signed-in user's position on a market (null if they haven't bet). */
  async getMyPosition(ref: MarketRef) {
    const market = this.deriveMarket(ref);
    const pos = positionPda(this.programId, market, this.wallet);
    return this.fetchPosition(pos);
  }

  // ---- place_bet ------------------------------------------------------------
  // side: SIDE_YES | SIDE_NO. amount: base units (use usdcToBaseUnits).
  // price: implied-prob bps (use priceToBps) — stored for analytics only.
  async placeBet(args: {
    market: MarketRef;
    side: number;
    amount: BN | number;
    priceBps: number;
    mint?: PublicKey;          // defaults to the market's mint (fetched if omitted)
    tokenProgram?: PublicKey;  // defaults to the classic SPL Token program
  }): Promise<string> {
    const user = this.wallet;
    const market = this.deriveMarket(args.market);
    const mint = args.mint ?? (await this.fetchMarket(market)).mint;
    const tokenProgram = args.tokenProgram ?? TOKEN_PROGRAM_ID;
    const amount = args.amount instanceof BN ? args.amount : new BN(args.amount);

    return this.program.methods
      .placeBet(args.side, amount, args.priceBps)
      .accounts({
        market,
        position: positionPda(this.programId, market, user),
        vault: vaultPda(this.programId, market),
        rewardPool: rewardPoolPda(this.programId),
        rewardVault: rewardVaultPda(this.programId),
        mint,
        userTokenAccount: getAssociatedTokenAddressSync(mint, user, false, tokenProgram),
        user,
        tokenProgram,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }

  // ---- claim_payout ---------------------------------------------------------
  // Only valid after the market is settled and the caller was on the winning side.
  async claimPayout(args: {
    market: MarketRef;
    mint?: PublicKey;
    tokenProgram?: PublicKey;
  }): Promise<string> {
    const user = this.wallet;
    const market = this.deriveMarket(args.market);
    const mint = args.mint ?? (await this.fetchMarket(market)).mint;
    const tokenProgram = args.tokenProgram ?? TOKEN_PROGRAM_ID;

    return this.program.methods
      .claimPayout()
      .accounts({
        market,
        position: positionPda(this.programId, market, user),
        vault: vaultPda(this.programId, market),
        mint,
        userTokenAccount: getAssociatedTokenAddressSync(mint, user, false, tokenProgram),
        user,
        tokenProgram,
      })
      .rpc();
  }

  // ---- mint_card ------------------------------------------------------------
  // Enforces the on-chain supply cap. `asset` is the cNFT id; until the Bubblegum
  // CPI is wired it can be any placeholder pubkey (the program only records it).
  async mintCard(args: { catalogId: number; rarity: number; asset?: PublicKey }): Promise<string> {
    const user = this.wallet;
    return this.program.methods
      .mintCard(args.catalogId, args.rarity)
      .accounts({
        cardSupply: cardSupplyPda(this.programId, args.catalogId),
        user,
        asset: args.asset ?? Keypair.generate().publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  }
}

// ---- provider helpers -------------------------------------------------------
/**
 * Anchor-compatible wallet from a Phantom-style injected provider
 * (window.solana / window.phantom.solana). Used by the browser frontend.
 */
export function anchorWalletFromInjected(injected: any) {
  return {
    publicKey: injected.publicKey,
    signTransaction: (tx: any) => injected.signTransaction(tx),
    signAllTransactions: (txs: any[]) =>
      injected.signAllTransactions ? injected.signAllTransactions(txs) : Promise.all(txs.map((t) => injected.signTransaction(t))),
  };
}

/** Build a SwipeClient for the browser from a connected Phantom provider. */
export function clientFromInjectedWallet(connection: Connection, injected: any): SwipeClient {
  const provider = new AnchorProvider(connection, anchorWalletFromInjected(injected) as any, {
    commitment: "confirmed",
  });
  return new SwipeClient(provider);
}
