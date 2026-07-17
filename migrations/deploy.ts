// Anchor deploy migration. Runs after `anchor deploy`.
// Initializes the global reward pool once, using the USDC devnet mint.

import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

// USDC devnet mint (agree the exact address with backend — plan §8).
// Circle USDC devnet: 4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU
const USDC_DEVNET_MINT = new PublicKey(
  process.env.USDC_MINT ?? "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU"
);

module.exports = async function (provider: anchor.AnchorProvider) {
  anchor.setProvider(provider);
  const program = anchor.workspace.SwipeNRekt as anchor.Program<any>;

  const [rewardPool] = PublicKey.findProgramAddressSync(
    [Buffer.from("reward_pool")],
    program.programId
  );

  const info = await provider.connection.getAccountInfo(rewardPool);
  if (info) {
    console.log("reward_pool already initialized:", rewardPool.toBase58());
    return;
  }

  const [rewardVault] = PublicKey.findProgramAddressSync(
    [Buffer.from("reward_vault")],
    program.programId
  );

  const tx = await program.methods
    .initializeRewardPool()
    .accounts({
      rewardPool,
      rewardVault,
      mint: USDC_DEVNET_MINT,
      authority: provider.wallet.publicKey,
      tokenProgram: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
      systemProgram: anchor.web3.SystemProgram.programId,
    })
    .rpc();

  console.log("reward_pool initialized:", rewardPool.toBase58(), "tx:", tx);
};
