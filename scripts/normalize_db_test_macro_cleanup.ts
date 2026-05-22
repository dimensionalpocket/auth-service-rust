#!/usr/bin/env bun

import { readdir, readFile, writeFile, stat } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();

type Change = {
  file: string;
  changed: boolean;
  replacements: number;
};

const DB_HELPERS = [
  "create_test_databases_with_config_and_pool_size",
  "create_test_databases_with_pool_size",
  "create_test_databases_with_config",
  "create_test_database",
];

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

function parsePoolSizeCall(line: string): { poolSize?: number } {
  const m = line.match(/create_test_databases_with_pool_size\(\s*(\d+)\s*\)\s*\.await\s*;\s*$/);
  if (!m) return {};
  return { poolSize: Number(m[1]) };
}

function parseConfigCall(line: string): { configureSqlite?: boolean } {
  const m = line.match(
    /create_test_databases_with_config\(\s*(true|false)\s*\)\s*\.await\s*;\s*$/,
  );
  if (!m) return {};
  return { configureSqlite: m[1] === "true" };
}

function parseConfigAndPoolSizeCall(
  line: string,
): { configureSqlite?: boolean; poolSize?: number } {
  const m = line.match(
    /create_test_databases_with_config_and_pool_size\(\s*(true|false)\s*,\s*(\d+)\s*\)\s*\.await\s*;\s*$/,
  );
  if (!m) return {};
  return { configureSqlite: m[1] === "true", poolSize: Number(m[2]) };
}

