// Browser bundle entry — exposes the on-chain SDK as `window.SNRChain` so the
// no-build Swipe N Rekt frontend can call it without a bundler of its own.
//
// Produce the bundle with:  npm run build:browser  (after `anchor build`)
// then copy app/dist/chain.bundle.js into the frontend repo.
import * as SDK from "./client";
import { Connection, PublicKey, clusterApiUrl } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

(globalThis as any).SNRChain = {
  ...SDK,
  Connection,
  PublicKey,
  clusterApiUrl,
  BN,
};
