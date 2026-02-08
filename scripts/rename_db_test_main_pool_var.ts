#!/usr/bin/env bun

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import { join } from "node:path";

const ROOT = process.cwd();

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

function isIdentChar(ch: string): boolean {
  return /[A-Za-z0-9_]/.test(ch);
}

function replaceWord(input: string, from: string, to: string): { output: string; replaced: number } {
  let replaced = 0;
  let out = "";
  for (let i = 0; i < input.length; ) {
    if (input.startsWith(from, i)) {
      const before = i === 0 ? "" : input[i - 1];
      const after = i + from.length >= input.length ? "" : input[i + from.length];
      const beforeOk = before === "" || !isIdentChar(before);
      const afterOk = after === "" || !isIdentChar(after);
      if (beforeOk && afterOk) {
        out += to;
        i += from.length;
        replaced++;
        continue;
      }
    }
    out += input[i];
    i++;
  }
  return { output: out, replaced };
}

function replacePoolIdentWithMainPool(input: string): { output: string; replaced: number } {
  const from = "pool";
  const to = "main_pool";
  let replaced = 0;
  let out = "";

  for (let i = 0; i < input.length; ) {
    if (input.startsWith(from, i)) {
      const before = i === 0 ? "" : input[i - 1];
      const after = i + from.length >= input.length ? "" : input[i + from.length];

      const beforeOk = before === "" || (!isIdentChar(before) && before !== ".");
      const afterOk = after === "" || !isIdentChar(after);

      if (beforeOk && afterOk) {
        // Avoid renaming struct field names in literals/patterns: `pool: ...`
        let j = i + from.length;
        while (j < input.length && /\s/.test(input[j])) j++;
        if (j < input.length && input[j] === ":") {
          out += from;
          i += from.length;
          continue;
        }

        out += to;
        i += from.length;
        replaced++;
        continue;
      }
    }

    out += input[i];
    i++;
  }

  return { output: out, replaced };
}

type ScanState =
  | { kind: "normal" }
  | { kind: "line_comment" }
  | { kind: "block_comment"; depth: number }
  | { kind: "string" }
  | { kind: "char" }
  | { kind: "raw_string"; hashes: number };

function parseRawStringHashes(src: string, i: number): number | null {
  // Detect r#" or r##" etc. Returns number of #.
  if (src[i] !== "r") return null;
  let j = i + 1;
  let hashes = 0;
  while (j < src.length && src[j] === "#") {
    hashes++;
    j++;
  }
  if (j < src.length && src[j] === '"') return hashes;
  return null;
}

function findMatchingBrace(src: string, openBraceIndex: number): number {
  let depth = 0;
  let state: ScanState = { kind: "normal" };
  let escaped = false;

  for (let i = openBraceIndex; i < src.length; i++) {
    const ch = src[i];
    const next = i + 1 < src.length ? src[i + 1] : "";

    switch (state.kind) {
      case "normal": {
        if (ch === "/" && next === "/") {
          state = { kind: "line_comment" };
          i++;
          continue;
        }
        if (ch === "/" && next === "*") {
          state = { kind: "block_comment", depth: 1 };
          i++;
          continue;
        }
        const rawHashes = parseRawStringHashes(src, i);
        if (rawHashes !== null) {
          // Skip r###" opener
          i += 1 + rawHashes;
          state = { kind: "raw_string", hashes: rawHashes };
          continue;
        }
        if (ch === '"') {
          state = { kind: "string" };
          escaped = false;
          continue;
        }
        if (ch === "'") {
          state = { kind: "char" };
          escaped = false;
          continue;
        }
        if (ch === "{") {
          depth++;
          continue;
        }
        if (ch === "}") {
          depth--;
          if (depth === 0) return i;
          continue;
        }
        break;
      }
      case "line_comment": {
        if (ch === "\n") state = { kind: "normal" };
        break;
      }
      case "block_comment": {
        if (ch === "/" && next === "*") {
          state.depth++;
          i++;
          continue;
        }
        if (ch === "*" && next === "/") {
          state.depth--;
          i++;
          if (state.depth === 0) state = { kind: "normal" };
          continue;
        }
        break;
      }
      case "string": {
        if (!escaped && ch === "\\") {
          escaped = true;
          continue;
        }
        if (!escaped && ch === '"') {
          state = { kind: "normal" };
          continue;
        }
        escaped = false;
        break;
      }
      case "char": {
        if (!escaped && ch === "\\") {
          escaped = true;
          continue;
        }
        if (!escaped && ch === "'") {
          state = { kind: "normal" };
          continue;
        }
        escaped = false;
        break;
      }
      case "raw_string": {
        if (ch !== '"') break;
        const hashes = state.hashes;
        let ok = true;
        for (let k = 0; k < hashes; k++) {
          if (src[i + 1 + k] !== "#") {
            ok = false;
            break;
          }
        }
        if (ok) {
          i += hashes;
          state = { kind: "normal" };
        }
        break;
      }
    }
  }

  throw new Error("Failed to find matching brace");
}

