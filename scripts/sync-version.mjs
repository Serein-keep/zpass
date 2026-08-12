#!/usr/bin/env node
// 将 package.json 的 version 同步到 src-tauri/Cargo.toml 的 [package] version
// 用法: node scripts/sync-version.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const pkgPath = path.join(root, "package.json");
const cargoPath = path.join(root, "src-tauri", "Cargo.toml");

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const version = pkg.version;

if (!version) {
  console.error("package.json 缺少 version 字段");
  process.exit(1);
}

const cargo = readFileSync(cargoPath, "utf8");
// 只匹配 [package] 段的 version（行首无缩进），不碰 dependencies 里的 version = "x.y.z" 行
const versionRegex = /^version = ".*"$/m;
if (!versionRegex.test(cargo)) {
  console.error("Cargo.toml 中未找到 [package] 段的 version 字段");
  process.exit(1);
}
const updated = cargo.replace(versionRegex, `version = "${version}"`);

writeFileSync(cargoPath, updated);
console.log(`synced package.json version "${version}" -> src-tauri/Cargo.toml`);
