// portable 免安装交付：tauri build --no-bundle 已产出单文件 exe，
// 本脚本将其复制到 bundle/portable/ 下作为分发物。
import { cpSync, mkdirSync, readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const config = JSON.parse(
  readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"),
);
const { productName, version } = config;

const exe = resolve(root, `src-tauri/target/release/${productName}.exe`);
const outDir = resolve(root, "src-tauri/target/release/bundle/portable");
const out = resolve(outDir, `${productName}_${version}_x64-portable.exe`);

mkdirSync(outDir, { recursive: true });
cpSync(exe, out);
console.log(`Portable build at: ${out}`);
