#!/usr/bin/env bun

import { readdir, readFile, writeFile, stat } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();

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

function rewriteContent(input: string): { output: string; replacements: number } {
  let replacements = 0;
  let output = input;

  // Prefer fully-qualified attribute so tests don't need imports.
  // Handles both `#[dps_auth_db_test]` and `#[dps_auth_db_test(...)]`.
  output = output.replace(/\#\[\s*dps_auth_db_test\b/g, () => {
    replacements++;
    return "#[dps_auth_test_macros::dps_auth_db_test";
  });

  // Ensure the macro can resolve the test utils from unit tests (crate path = crate).
  output = output.replace(
    /\#\[\s*dps_auth_test_macros::dps_auth_db_test\s*\]/g,
    () => {
      replacements++;
      return "#[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]";
    },
  );
  output = output.replace(
    /\#\[\s*dps_auth_test_macros::dps_auth_db_test\s*\(([^\)]*)\)\s*\]/g,
    (full, inner: string) => {
      if (/\bcrate_path\s*=/.test(inner)) return full;
      replacements++;
      const trimmed = inner.trim();
      if (trimmed.length === 0) {
        return "#[dps_auth_test_macros::dps_auth_db_test(crate_path = crate)]";
      }
      return `#[dps_auth_test_macros::dps_auth_db_test(crate_path = crate, ${trimmed})]`;
    },
  );

  // Remove now-unnecessary `use dps_auth_test_macros::dps_auth_db_test;` lines.
  output = output.replace(
    /^\s*use\s+dps_auth_test_macros::dps_auth_db_test\s*;\s*\r?\n/gm,
    () => {
      replacements++;
      return "";
    },
  );

  return { output, replacements };
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  const { output, replacements } = rewriteContent(original);
  if (output === original) return { file, changed: false, replacements: 0 };
  await writeFile(file, output, "utf8");
  return { file, changed: true, replacements };
}

async function main() {
  const srcDir = join(ROOT, "src");
  const testsDir = join(ROOT, "tests");
  const macrosDir = join(ROOT, "crates");
  const files = [
    ...(await listRustFiles(srcDir)),
    ...(await listRustFiles(testsDir).catch(() => [])),
    ...(await listRustFiles(macrosDir).catch(() => [])),
  ];

  const changes: Change[] = [];
  for (const file of files) {
    changes.push(await rewriteFile(file));
  }

  const touched = changes.filter((c) => c.changed);
  const totalReplacements = touched.reduce((acc, c) => acc + c.replacements, 0);
  process.stdout.write(
    `Updated ${touched.length} file(s); ${totalReplacements} replacement(s)\n`,
  );
}

await main();