function mergeAttrArgs(
  attrLine: string,
  args: { configureSqlite?: boolean; poolSize?: number },
): string {
  const base = "#[dps_auth_test_macros::dps_auth_db_test";
  if (!attrLine.trim().startsWith(base)) return attrLine;

  const existingArgsMatch = attrLine.match(/\#\[\s*dps_auth_test_macros::dps_auth_db_test\s*\((.*)\)\s*\]/);
  const kv: Record<string, string> = {};

  if (existingArgsMatch) {
    const inner = existingArgsMatch[1].trim();
    if (inner.length > 0) {
      for (const part of inner.split(",")) {
        const p = part.trim();
        const mm = p.match(/^([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(.+)$/);
        if (mm) kv[mm[1]] = mm[2].trim();
      }
    }
  }

  if (typeof args.configureSqlite === "boolean") {
    kv.configure_sqlite = args.configureSqlite ? "true" : "false";
  }
  if (typeof args.poolSize === "number" && !Number.isNaN(args.poolSize)) {
    kv.pool_size = String(args.poolSize);
  }

  const entries: string[] = [];
  if (kv.configure_sqlite) entries.push(`configure_sqlite = ${kv.configure_sqlite}`);
  if (kv.pool_size) entries.push(`pool_size = ${kv.pool_size}`);

  if (entries.length === 0) {
    return "  #[dps_auth_test_macros::dps_auth_db_test]";
  }
  return `  #[dps_auth_test_macros::dps_auth_db_test(${entries.join(", ")})]`;
}

function isDbSetupTupleLine(line: string): boolean {
  return /^\s*let\s*\(\s*databases\s*,\s*[_a-zA-Z0-9]+\s*,\s*[_a-zA-Z0-9]+\s*\)\s*=/.test(
    line,
  );
}

function isDbSetupTupleLineSplitStart(line: string): boolean {
  return /^\s*let\s*\(\s*databases\s*,\s*[_a-zA-Z0-9]+\s*,\s*[_a-zA-Z0-9]+\s*\)\s*=\s*$/.test(
    line,
  );
}

function isDbSetupCallLine(line: string): boolean {
  return DB_HELPERS.some((h) => new RegExp(`\\b${h}\\b`).test(line)) && /\.await\s*;\s*$/.test(line);
}

function shouldDropPoolBindingLine(line: string): boolean {
  return /^\s*let\s+pool\s*=\s*databases\.main\(\)(?:\.clone\(\))?\s*;\s*$/.test(line);
}

function cleanupTestUtilsUseBlocks(content: string): { content: string; replacements: number } {
  const lines = content.split(/\r?\n/);
  let replacements = 0;
  const out: string[] = [];

  let inUse = false;
  let useStartIndex = -1;
  let useBuffer: string[] = [];
  let braceDepth = 0;

  function flushUseBuffer() {
    if (!inUse) return;
    const joined = useBuffer.join("\n");

    // If the import became empty, drop it.
    if (/use\s+[^;]*test_utils::\{\s*\}\s*;/.test(joined)) {
      replacements++;
    } else {
      out.push(...useBuffer);
    }

    inUse = false;
    useStartIndex = -1;
    useBuffer = [];
    braceDepth = 0;
  }

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // Drop simple single-line imports of db helpers.
    if (
      /^\s*use\s+.*::test_utils::create_test_database(?:_with_[a-zA-Z0-9_]+)?\s*;\s*$/.test(line)
    ) {
      replacements++;
      continue;
    }

    if (!inUse) {
      if (/^\s*use\s+.*::test_utils::\{/.test(line)) {
        inUse = true;
        useStartIndex = out.length;
        useBuffer.push(line);
        braceDepth += (line.match(/\{/g) || []).length;
        braceDepth -= (line.match(/\}/g) || []).length;
        if (braceDepth === 0 && /;\s*$/.test(line)) {
          // single-line use block
          // fall through to cleanup
        } else {
          continue;
        }
      } else {
        out.push(line);
        continue;
      }
    }

    // inUse
    if (inUse && useBuffer.length > 0 && useBuffer[useBuffer.length - 1] !== line) {
      // already appended line for this i
    } else if (inUse && useBuffer.length === 0) {
      // shouldn't happen
    } else if (inUse && useBuffer[useBuffer.length - 1] !== line) {
      // shouldn't happen
    }

    if (inUse && useBuffer[useBuffer.length - 1] !== line) {
      useBuffer.push(line);
    }

    // Cleanup helper names within use blocks.
    const lastIdx = useBuffer.length - 1;
    let cleaned = useBuffer[lastIdx];
    for (const h of DB_HELPERS) {
      const re = new RegExp(`\\b${h}\\b\\s*,?\\s*`, "g");
      const before = cleaned;
      cleaned = cleaned.replace(re, "");
      if (cleaned !== before) replacements++;
    }
    // Clean up dangling commas and empty lines.
    cleaned = cleaned
      .replace(/,\s*,/g, ",")
      .replace(/\{\s*,/g, "{")
      .replace(/,\s*\}/g, "}");
    useBuffer[lastIdx] = cleaned;

    braceDepth += (line.match(/\{/g) || []).length;
    braceDepth -= (line.match(/\}/g) || []).length;

    if (/;\s*$/.test(line) && braceDepth <= 0) {
      // normalize `use x::{}` form if it happened
      const joined = useBuffer.join("\n");
      if (/use\s+.*::test_utils::\{[\s,]*\}\s*;/.test(joined)) {
        replacements++;
        // drop entirely
      } else {
        out.push(...useBuffer);
      }
      inUse = false;
      useStartIndex = -1;
      useBuffer = [];
      braceDepth = 0;
    }
  }

  flushUseBuffer();

  return { content: out.join("\n"), replacements };
}

async function rewriteFile(file: string): Promise<Change> {
  const original = await readFile(file, "utf8");
  let replacements = 0;
  let changed = false;

  const lines = original.split(/\r?\n/);
  const out: string[] = [];

  let lastDbAttrIndex: number | null = null;
  let inDbTestFn = false;
  let braceDepth = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    if (!inDbTestFn) {
      if (/^\s*#\[\s*dps_auth_test_macros::dps_auth_db_test\b/.test(line)) {
        lastDbAttrIndex = out.length;
        out.push(line);
        continue;
      }

      out.push(line);

      if (lastDbAttrIndex !== null && /\basync\s+fn\b/.test(line)) {
        // start tracking brace depth once body opens
        const opens = (line.match(/\{/g) || []).length;
        const closes = (line.match(/\}/g) || []).length;
        braceDepth = opens - closes;
        inDbTestFn = braceDepth > 0;
      }

      continue;
    }

    // inDbTestFn
    // Remove pool binding lines (macro injects `pool`).
    if (shouldDropPoolBindingLine(line)) {
      replacements++;
      changed = true;
      continue;
    }

    // Remove db setup tuple lines and infer macro args.
    if (isDbSetupTupleLine(line)) {
      // handle split: `let (...) =` then next line is the call
      if (isDbSetupTupleLineSplitStart(line) && i + 1 < lines.length) {
        const next = lines[i + 1];
        if (isDbSetupCallLine(next)) {
          const args = {
            ...parseConfigAndPoolSizeCall(next),
            ...parsePoolSizeCall(next),
            ...parseConfigCall(next),
          };

          if (lastDbAttrIndex !== null) {
            const old = out[lastDbAttrIndex];
            const updated = mergeAttrArgs(old, {
              configureSqlite: args.configureSqlite,
              poolSize: args.poolSize,
            });
            if (updated !== old) {
              out[lastDbAttrIndex] = updated;
              replacements++;
              changed = true;
            }
          }

          // drop both lines
          replacements++;
          changed = true;
          i++;
          continue;
        }
      }

      // single-line assignment; try to infer args from same line
      const args = {
        ...parseConfigAndPoolSizeCall(line),
        ...parsePoolSizeCall(line),
        ...parseConfigCall(line),
      };

      if (lastDbAttrIndex !== null) {
        const old = out[lastDbAttrIndex];
        const updated = mergeAttrArgs(old, {
          configureSqlite: args.configureSqlite,
          poolSize: args.poolSize,
        });
        if (updated !== old) {
          out[lastDbAttrIndex] = updated;
          replacements++;
          changed = true;
        }
      }

      replacements++;
      changed = true;
      continue;
    }

    out.push(line);

    // update brace depth
    const opens = (line.match(/\{/g) || []).length;
    const closes = (line.match(/\}/g) || []).length;
    braceDepth += opens - closes;
    if (braceDepth <= 0) {
      inDbTestFn = false;
      lastDbAttrIndex = null;
      braceDepth = 0;
    }
  }

  let updated = out.join("\n");
  const cleanedImports = cleanupTestUtilsUseBlocks(updated);
  if (cleanedImports.replacements > 0) {
    replacements += cleanedImports.replacements;
    updated = cleanedImports.content;
    changed = true;
  }

  if (!changed || updated === original) {
    return { file, changed: false, replacements: 0 };
  }

  await writeFile(file, updated, "utf8");
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
