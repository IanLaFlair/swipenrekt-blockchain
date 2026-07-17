# app/ — on-chain client bridge

The client SDK that connects callers to the deployed `swipe_n_rekt` program:

- **`client.ts`** — `SwipeClient` + pure PDA derivation. Covers the user-signed
  instructions (`place_bet`, `claim_payout`, `mint_card`) plus account reads.
  Wallet-agnostic: works with a backend `Keypair` wallet **or** a browser Phantom
  wallet. Market creation/settlement are backend-authority instructions and are
  NOT here.
- **`browser-entry.ts` / `build-browser.mjs`** — bundle the SDK into one IIFE
  (`window.SNRChain`) for the no-build frontend.

> The SDK imports `../target/idl/swipe_n_rekt.json` and `../target/types/...`, so
> **it only compiles after `anchor build`**. The program id is read from the IDL,
> so it self-updates after `anchor keys sync` / deploy — nothing to hardcode.

---

## ⚠️ The API ↔ chain gap you must close first

The current backend (`swipe-api.fachry.dev`) tracks bets as a **database balance**.
This program tracks bets as **USDC in a market vault PDA**. They are not connected.
To go on-chain, a market must exist on-chain for each swipe card, and the frontend
must be able to derive that market's PDA. That PDA comes from three params:

```
marketPda = PDA([ "market", fixture_id(i64 LE), stat_key(u32 LE), window_start(i64 LE) ])
```

**The `/proposition` API today returns `id, question, oddsYes/No, settlesAt` — none
of these three.** So the backend must, per proposition, additionally:

1. Call `initialize_market(fixture_id, stat_key, period, threshold, comparison,
   window_start, window_end)` (backend authority) when it creates the proposition.
2. Expose `fixtureId`, `statKey`, `windowStart` on the proposition payload (a
   `market` object) so the frontend can derive the same PDA and call `place_bet`.

Until that mapping exists, the frontend cannot place on-chain bets — it can only
run the DB flow. This is the single most important coordination item.

---

## Build → deploy → bundle (run locally; needs the toolchain)

```bash
# toolchain (once): rustup, solana/agave, anchor via avm — see ../README.md
anchor keys sync                         # real program id into declare_id! + Anchor.toml
anchor build                             # emits target/idl + target/types
anchor test                              # local validator, tests/swipe_n_rekt.ts

solana config set --url devnet
solana airdrop 2
anchor deploy --provider.cluster devnet  # deploy program
anchor run --provider.cluster devnet     # migrations/deploy.ts → initialize_reward_pool

# browser bundle for the frontend
npm i -D esbuild buffer
npm run build:browser                    # → app/dist/chain.bundle.js
cp app/dist/chain.bundle.js ../swipenrekt/chain.bundle.js
```

Devnet USDC: users need the program's USDC mint
(`4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU`, or your own) in their wallet ATA.
Mint some to demo wallets with `spl-token` for the demo.

---

## Frontend wiring (Swipe N Rekt `index.html`)

After copying `chain.bundle.js`, load it alongside `api.js`:

```html
<script src="./api.js"></script>
<script src="./chain.bundle.js"></script>   <!-- exposes window.SNRChain -->
<script src="./support.js"></script>
```

Then, in the component's `confirmPosition`, swap the on-chain path in where the
proposition carries `market` params (from the API bridge above). `SNR` is the
existing Phantom connection from `api.js`:

```js
// once, after Phantom connect:
const { Connection, clusterApiUrl, clientFromInjectedWallet } = window.SNRChain;
const connection = new Connection(clusterApiUrl('devnet'), 'confirmed');
const provider   = window.phantom.solana;            // connected in api.js
const chain      = clientFromInjectedWallet(connection, provider);

// on swipe-confirm, when card.market = { fixtureId, statKey, windowStart } exists:
const sig = await chain.placeBet({
  market:  card.market,
  side:    side === 'yes' ? window.SNRChain.SIDE_YES : window.SNRChain.SIDE_NO,
  amount:  window.SNRChain.usdcToBaseUnits(stake),   // 6-decimal USDC
  priceBps: window.SNRChain.priceToBps(price),       // implied prob → bps
});
// sig = the devnet tx signature → show it as the on-chain "receipt"
// (this is the real settlement receipt the Result screen should link to Explorer)

// on the Result screen, if the market is settled and the user won:
await chain.claimPayout({ market: card.market });
```

Phantom pops a signature request per `placeBet` / `claimPayout` (expected in Web3).
Wrap calls in try/catch and surface failures the same way `api.js` does today.

### Read a user's on-chain position
```js
const pos = await chain.getMyPosition(card.market); // null if none
// pos.side (0/1), pos.amount (base units), pos.price (bps), pos.claimed
```

---

## What stays on the backend (not in this SDK)

- `initialize_market` — backend authority, when a proposition is created
- `settle_market` — backend keeper, with TxLINE Merkle proofs (trustless CPI)
- Event indexing — listen for `BetPlaced` / `MarketSettled` / `PayoutClaimed`
  to keep the DB/leaderboard in sync with chain truth
