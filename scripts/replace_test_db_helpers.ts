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

function applyLineRewrite(line: string): { line: string; did: boolean; tmpVar?: string } {
  // Matches: let (pool, _tmp) = create_test_database().await;
  // Matches: let (pool, tmp) = create_test_databases_with_pool_size(1).await;
  const m = line.match(
    /^\s*let\s*\(\s*pool\s*,\s*(?<tmp>[_a-zA-Z0-9]+)\s*\)\s*=\s*(?<call>create_test_database(?:_with_[a-zA-Z0-9_]+)?\([^;]*\)\.await)\s*;\s*$/,
  );
  if (!m || !m.groups) return { line, did: false };

  const tmp = m.groups.tmp;
  const call = m.groups.call;

  // If tmp is a named var (not leading underscore), keep it as main tempfile.
  // If tmp is ignored, preserve ignore semantics.
  const mainTmp = tmp.startsWith("_") ? "_main_temp_file" : tmp;
  const sessionTmp = "_session_temp_file";

  const rewritten = line.replace(
    /let\s*\(\s*pool\s*,\s*[_a-zA-Z0-9]+\s*\)\s*=\s*create_test_database(?:_with_[a-zA-Z0-9_]+)?\([^;]*\)\.await\s*;/,
    `let (databases, ${mainTmp}, ${sessionTmp}) = ${call};`,
  );

  return { line: rewritten, did: true, tmpVar: tmp };
}

function needsPoolBinding(nextLine: string | undefined): boolean {
  if (!nextLine) return true;
  return !/^\s*let\s+pool\s*=\s*databases\.main\(\)(?:\.clone\(\))?\s*;\s*$/.test(
    nextLine,
  );
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  const lines = original.split(/\r?\n/);
  let replacements = 0;
  let changed = false;

  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Second-pass fix: if an earlier run inserted a borrowed pool,
    // make it owned so it can be passed into helpers expecting `SqlitePool`.
    if (/^\s*let\s+pool\s*=\s*databases\.main\(\)\s*;\s*$/.test(line)) {
      out.push(line.replace("databases.main();", "databases.main().clone();"));
      replacements++;
      changed = true;
      continue;
    }

    const res = applyLineRewrite(line);
    out.push(res.line);

    if (res.did) {
      replacements++;
      changed = true;

      const next = lines[i + 1];
      if (needsPoolBinding(next)) {
        out.push("  let pool = databases.main().clone();");
        changed = true;
      }
    }
  }

  if (!changed) return { file, changed: false, replacements: 0 };

  const updated = out.join("\n");
  if (updated !== original) {
    await writeFile(file, updated, "utf8");
  }

  return { file, changed: true, replacements };
}

async function main() {
  const srcDir = join(ROOT, "src");
  const testsDir = join(ROOT, "tests");
  const files = [
    ...(await listRustFiles(srcDir)),
    ...(await listRustFiles(testsDir).catch(() => [])),
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

  // Print a short list to help manual follow-up.
  for (const c of touched.slice(0, 50)) {
    process.stdout.write(`- ${c.file}: ${c.replacements}\n`);
  }
  if (touched.length > 50) {
    process.stdout.write(`- ... and ${touched.length - 50} more\n`);
  }
}

await main();
