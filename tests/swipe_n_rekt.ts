import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { SwipeNRekt } from "../target/types/swipe_n_rekt";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } from "@solana/web3.js";
import { assert } from "chai";

// All value transfers are native SOL (lamports, 9 decimals).
const SIDE_NO = 0;
const SIDE_YES = 1;
const CMP_GT = 0; // GreaterThan
const enc = anchor.utils.bytes.utf8.encode;
const SOL = LAMPORTS_PER_SOL;

describe("swipe_n_rekt", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SwipeNRekt as Program<SwipeNRekt>;
  const conn = provider.connection;
  const authority = (provider.wallet as anchor.Wallet).payer;

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
  const posPda = (who: PublicKey) =>
    PublicKey.findProgramAddressSync(
      [enc("position"), market.toBuffer(), who.toBuffer()],
      program.programId
    )[0];

  before(async () => {
    for (const kp of [alice, bob]) {
      const sig = await conn.requestAirdrop(kp.publicKey, 100 * SOL);
      await conn.confirmTransaction(sig);
    }
    [rewardPool] = PublicKey.findProgramAddressSync([enc("reward_pool")], program.programId);
    [rewardVault] = PublicKey.findProgramAddressSync([enc("reward_vault")], program.programId);
    [market] = PublicKey.findProgramAddressSync(marketSeeds(), program.programId);
    [vault] = PublicKey.findProgramAddressSync([enc("vault"), market.toBuffer()], program.programId);
  });

  it("initializes the reward pool", async () => {
    await program.methods
      .initializeRewardPool()
      .accounts({ rewardPool, authority: authority.publicKey, systemProgram: SystemProgram.programId })
      .rpc();
    const pool = await program.account.rewardPool.fetch(rewardPool);
    assert.equal(pool.totalCollected.toNumber(), 0);
  });

  it("initializes a market", async () => {
    await program.methods
      .initializeMarket(fixtureId, statKey, period, threshold, CMP_GT, windowStart, windowEnd)
      .accounts({ market, authority: authority.publicKey, systemProgram: SystemProgram.programId })
      .rpc();
    const m = await program.account.market.fetch(market);
    assert.equal(m.fixtureId.toString(), fixtureId.toString());
    assert.equal(m.threshold, threshold);
    assert.deepEqual(m.status, { open: {} });
  });

  it("takes bets on both sides, collecting 2% fee (in SOL)", async () => {
    // Alice bets 1 SOL YES, Bob bets 1 SOL NO.
    await program.methods
      .placeBet(SIDE_YES, new BN(1 * SOL), 5000)
      .accounts({
        market,
        position: posPda(alice.publicKey),
        vault,
        rewardPool,
        rewardVault,
        user: alice.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([alice])
      .rpc();

    await program.methods
      .placeBet(SIDE_NO, new BN(1 * SOL), 5000)
      .accounts({
        market,
        position: posPda(bob.publicKey),
        vault,
        rewardPool,
        rewardVault,
        user: bob.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([bob])
      .rpc();

    const m = await program.account.market.fetch(market);
    // 2% fee -> 0.98 SOL net each side.
    assert.equal(m.totalYes.toNumber(), 0.98 * SOL);
    assert.equal(m.totalNo.toNumber(), 0.98 * SOL);

    // Vault balances are PDAs (no tx-fee noise) — assert exactly.
    assert.equal(await conn.getBalance(rewardVault), 0.04 * SOL); // 0.02 + 0.02
    assert.equal(await conn.getBalance(vault), 1.96 * SOL);
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
    const vaultBefore = await conn.getBalance(vault);
    const aliceBefore = await conn.getBalance(alice.publicKey);

    await program.methods
      .claimPayout()
      .accounts({ market, position: posPda(alice.publicKey), vault, user: alice.publicKey, systemProgram: SystemProgram.programId })
      .signers([alice])
      .rpc();

    // Alice is the only YES staker -> the entire 1.96 SOL pot leaves the vault.
    assert.equal(vaultBefore - (await conn.getBalance(vault)), 1.96 * SOL);
    // Alice receives it (minus her own ~5000-lamport tx fee).
    const gained = (await conn.getBalance(alice.publicKey)) - aliceBefore;
    assert.isAbove(gained, 1.96 * SOL - 20_000);

    // Bob (loser) cannot claim.
    let failed = false;
    try {
      await program.methods
        .claimPayout()
        .accounts({ market, position: posPda(bob.publicKey), vault, user: bob.publicKey, systemProgram: SystemProgram.programId })
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
      .accounts({ cardSupply, user: alice.publicKey, asset, systemProgram: SystemProgram.programId })
      .signers([alice])
      .rpc();
    const cs = await program.account.cardSupply.fetch(cardSupply);
    assert.equal(cs.minted, 1);
    assert.equal(cs.cap, 300);
    assert.equal(cs.rarity, 4);
  });

  it("distributes a % of the reward pool to a set completer (SOL)", async () => {
    const bobBefore = await conn.getBalance(bob.publicKey);
    // 0.04 SOL in pool, distribute 50% = 0.02 SOL. Bob is the recipient (not a
    // signer here — the authority pays the tx fee), so his delta is exact.
    await program.methods
      .claimSetReward(7, 1, 5000)
      .accounts({ rewardPool, rewardVault, user: bob.publicKey, authority: authority.publicKey, systemProgram: SystemProgram.programId })
      .rpc();
    assert.equal((await conn.getBalance(bob.publicKey)) - bobBefore, 0.02 * SOL);
  });
});
