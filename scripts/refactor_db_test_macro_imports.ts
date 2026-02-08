#!/usr/bin/env bun

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();
const MACRO_IMPORT = "use dps_auth_test_macros::dps_auth_db_test;";

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

function ensureMacroImportNearTop(input: string): { output: string; changed: boolean } {
  if (input.includes(MACRO_IMPORT)) return { output: input, changed: false };

  const lines = input.split(/\r?\n/);
  let insertAt = 0;

  // Keep file header comments and inner attributes (#![...]) at the top.
  while (insertAt < lines.length) {
    const l = lines[insertAt];
    if (l.startsWith("//")) {
      insertAt++;
      continue;
    }
    if (l.startsWith("#![")) {
      insertAt++;
      continue;
    }
    if (l.trim() === "") {
      insertAt++;
      continue;
    }
    break;
  }

  lines.splice(insertAt, 0, MACRO_IMPORT);
  return { output: lines.join("\n"), changed: true };
}

function rewriteContent(input: string): { output: string; replacements: number } {
  let output = input;
  let replacements = 0;

  // Convert fully-qualified macro attributes to local name.
  output = output.replace(
    /#\[\s*dps_auth_test_macros::dps_auth_db_test\s*/g,
    () => {
      replacements++;
      return "#[dps_auth_db_test";
    },
  );

  // Remove explicit crate_path args now that the macro defaults correctly.
  // Handles:
  // - (crate_path = crate)
  // - (crate_path = dps_auth_api, pool_size = 1)
  // - (pool_size = 1, crate_path = dps_auth_api)
  output = output.replace(
    /#\[dps_auth_db_test\(\s*crate_path\s*=\s*[^,\)\]]+\s*(?:,\s*)?([^\)]*)\)\]/g,
    (_full, rest: string) => {
      replacements++;
      const trimmed = (rest ?? "").trim();
      if (trimmed.length === 0) return "#[dps_auth_db_test]";
      return `#[dps_auth_db_test(${trimmed})]`;
    },
  );
  output = output.replace(
    /#\[dps_auth_db_test\(\s*([^\)]*?)\s*(?:,\s*)?crate_path\s*=\s*[^,\)\]]+\s*\)\]/g,
    (_full, before: string) => {
      replacements++;
      const trimmed = (before ?? "").trim().replace(/,\s*$/, "");
      if (trimmed.length === 0) return "#[dps_auth_db_test]";
      return `#[dps_auth_db_test(${trimmed})]`;
    },
  );

  // If we used the macro, ensure the import exists.
  if (replacements > 0 && /#\[\s*dps_auth_db_test\b/.test(output)) {
    const res = ensureMacroImportNearTop(output);
    if (res.changed) replacements++;
    output = res.output;
  }

  return { output, replacements };
}

async function rewriteFile(file: string): Promise<{ file: string; changed: boolean; replacements: number }> {
  const original = await readFile(file, "utf8");
  const { output, replacements } = rewriteContent(original);
  if (output === original) return { file, changed: false, replacements: 0 };
  await writeFile(file, output, "utf8");
  return { file, changed: true, replacements };
}

async function main() {
  const srcDir = join(ROOT, "src");
  const testsDir = join(ROOT, "tests");
  const files = [
    ...(await listRustFiles(srcDir)),
    ...(await listRustFiles(testsDir).catch(() => [])),
  ];

  let touched = 0;
  let totalReplacements = 0;
  for (const file of files) {
    const res = await rewriteFile(file);
    if (res.changed) {
      touched++;
      totalReplacements += res.replacements;
    }
  }

  process.stdout.write(
    `Updated ${touched} file(s); ${totalReplacements} change(s) (including import insertions)\n`,
  );
}

await main();
