# Simplify README: analysis and plan

Purpose

- Summarize repetitive blocks in [`README.md`](README.md:1) and propose a simplified structure.

Background

- I reviewed [`docs/llm/instructions.md`](docs/llm/instructions.md:1) and [`README.md`](README.md:1).

Findings

1. Environment variables duplicated between Configuration and Local Development sections.
2. Multiple Usage examples (Minimal / Complete) that largely repeat.
3. Migration binary build/run instructions repeated across Build, Run, Production, and Docker subsections.

Simplification goals

- Centralize environment variables into a single "Environment variables" section.
- Consolidate usage examples: Quickstart + Full example + Variants.
- Condense migration instructions into Quickstart and Reference.
- Remove the "With Logging" usage example from the README.

Proposed README structure

- Title and badges
- Short description
- Features (unchanged)
- Quickstart (minimal commands)
- Usage
  - Full example (one canonical code sample)
  - Variants (programmatic config)
- Environment variables (single canonical list)
- Local development (refer to Quickstart; show only env var differences)
- Database / Migration (build, run, docker notes; link to details)
- API Endpoints and GraphQL ops (concise)
- License

Files to create/modify

- Modify: [`README.md`](README.md:1)
- Create: [`docs/llm/plans/2025/11/16-simplify-readme.md`](docs/llm/plans/2025/11/16-simplify-readme.md:1)

Change plan (detailed steps)

1. Prepare plan file (this file).
2. Draft README edits locally: centralize env vars, consolidate examples, remove the "With Logging" usage example, condense migration.
3. Present diff for review.
4. After approval, apply edits.

Implementation notes

- Do not add dependencies or git operations.
- Keep examples minimal; link to detailed docs for lengthy instructions.

Before/After examples

- Before: environment variables appear in multiple sections (excerpt)

```text
# from [`README.md`](README.md:1)
- `DPS_AUTH_API_SESSION_SECRET`: 32-byte secret for session encryption (required)
...
```

- After: single canonical list under "Environment variables"

```text
Environment variables
- DPS_AUTH_API_PORT: Server port (default: 3000)
- DPS_AUTH_API_SESSION_SECRET: 32-byte secret (required)
...
```


Review checkpoints

- Present README diff for review before applying.
- Ensure plan file saved in [`docs/llm/plans/2025/11/16-simplify-readme.md`](docs/llm/plans/2025/11/16-simplify-readme.md:1).

Requested approval

- Approve plan and proceed to draft README changes, or request adjustments.

End.