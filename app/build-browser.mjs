// Bundle the on-chain SDK into a single self-contained IIFE for the no-build
// frontend. Run AFTER `anchor build` (needs target/idl + target/types).
//
//   npm i -D esbuild buffer
//   npm run build:browser
//
// Output: app/dist/chain.bundle.js  →  copy into the frontend repo and load with
//   <script src="./chain.bundle.js"></script>   (exposes window.SNRChain)
import { build } from "esbuild";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));

await build({
  entryPoints: [resolve(here, "browser-entry.ts")],
  bundle: true,
  format: "iife",
  outfile: resolve(here, "dist/chain.bundle.js"),
  platform: "browser",
  target: ["es2020"],
  minify: true,
  sourcemap: true,
  // anchor/web3.js assume Node globals; inject the shim into every module.
  inject: [resolve(here, "buffer-shim.js")],
  define: {
    "process.env.NODE_ENV": '"production"',
    "process.env.ANCHOR_BROWSER": "true",
    global: "globalThis",
  },
});

console.log("✓ app/dist/chain.bundle.js");
