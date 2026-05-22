import { readdir, readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

// Renames `conn` -> `main_conn` for Rust identifiers in src/ and tests/.
// Also handles conn1/conn2/... -> main_conn1/main_conn2/... (usually in tests).
// This is a surgical identifier rename (not string literals / comments).

const ROOT_DIR = process.cwd();
const TARGET_DIRS = ["src", "tests"];

async function* walk(dir: string): AsyncGenerator<string> {
  const entries = await readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) {
      yield* walk(p);
    } else {
      yield p;
    }
  }
}

function renameIdentifiers(contents: string): { updated: string; changed: boolean } {
  // Identifier boundaries: Rust identifiers are [A-Za-z_][A-Za-z0-9_]*.
  // We only rename when `conn` is a full identifier (not part of a larger one like `connection`).
  // We also rename `conn<digits>` as an identifier prefix, e.g. conn1 -> main_conn1.
  const reConnDigits = /\bconn(\d+)\b/g;
  const reConn = /\bconn\b/g;

  let updated = contents.replace(reConnDigits, "main_conn$1");
  updated = updated.replace(reConn, "main_conn");

  return { updated, changed: updated !== contents };
}

async function main() {
  let changedFiles = 0;
  let changedOccurrencesApprox = 0;

  for (const dir of TARGET_DIRS) {
    const absDir = join(ROOT_DIR, dir);
    for await (const filePath of walk(absDir)) {
      if (!filePath.endsWith(".rs")) continue;

      const before = await readFile(filePath, "utf8");
      const { updated, changed } = renameIdentifiers(before);
      if (!changed) continue;

      // Approx occurrences: count of the replaced tokens in the before file.
      const matches = before.match(/\bconn(\d+)?\b/g);
      if (matches) changedOccurrencesApprox += matches.length;

      await writeFile(filePath, updated, "utf8");
      changedFiles += 1;
    }
  }

  process.stdout.write(
    `rename_conn_to_main_conn: updated ${changedFiles} file(s), ~${changedOccurrencesApprox} occurrence(s)\n`,
  );
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
