---
name: "OPSX: Explain"
description: Generate a human-readable HTML explanation of a change (after apply or verify)
category: Workflow
tags: [workflow, explain, documentation, experimental]
---

Generate a rich, self-contained interactive HTML explanation of an OpenSpec change. Produces a teaching document at `docs/wiki/YYYY-MM-DD-<change-name>/index.html` that explains what the change did, why, and how it works. The date prefix matches the archive convention: if the change is already archived, use that date; otherwise use the current date.

**Input**: Optionally specify a change name after `/opsx:explain` (e.g., `/opsx:explain add-auth`). If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**When to use**: After `/opsx:apply` (implementation done) or `/opsx:verify` (implementation verified). Non-mandatory—`/opsx:archive` will issue a soft prompt if this hasn't been run.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - Infer from conversation context if the user mentioned a change
   - Auto-select if only one active change exists
   - If ambiguous, run `openspec list --json` to get available changes and use the **AskUserQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/opsx:explain <other>`).

2. **Load change artifacts**

   ```bash
   openspec status --change "<name>" --json
   openspec instructions apply --change "<name>" --json
   ```

   Read every file path listed under `contextFiles` (proposal, design, tasks, specs).

3. **Determine the diff baseline**

   Auto-infer the code changes belonging to this change. Try strategies in order:

   - **A — Merge-base diff**: `git diff $(git merge-base main HEAD)...HEAD`
   - **B — Historical commits**: Extract file paths from tasks/specs, `git log -- <paths>`, diff from earliest relevant commit's parent
   - **C — Working tree diff**: `git diff HEAD` for uncommitted changes matching the change's scope
   - **D — User-specified**: If A–C are ambiguous, ask the user for a baseline commit/branch

   Combine strategies as needed. State the chosen baseline and strategy in the HTML.

4. **Explore surrounding code**

   For each significantly changed file: read the full current version, find callers, identify data models and contracts, look at tests.

5. **Write the HTML file**

   Determine the date prefix: if the change is already archived (in `openspec/changes/archive/YYYY-MM-DD-<name>/`), use that date; otherwise use the current date.

   Output to `docs/wiki/YYYY-MM-DD-<change-name>/index.html`:
   - Self-contained: inline CSS and JS, no external dependencies
   - Chinese content
   - Required sections: 背景 (Background), 直觉 (Intuition), 代码 (Code), 测验 (Quiz)
   - Diagrams: HTML/CSS only, never ASCII
   - Quiz: exactly 5 interactive multiple-choice questions with randomized options, balanced answer positions, plausible distractors

   See the `openspec-explain-change` skill for full HTML structure and quiz quality rules.

6. **Validate the artifact**

   Verify: file exists, complete HTML document, no external dependencies, 5 working quiz interactions, `<pre>` has `white-space: pre` or `pre-wrap`.

7. **Display summary**

   ```
   ## Explanation Generated

**Change:** <change-name>
**Output:** docs/wiki/YYYY-MM-DD-<change-name>/index.html
**Diff baseline:** <strategy> — <description>

   Open in browser: file://<absolute-path>
   ```

**Guardrails**
- Self-contained HTML only — no external fonts, CDNs, images, or network access
- No ASCII diagrams — all diagrams must be HTML/CSS
- Chinese content for the HTML page
- Quiz is mandatory — exactly 5 questions with quality rules
- Ground in reality — every claim traces to inspected source
- Don't dump the diff — walk through conceptually
- Non-blocking — never modifies code or change artifacts
