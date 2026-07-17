import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SwipeNRekt } from "../target/types/swipe_n_rekt";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";

const SIDE_NO = 0;
const SIDE_YES = 1;
const CMP_GT = 0; // GreaterThan
const enc = anchor.utils.bytes.utf8.encode;

describe("swipe_n_rekt", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SwipeNRekt as Program<SwipeNRekt>;
  const conn = provider.connection;
  const authority = (provider.wallet as anchor.Wallet).payer;

  let mint: PublicKey;
  const alice = Keypair.generate(); // YES bettor (winner)
  const bob = Keypair.generate(); // NO bettor (loser)

  let rewardPool: PublicKey, rewardVault: PublicKey;
  let market: PublicKey, vault: PublicKey;

  const fixtureId = new BN(778899);
  const statKey = 1; // e.g. goals
  const period = 0; // full game
  const threshold = 1; // "> 1 goal?"
  const windowStart = new BN(Math.floor(Date.now() / 1000) - 10);
  const windowEnd = new BN(Math.floor(Date.now() / 1000) + 3600);

  const marketSeeds = () => [
    enc("market"),
    fixtureId.toArrayLike(Buffer, "le", 8),
    new BN(statKey).toArrayLike(Buffer, "le", 4),
    windowStart.toArrayLike(Buffer, "le", 8),
  ];

  before(async () => {
    for (const kp of [alice, bob]) {
      const sig = await conn.requestAirdrop(kp.publicKey, 2 * LAMPORTS_PER_SOL);
      await conn.confirmTransaction(sig);
    }
    // 6-decimal mint like USDC.
    mint = await createMint(conn, authority, authority.publicKey, null, 6);

    [rewardPool] = PublicKey.findProgramAddressSync([enc("reward_pool")], program.programId);
    [rewardVault] = PublicKey.findProgramAddressSync([enc("reward_vault")], program.programId);
    [market] = PublicKey.findProgramAddressSync(marketSeeds(), program.programId);
    [vault] = PublicKey.findProgramAddressSync(
      [enc("vault"), market.toBuffer()],
      program.programId
    );
  });

  it("initializes the reward pool", async () => {
    await program.methods
      .initializeRewardPool()
      .accounts({
        rewardPool,
        rewardVault,
        mint,
        authority: authority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    const pool = await program.account.rewardPool.fetch(rewardPool);
    assert.equal(pool.totalCollected.toNumber(), 0);
  });

  it("initializes a market", async () => {
    await program.methods
      .initializeMarket(fixtureId, statKey, period, threshold, CMP_GT, windowStart, windowEnd)
      .accounts({
        market,
        vault,
        mint,
        authority: authority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    const m = await program.account.market.fetch(market);
    assert.equal(m.fixtureId.toString(), fixtureId.toString());
    assert.equal(m.threshold, threshold);
    assert.deepEqual(m.status, { open: {} });
  });

  const fund = async (kp: Keypair, amount: number) => {
    const ata = await getOrCreateAssociatedTokenAccount(conn, authority, mint, kp.publicKey);
    await mintTo(conn, authority, mint, ata.address, authority, amount);
    return ata.address;
  };

  it("takes bets on both sides, collecting 2% fee", async () => {
    const aliceAta = await fund(alice, 100_000_000); // 100 USDC
    const bobAta = await fund(bob, 100_000_000);

    const [alicePos] = PublicKey.findProgramAddressSync(
      [enc("position"), market.toBuffer(), alice.publicKey.toBuffer()],
      program.programId
    );
    const [bobPos] = PublicKey.findProgramAddressSync(
      [enc("position"), market.toBuffer(), bob.publicKey.toBuffer()],
      program.programId
    );

    // Alice bets 100 USDC YES.
    await program.methods
      .placeBet(SIDE_YES, new BN(100_000_000), 5000)
      .accounts({
        market,
        position: alicePos,
        vault,
        rewardPool,
        rewardVault,
        mint,
        userTokenAccount: aliceAta,
        user: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([alice])
      .rpc();

    // Bob bets 100 USDC NO.
    await program.methods
      .placeBet(SIDE_NO, new BN(100_000_000), 5000)
      .accounts({
        market,
        position: bobPos,
        vault,
        rewardPool,
        rewardVault,
        mint,
        userTokenAccount: bobAta,
        user: bob.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([bob])
      .rpc();

    const m = await program.account.market.fetch(market);
    // 2% fee -> 98 USDC net each side.
    assert.equal(m.totalYes.toNumber(), 98_000_000);
    assert.equal(m.totalNo.toNumber(), 98_000_000);

    const rv = await getAccount(conn, rewardVault);
    assert.equal(Number(rv.amount), 4_000_000); // 2 + 2 USDC fees
    const v = await getAccount(conn, vault);
    assert.equal(Number(v.amount), 196_000_000);
  });

  it("settles the market (mock) as YES", async () => {
    await program.methods
      .settleMarketMock(SIDE_YES)
      .accounts({ market, authority: authority.publicKey })
      .rpc();
    const m = await program.account.market.fetch(market);
    assert.deepEqual(m.status, { settled: {} });
    assert.equal(m.winningSide, SIDE_YES);
  });

  it("pays the winner the whole pot, rejects the loser", async () => {
    const aliceAta = await getOrCreateAssociatedTokenAccount(
      conn,
      authority,
      mint,
      alice.publicKey
    );
    const before = Number((await getAccount(conn, aliceAta.address)).amount);

    const [alicePos] = PublicKey.findProgramAddressSync(
      [enc("position"), market.toBuffer(), alice.publicKey.toBuffer()],
      program.programId
    );
    await program.methods
      .claimPayout()
      .accounts({
        market,
        position: alicePos,
        vault,
        mint,
        userTokenAccount: aliceAta.address,
        user: alice.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([alice])
      .rpc();

    const after = Number((await getAccount(conn, aliceAta.address)).amount);
    // Alice is the only YES staker -> collects the entire 196 USDC pot.
    assert.equal(after - before, 196_000_000);

    // Bob (loser) cannot claim.
    const bobAta = await getOrCreateAssociatedTokenAccount(conn, authority, mint, bob.publicKey);
    const [bobPos] = PublicKey.findProgramAddressSync(
      [enc("position"), market.toBuffer(), bob.publicKey.toBuffer()],
      program.programId
    );
    let failed = false;
    try {
      await program.methods
        .claimPayout()
        .accounts({
          market,
          position: bobPos,
          vault,
          mint,
          userTokenAccount: bobAta.address,
          user: bob.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([bob])
        .rpc();
    } catch {
      failed = true;
    }
    assert.isTrue(failed, "loser claim should fail");
  });

  it("enforces the legendary supply cap conceptually (mint increments counter)", async () => {
    const catalogId = 42;
    const rarity = 4; // legendary, cap 300
    const [cardSupply] = PublicKey.findProgramAddressSync(
      [enc("card_supply"), new BN(catalogId).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    const asset = Keypair.generate().publicKey;
    await program.methods
      .mintCard(catalogId, rarity)
      .accounts({
        cardSupply,
        user: alice.publicKey,
        asset,
        systemProgram: SystemProgram.programId,
      })
      .signers([alice])
      .rpc();
    const cs = await program.account.cardSupply.fetch(cardSupply);
    assert.equal(cs.minted, 1);
    assert.equal(cs.cap, 300);
    assert.equal(cs.rarity, 4);
  });

  it("distributes a % of the reward pool to a set completer", async () => {
    const winnerAta = await getOrCreateAssociatedTokenAccount(
      conn,
      authority,
      mint,
      bob.publicKey
    );
    const before = Number((await getAccount(conn, winnerAta.address)).amount);
    // 4 USDC in pool, distribute 50% = 2 USDC.
    await program.methods
      .claimSetReward(7, 1, 5000)
      .accounts({
        rewardPool,
        rewardVault,
        mint,
        userTokenAccount: winnerAta.address,
        user: bob.publicKey,
        authority: authority.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    const after = Number((await getAccount(conn, winnerAta.address)).amount);
    assert.equal(after - before, 2_000_000);
  });
});
