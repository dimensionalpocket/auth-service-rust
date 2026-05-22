#!/usr/bin/env bun

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();
const TARGET_DIRS = ["src", "tests"];

const RENAMES: Array<{ from: string; to: string }> = [
  { from: "create_test_user_with_pool_and_password", to: "create_test_user_with_databases_and_password" },
  { from: "create_test_user_full_with_pool", to: "create_test_user_full_with_databases" },
  { from: "create_test_user_with_pool_and_uuid", to: "create_test_user_with_databases_and_uuid" },
  { from: "create_test_user_with_pool", to: "create_test_user_with_databases" },
  { from: "create_test_role_model_with_pool", to: "create_test_role_model_with_databases" },
  { from: "create_test_role_with_pool", to: "create_test_role_with_databases" },
];

type Change = {
  file: string;
  changed: boolean;
  replacements: number;
};

async function listRustFiles(dir: string): Promise<string[]> {
  const out: string[] = [];
  const entries = await readdir(dir);
  for (const entry of entries) {
    if (entry === "target" || entry === ".git") continue;
    const full = join(dir, entry);
    const st = await stat(full);
    if (st.isDirectory()) {
      out.push(...(await listRustFiles(full)));
    } else if (st.isFile() && full.endsWith(".rs")) {
      out.push(full);
    }
  }
  return out;
}

function renameInText(text: string): { text: string; replacements: number } {
  let out = text;
  let replacements = 0;

  for (const { from, to } of RENAMES) {
    // Replace identifier usage only.
    const re = new RegExp(`\\b${from}\\b`, "g");
    const before = out;
    out = out.replace(re, to);
    if (out !== before) {
      // Count matches without needing matchAll on huge strings.
      const count = (before.match(re) || []).length;
      replacements += count;
    }
  }

  return { text: out, replacements };
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  const res = renameInText(original);
  if (res.text === original) {
    return { file, changed: false, replacements: 0 };
  }
  await writeFile(file, res.text, "utf8");
  return { file, changed: true, replacements: res.replacements };
}

async function main() {
  const files: string[] = [];
  for (const dir of TARGET_DIRS) {
    files.push(...(await listRustFiles(join(ROOT, dir)).catch(() => [])));
  }

  const changes: Change[] = [];
  for (const f of files) {
    changes.push(await rewriteFile(f));
  }

  const touched = changes.filter((c) => c.changed);
  const total = touched.reduce((acc, c) => acc + c.replacements, 0);
  process.stdout.write(`Updated ${touched.length} file(s); ${total} replacement(s)\n`);
  for (const c of touched.slice(0, 80)) {
    process.stdout.write(`- ${c.file}: ${c.replacements}\n`);
  }
  if (touched.length > 80) {
    process.stdout.write(`- ... and ${touched.length - 80} more\n`);
  }
}

await main();
