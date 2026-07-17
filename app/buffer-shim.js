// Buffer + global polyfill for the browser bundle. @coral-xyz/anchor and
// @solana/web3.js assume Node's Buffer and `global` exist; esbuild injects this.
import { Buffer } from "buffer";
if (typeof globalThis.Buffer === "undefined") globalThis.Buffer = Buffer;
if (typeof globalThis.global === "undefined") globalThis.global = globalThis;
export { Buffer };
