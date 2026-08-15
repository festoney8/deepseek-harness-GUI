// macOS pkg 安装包：`tauri build --bundles app` 已产出 .app 目录（仅 arm64），
// 本脚本用 pkgbuild 将其打包为 pkg 安装包，仅限 macOS 上运行（非交叉编译）。
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const config = JSON.parse(readFileSync(resolve(root, "src-tauri/tauri.conf.json"), "utf8"));
const { productName, version, identifier } = config;

const appBundle = resolve(root, `src-tauri/target/release/bundle/macos/${productName}.app`);
if (!existsSync(appBundle)) {
  console.error(`App bundle not found: ${appBundle}`);
  console.error("Run `pnpm build:pkg` on macOS (this script is not cross-platform).");
  process.exit(1);
}
const outDir = resolve(root, "src-tauri/target/release/bundle/pkg");
mkdirSync(outDir, { recursive: true });
const out = resolve(outDir, `${productName}_${version}_arm64.pkg`);

execFileSync(
  "pkgbuild",
  [
    "--component",
    appBundle,
    "--install-location",
    "/Applications",
    "--identifier",
    identifier,
    "--version",
    version,
    out,
  ],
  { stdio: "inherit" },
);
console.log(`pkg build at: ${out}`);
