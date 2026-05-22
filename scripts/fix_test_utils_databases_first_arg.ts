#!/usr/bin/env bun

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();
const TARGET_DIRS = ["src", "tests"];

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

function rewrite(text: string): { text: string; replacements: number } {
  let out = text;
  let replacements = 0;

  // After renaming helpers from *_with_pool -> *_with_databases, the first arg is still often &main_pool.
  // Rewrite common patterns to pass &databases instead.
  const patterns: Array<[RegExp, string]> = [
    [/\b(create_test_user_with_databases(?:_and_password|_and_uuid)?|create_test_user_full_with_databases|create_test_role_with_databases|create_test_role_model_with_databases)\(\s*&main_pool\s*,/g, "$1(&databases,"],
    [/\b(create_test_user_with_databases(?:_and_password|_and_uuid)?|create_test_user_full_with_databases|create_test_role_with_databases|create_test_role_model_with_databases)\(\s*main_pool\s*,/g, "$1(databases,"],
    [/\b(create_test_user_with_databases(?:_and_password|_and_uuid)?|create_test_user_full_with_databases|create_test_role_with_databases|create_test_role_model_with_databases)\(\s*&pool\s*,/g, "$1(&databases,"],
    [/\b(create_test_user_with_databases(?:_and_password|_and_uuid)?|create_test_user_full_with_databases|create_test_role_with_databases|create_test_role_model_with_databases)\(\s*pool\s*,/g, "$1(databases,"],
  ];

  for (const [re, repl] of patterns) {
    const before = out;
    out = out.replace(re, repl);
    if (out !== before) replacements += (before.match(re) || []).length;
  }

  return { text: out, replacements };
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  const res = rewrite(original);
  if (res.text === original) return { file, changed: false, replacements: 0 };
  await writeFile(file, res.text, "utf8");
  return { file, changed: true, replacements: res.replacements };
}

async function main() {
  const files: string[] = [];
  for (const dir of TARGET_DIRS) {
    files.push(...(await listRustFiles(join(ROOT, dir)).catch(() => [])));
  }

  const changes: Change[] = [];
  for (const f of files) changes.push(await rewriteFile(f));

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
