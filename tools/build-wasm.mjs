// Rust 핵심 → WebAssembly 빌드 후 app/public/kgbone.wasm으로 복사.
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const release = !process.argv.includes("--debug");
const args = ["build", "-p", "kgbone-wasm", "--target", "wasm32-unknown-unknown"];
if (release) args.push("--release");
const local = join(homedir(), ".cargo", "bin", process.platform === "win32" ? "cargo.exe" : "cargo");
execFileSync(existsSync(local) ? local : "cargo", args, { cwd: root, stdio: "inherit" });
mkdirSync(join(root, "app", "public"), { recursive: true });
copyFileSync(join(root, "target", "wasm32-unknown-unknown", release ? "release" : "debug", "kgbone_wasm.wasm"), join(root, "app", "public", "kgbone.wasm"));
console.log("app/public/kgbone.wasm 갱신");
