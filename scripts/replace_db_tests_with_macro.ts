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

function ensureMacroImport(lines: string[]): { lines: string[]; did: boolean } {
  // Not needed when using fully-qualified attribute.
  return { lines, did: false };
}

function rewriteTestFnHeader(line: string): { line: string; did: boolean } {
  if (/^\s*#\[tokio::test\]\s*$/.test(line)) {
    return { line: "  #[dps_auth_test_macros::dps_auth_db_test]", did: true };
  }
  return { line, did: false };
}

function shouldDropDbSetupLine(line: string): boolean {
  return (
    /^\s*let\s*\(\s*databases\s*,\s*[_a-zA-Z0-9]+\s*,\s*[_a-zA-Z0-9]+\s*\)\s*=\s*create_test_database(?:_with_[a-zA-Z0-9_]+)?\([^;]*\)\.await\s*;\s*$/.test(
      line,
    ) ||
    /^\s*let\s+pool\s*=\s*databases\.main\(\)\.clone\(\)\s*;\s*$/.test(line)
  );
}

function shouldDropHelperImportLine(line: string): boolean {
  // We'll remove the helper from use-lists; keeping other imports intact.
  return /\bcreate_test_database\b/.test(line) && /^\s*use\s+/.test(line);
}

function removeCreateTestDatabaseFromUse(line: string): { line: string; did: boolean } {
  if (!shouldDropHelperImportLine(line)) return { line, did: false };

  // Conservative edit: remove `create_test_database` and clean up commas/spaces.
  // Handles patterns like: use crate::test_utils::{create_test_database, foo};
  // and: use crate::test_utils::{foo, create_test_database};
  // and: use crate::test_utils::create_test_database;

  if (/^\s*use\s+crate::test_utils::create_test_database\s*;\s*$/.test(line)) {
    return { line: "", did: true };
  }

  if (!/\{/.test(line) || !/\}/.test(line)) return { line, did: false };

  const updated = line
    .replace(/\{\s*create_test_database\s*,\s*/g, "{")
    .replace(/,\s*create_test_database\s*\}/g, "}")
    .replace(/,\s*create_test_database\s*,/g, ",")
    .replace(/\{\s*create_test_database\s*\}/g, "{}")
    .replace(/\{\s*,/g, "{")
    .replace(/,\s*\}/g, "}");

  // If it became an empty brace import, drop the line.
  if (/use\s+crate::test_utils::\{\s*\}\s*;/.test(updated)) {
    return { line: "", did: true };
  }

  return { line: updated, did: updated !== line };
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  const lines = original.split(/\r?\n/);
  let replacements = 0;
  let changed = false;

  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    let line = lines[i];

    const header = rewriteTestFnHeader(line);
    if (header.did) {
      line = header.line;
      replacements++;
      changed = true;
    }

    const useEdit = removeCreateTestDatabaseFromUse(line);
    if (useEdit.did) {
      line = useEdit.line;
      replacements++;
      changed = true;
    }

    if (shouldDropDbSetupLine(line)) {
      replacements++;
      changed = true;
      continue;
    }

    // If we converted the test to macro, we must also drop the old pool binding
    // variants (borrowed pool setup from older script versions).
    if (/^\s*let\s+pool\s*=\s*databases\.main\(\)\s*;\s*$/.test(line)) {
      replacements++;
      changed = true;
      continue;
    }

    out.push(line);
  }

  // Add macro import inside test modules if we replaced any headers.
  if (out.some((l) => /\#\[dps_auth_db_test\]/.test(l))) {
    const res = ensureMacroImport(out);
    if (res.did) {
      replacements++;
      changed = true;
      out.splice(0, out.length, ...res.lines);
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
}

await main();