function rewriteDbTestFunctions(input: string): { output: string; changed: boolean; replaced: number } {
  if (!/#\[\s*dps_auth_db_test\b/.test(input)) return { output: input, changed: false, replaced: 0 };

  let output = input;
  let totalReplaced = 0;

  // Find each occurrence of the attribute and rewrite the following function body.
  const attrRe = /#\[\s*dps_auth_db_test\b[^\]]*\]\s*/g;
  let match: RegExpExecArray | null;

  // Iterate from start; when we rewrite, restart the regex scan to keep indices simple.
  let safety = 0;
  while ((match = attrRe.exec(output)) !== null) {
    safety++;
    if (safety > 5000) throw new Error("Safety limit exceeded while rewriting");

    const afterAttr = match.index + match[0].length;

    // Find the next '{' that opens the function body.
    const openBraceIndex = output.indexOf("{", afterAttr);
    if (openBraceIndex === -1) continue;

    const closeBraceIndex = findMatchingBrace(output, openBraceIndex);
    const body = output.slice(openBraceIndex + 1, closeBraceIndex);

    // Undo accidental renames of struct fields like `.pool` -> `.main_pool`.
    const undoField = body.replace(/\.main_pool\b/g, ".pool");
    const replacedBody = replacePoolIdentWithMainPool(undoField);
    if (replacedBody.replaced === 0 && undoField === body) continue;

    totalReplaced += replacedBody.replaced;
    output =
      output.slice(0, openBraceIndex + 1) + replacedBody.output + output.slice(closeBraceIndex);

    // Reset regex scan because we changed string lengths.
    attrRe.lastIndex = 0;
  }

  return { output, changed: output !== input, replaced: totalReplaced };
}

async function rewriteFile(file: string): Promise<{ file: string; changed: boolean; replaced: number }> {
  const original = await readFile(file, "utf8");
  const res = rewriteDbTestFunctions(original);
  if (!res.changed) return { file, changed: false, replaced: 0 };
  await writeFile(file, res.output, "utf8");
  return { file, changed: true, replaced: res.replaced };
}

async function main() {
  const srcDir = join(ROOT, "src");
  const testsDir = join(ROOT, "tests");
  const files = [
    ...(await listRustFiles(srcDir)),
    ...(await listRustFiles(testsDir).catch(() => [])),
  ];

  let touched = 0;
  let replaced = 0;

  for (const file of files) {
    const res = await rewriteFile(file);
    if (res.changed) {
      touched++;
      replaced += res.replaced;
    }
  }

  process.stdout.write(`Updated ${touched} file(s); renamed ${replaced} occurrence(s) of pool -> main_pool inside #[dps_auth_db_test] functions\n`);
}

await main();
