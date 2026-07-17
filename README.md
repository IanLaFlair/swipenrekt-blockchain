# Swipe n Rekt — Blockchain (Solana / Anchor)

On-chain program for Swipe n Rekt. **On-chain = money & proof; everything else is the NestJS backend.**

- **Escrow** USDC per market (one swipe card = one market)
- **Trustless settlement** via CPI into TxLINE TxOracle `validate_stat`
- **Proportional payouts** to winners
- **Reward pool** — 2% fee collection + %-based distributions
- **cNFT card supply caps** per player/rarity

Program (Rust) lives in `programs/swipe_n_rekt/src`. Implements the full plan in
`Swipe_n_Rekt_Blockchain_Plan.md`.

---

## Toolchain (install once)

Not present in this environment — install to build/test/deploy:

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Solana CLI (Agave)
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
# Anchor via avm
cargo install --git https://github.com/coral-xyz/anchor avm --force
avm install 0.30.1 && avm use 0.30.1
# JS deps
npm install   # or: yarn
```

## Build / test / deploy

```bash
# 1. Generate the real program keypair and sync it into lib.rs + Anchor.toml
anchor keys sync            # replaces the placeholder Fg6Pa... program id

anchor build               # emits target/idl/swipe_n_rekt.json + target/types/*.ts
anchor test                # local validator, runs tests/swipe_n_rekt.ts

# devnet
solana config set --url devnet
solana airdrop 2
anchor deploy --provider.cluster devnet
anchor run --provider.cluster devnet   # runs migrations/deploy.ts (inits reward pool)
```

> The declared program id is a **placeholder**. Run `anchor keys sync` before the
> first real build so `declare_id!` and `Anchor.toml` match your deploy keypair.

---

## Instructions

| Instruction | Caller | Purpose |
|---|---|---|
| `initialize_reward_pool` | authority (once) | Create global reward pool + vault |
| `initialize_market` | backend authority | New market for a swipe card |
| `place_bet(side, amount, price)` | user | Lock USDC, 2% fee → pool |
| `settle_market(side, oracle_args)` | keeper | **CPI to TxOracle**, trustless |
| `settle_market_mock(side)` | authority | Week-1 fallback / EqualTo-NO |
| `claim_payout` | winner | Proportional pot share |
| `mint_card(catalog_id, rarity)` | user | Supply-capped card mint |
| `claim_set_reward(country, period, bps)` | user + authority | % of pool for a set |

### PDA seeds (agreed with backend — plan §8)

```
market       [b"market", fixture_id(le8), stat_key(le4), window_start(le8)]
position     [b"position", market, user]
vault        [b"vault", market]
reward_pool  [b"reward_pool"]
reward_vault [b"reward_vault"]
card_supply  [b"card_supply", catalog_id(le4)]
```

### Settled decisions (defaults from plan §8)

| Item | Value |
|---|---|
| Market seed | `[b"market", fixture_id, stat_key, window_start]` |
| Fee | 2% (`FEE_BPS = 200`) → reward pool |
| `initialize_market` caller | backend authority |
| `settle_market` caller | backend keeper (`market.authority`) |
| USDC devnet mint | `4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU` (override via `USDC_MINT`) |
| catalog_id | player CSV index, `1..=1248` |

Supply caps: Common/Uncommon = unlimited, Rare = 5 000, Epic = 1 000, Legendary = 300.

---

## Settlement design (the important bit)

`settle_market` is **trustless**: the keeper *claims* a winning side and supplies
the Merkle proofs it pulled from the TxLINE API, but the **contract** rebuilds the
predicate that must hold for that claim and CPIs `validate_stat`. If the proof
doesn't support the predicate, the CPI errors and the whole tx reverts — a keeper
cannot lie.

Predicate reconstruction (integer stats):

```
YES claim:  market's own (threshold, comparison)
NO  claim:  the negation, as a single comparison:
   stat >  T   →  NO: stat <  T+1
   stat <  T   →  NO: stat >  T-1
   stat == T   →  not expressible → use settle_market_mock / validate_stat_v2
```

The oracle types (`ProofNode`, `ScoresBatchSummary`, `StatTerm`, `TraderPredicate`,
…) are mirrored in `src/txoracle/types.rs`; the raw CPI (with the precomputed
`validate_stat` discriminator) is in `src/txoracle/cpi.rs`. **Before relying on it,
diff these against `idl/txoracle.json` and the example
`examples/devnet/scripts/subscription_scores_1stat.ts` in `txodds/tx-on-chain`** —
account order and the `daily_scores_merkle_roots` account are the two things most
likely to need adjustment once you run it against the real cloned program on devnet.

### Backend → `settle_market` (TS sketch)

```ts
// Backend fetches proofs from TxLINE, then:
await program.methods
  .settleMarket(SIDE_YES, {
    ts: new BN(unixTs),
    fixtureSummary: { fixtureId, updateStats: {...}, eventsSubTreeRoot: [...] },
    fixtureProof: [{ hash: [...], isRightSibling: false }, ...],
    mainTreeProof: [ ... ],
    predicate: { threshold: 0, comparison: { greaterThan: {} } }, // overwritten on-chain
    statA: { statToProve: { key, value, period }, eventStatRoot: [...], statProof: [...] },
    statB: null,
    op: null,
  })
  .accounts({
    market,
    dailyScoresMerkleRoots: TXORACLE_ROOTS_ACCOUNT,
    txoracleProgram: new PublicKey("9ExbZjAapQww1vfcisDmrngPinHTEfpjYRWMunJgcKaA"),
    authority: keeper.publicKey,
  })
  .signers([keeper])
  .rpc();
```

---

## Events (backend listens)

`MarketInitialized`, `BetPlaced`, `MarketSettled`, `PayoutClaimed`, `CardMinted`,
`SetRewardClaimed` — see `src/events.rs`.

## Handoff to backend

After `anchor build`, send the backend team:
1. `target/idl/swipe_n_rekt.json` (IDL)
2. `target/types/swipe_n_rekt.ts` (TS types)
3. The deployed **program id** (from `anchor keys sync` / deploy output)
4. The USDC devnet mint address in use

## Open items to confirm with backend

- Exact USDC devnet mint address (default assumed above)
- TxOracle `daily_scores_merkle_roots` account address on devnet + exact account
  ordering for `validate_stat` (verify against the cloned program)
- Merkle-proof wire format from the TxLINE API → `ValidateStatArgs`
- Bubblegum cNFT account set for `mint_card` (integration point marked in code)
