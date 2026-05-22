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

function removeAllMacroImports(input: string): { output: string; removed: number } {
  let removed = 0;
  const output = input.replace(
    /^\s*use\s+dps_auth_test_macros::dps_auth_db_test\s*;\s*\r?\n/gm,
    () => {
      removed++;
      return "";
    },
  );
  return { output, removed };
}

function insertImportIntoTestsModule(input: string): { output: string; inserted: number } {
  let inserted = 0;
  let output = input;

  // Only insert if this file uses the attribute at all.
  if (!/#\[\s*dps_auth_db_test\b/.test(output)) return { output, inserted };

  // Insert into `mod tests { ... }` that is behind cfg(test).
  // We insert right after the opening brace.
  output = output.replace(
    /(#\[\s*cfg\(test\)\s*\]\s*\r?\n\s*mod\s+tests\s*\{)/g,
    (m) => {
      // If it already has the import somewhere inside the tests module, don't insert.
      // (Crude but effective: check if the import occurs after this match position later.)
      return m;
    },
  );

  // Do a more controlled insertion by scanning for each tests module opening.
  const re = /#\[\s*cfg\(test\)\s*\]\s*\r?\n\s*mod\s+tests\s*\{/g;
  let match: RegExpExecArray | null;
  let cursor = 0;
  let rebuilt = "";

  while ((match = re.exec(output)) !== null) {
    const start = match.index;
    const openBraceIndex = output.indexOf("{", start);
    if (openBraceIndex === -1) break;

    // Append everything up to and including '{'
    rebuilt += output.slice(cursor, openBraceIndex + 1);

    // Find module body start for inspection (next char)
    const after = output.slice(openBraceIndex + 1);
    const already = after.includes(MACRO_IMPORT);

    // Only insert if this tests module uses the attribute (some files could have multiple cfg(test) mods)
    // We'll approximate by checking ahead until the next `}` at column 0.
    const nextClose = after.search(/^}\s*$/m);
    const moduleBody = nextClose === -1 ? after : after.slice(0, nextClose);
    const usesMacro = /#\[\s*dps_auth_db_test\b/.test(moduleBody);

    if (usesMacro && !already) {
      rebuilt += `\n  ${MACRO_IMPORT}\n`;
      inserted++;
    }

    cursor = openBraceIndex + 1;
  }

  rebuilt += output.slice(cursor);
  return { output: rebuilt, inserted };
}

async function rewriteFile(file: string): Promise<{ file: string; changed: boolean; removed: number; inserted: number }> {
  const original = await readFile(file, "utf8");
  const removedRes = removeAllMacroImports(original);
  const insertedRes = insertImportIntoTestsModule(removedRes.output);

  const next = insertedRes.output;
  if (next === original) return { file, changed: false, removed: 0, inserted: 0 };
  await writeFile(file, next, "utf8");
  return { file, changed: true, removed: removedRes.removed, inserted: insertedRes.inserted };
}

async function main() {
  const srcDir = join(ROOT, "src");
  const files = await listRustFiles(srcDir);

  let touched = 0;
  let removed = 0;
  let inserted = 0;

  for (const file of files) {
    const res = await rewriteFile(file);
    if (res.changed) touched++;
    removed += res.removed;
    inserted += res.inserted;
  }

  process.stdout.write(
    `Updated ${touched} file(s); removed ${removed} import(s); inserted ${inserted} import(s) into cfg(test) modules\n`,
  );
}

await main();
