---
name: openspec-explain-change
description: Generate a rich, self-contained interactive HTML explanation of an OpenSpec change. Use when the user wants a human-readable walkthrough of what a change did, why, and how it works—after apply or verify, before archive.
license: MIT
compatibility: Requires openspec CLI.
metadata:
  author: openspec
  version: "1.0"
  generatedBy: "1.3.1"
---

Produce a single self-contained HTML page that teaches a reader how a specified OpenSpec change works. Investigate the surrounding system before explaining the diff: the page should make sense to a beginner while still giving an experienced engineer a concise path to the changed behavior.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Output**: `docs/wiki/YYYY-MM-DD-<change-name>/index.html` — a self-contained HTML file with inline CSS and JavaScript. No external fonts, CDNs, images, or network access. The date prefix matches the archive convention: if the change is already archived (in `openspec/changes/archive/YYYY-MM-DD-<name>/`), use that date; otherwise use the current date.

**Position in workflow**: Run after `/opsx:apply` (implementation done) or `/opsx:verify` (implementation verified). Non-mandatory—archive will issue a soft prompt if this hasn't been run.

---

## Steps

### 1. Select the change

If a name is provided, use it. Otherwise:
- Infer from conversation context if the user mentioned a change
- Auto-select if only one active change exists
- If ambiguous, run `openspec list --json` to get available changes and use the **AskUserQuestion tool** to let the user select

Always announce: "Using change: <name>" and how to override (e.g., `/opsx:explain <other>`).

### 2. Load change artifacts

```bash
openspec status --change "<name>" --json
openspec instructions apply --change "<name>" --json
```

Read every file path listed under `contextFiles`:
- `proposal.md` — what & why (narrative backbone for Background section)
- `design.md` — how & key decisions (backbone for Intuition + Code sections)
- `tasks.md` — implementation steps (backbone for Code section ordering)
- `specs/*/spec.md` — requirement contracts (backbone for Code + Quiz sections)

If some artifacts are missing, proceed with what's available and note the gap in the HTML.

### 3. Determine the diff baseline

Auto-infer the code changes belonging to this change. Try strategies in order:

**Strategy A — Merge-base diff against main branch:**
```bash
git merge-base main HEAD   # find common ancestor
git diff $(git merge-base main HEAD)...HEAD --stat
```
If this yields a focused, non-trivial set of files, use it.

**Strategy B — Historical commits touching change-relevant files:**
Extract file paths and keywords from tasks.md and specs. Find commits that touch those files:
```bash
git log --oneline -- <relevant-file-paths>
```
If a coherent set of commits emerges, diff from the earliest relevant commit's parent.

**Strategy C — Working tree diff (uncommitted changes):**
```bash
git diff HEAD --stat
git diff --cached --stat
```
If there are uncommitted changes that match the change's scope, include them.

**Strategy D — User-specified baseline:**
If strategies A–C yield ambiguous or overly broad results, ask the user:
> "I couldn't auto-detect a clean diff baseline for this change. What commit/branch should I diff against?"

**Combine strategies as needed.** The goal is a diff that covers the change's actual code impact without unrelated noise. State the chosen baseline and strategy in the HTML's Background section.

### 4. Explore surrounding code

Trace the old and new paths far enough to explain behavior, not merely file-by-file edits. Prefer checked-in examples and tests over speculation.

For each significantly changed file:
- Read the full current version (not just the diff hunks)
- Find callers and integration points
- Identify data models, contracts, and configuration involved
- Look at tests that exercise the changed behavior

### 5. Build the narrative

Before writing HTML, construct a mental outline:

1. **What problem or constraint motivated the change** — from proposal.md
2. **How the old system behaved** — from codebase investigation (pre-diff state)
3. **The smallest useful mental model of the new behavior** — from design.md + code
4. **How the implementation realizes that model** — from tasks.md + diff walkthrough
5. **Edge cases, trade-offs, and observable consequences** — from specs + code analysis

### 6. Write the HTML file

Determine the date prefix for the output directory:
- If the change is already archived (path matches `openspec/changes/archive/YYYY-MM-DD-<name>/`), extract the date from the archive directory name.
- Otherwise, use the current date in `YYYY-MM-DD` format.

Generate `docs/wiki/YYYY-MM-DD-<change-name>/index.html` with the following required structure. All content in **Chinese** (中文), matching existing `docs/wiki/` documentation language.

Create the directory first:
```bash
mkdir -p docs/wiki/YYYY-MM-DD-<change-name>
```

#### Required page structure

Include a clear title, a short summary, and a table of contents linking to these sections in this order:

1. **背景 (Background)** — Explain only the system needed for the change. Start with an optional beginner-friendly mental model, then narrow to the exact components, contracts, and prior behavior involved.
2. **直觉 (Intuition)** — Explain the core idea before implementation detail. Use small concrete toy inputs and outputs. Show the old and new behavior when comparison makes the change clearer.
3. **代码 (Code)** — Walk through the changes in conceptual groups, ordered by execution or dependency flow rather than arbitrary file order. Include precise file and line references when available, but do not dump the whole diff.
4. **测验 (Quiz)** — Include exactly five medium-difficulty, interactive multiple-choice questions. Clicking an option must immediately show whether it is correct and explain why, including the relevant behavior or code path.

Use smooth transitions, plain language, and precise systems-oriented prose. Explain jargon on first use. Use callouts for definitions, invariants, important edge cases, and practical consequences. Keep the page readable on phones with responsive CSS. Do not use top-level tabs; make it one continuous page.

#### Diagrams and examples

Use a small, reusable set of HTML/CSS diagram patterns rather than ornamental graphics:

- flow diagrams for requests, data, or control flow;
- before/after panels for changed behavior;
- labeled component cards for system boundaries;
- compact tables for mappings, invariants, and toy data.

**Never use ASCII diagrams.** Build diagrams with semantic HTML elements and CSS. Label arrows and include example values whenever the diagram describes data movement. Add accessible text or a caption so the explanation does not depend on visual inspection alone.

#### Quiz quality rules

Treat quiz design as part of the explanation, not decoration. Before emitting the page, inspect all five questions as a set.

- Randomize the option order independently for each question. Do not always place the correct answer first, second, or in any fixed position. A deterministic shuffle with a per-page seed is acceptable; the visible order must vary across questions.
- Balance correct-answer positions across the five questions as evenly as possible. Never let position, letter, punctuation, or a repeated pattern reveal the answer.
- Keep options comparable in length, grammar, specificity, and confidence. Do not make the correct option conspicuously longer, more qualified, or more technically precise than distractors. Shorten or enrich distractors as needed.
- Make every distractor plausible and tied to a real misunderstanding of the change. Avoid joke answers, obviously impossible claims, "all/none of the above," and trivia that cannot be inferred from the page.
- Ask about behavior, causality, contracts, edge cases, or trade-offs. Avoid questions whose answer can be guessed from a single copied phrase.
- Keep the correct answer and explanation in the page's JavaScript data or DOM so the interaction works offline. Reveal feedback only after selection. Mark the selected option and explain both the right reasoning and, when useful, the misconception behind the distractors.
- Ensure the UI does not expose the answer through styling before selection, DOM labels, `title` attributes, source ordering, or accessibility text. Accessibility labels should describe the option, not its correctness.

#### HTML and code-block constraints

- Escape user/code-derived text for HTML and JavaScript contexts. Preserve meaningful whitespace in code examples.
- Use `<pre><code>...</code></pre>` for code blocks. The CSS for `pre` must explicitly include `white-space: pre` or `white-space: pre-wrap`; verify every code block in the saved source before delivery.
- Keep JavaScript small, namespaced, and dependency-free. Use event listeners rather than inline handlers when convenient, and handle repeated quiz cards without relying on fragile global selectors.
- Include visible focus states and sufficient color contrast. Do not make correctness depend on color alone.
- Avoid claiming behavior that the inspected source does not support. Distinguish observed facts from reasonable interpretation.

### 7. Validate the artifact

Before handing off, verify:
- File exists at `docs/wiki/YYYY-MM-DD-<change-name>/index.html`
- Is a complete HTML document (`<!DOCTYPE html>` through `</html>`)
- Contains no external asset dependencies (no `src=`, `href=` pointing to CDNs, external fonts, or remote images)
- Has working quiz interactions (5 questions, each with selectable options and feedback)
- Every `<pre>` has `white-space: pre` or `white-space: pre-wrap` in its CSS

If practical, open it in a browser or use a local HTML inspection tool to catch layout or JavaScript errors.

### 8. Display summary

```
## Explanation Generated

**Change:** <change-name>
**Output:** docs/wiki/YYYY-MM-DD-<change-name>/index.html
**Diff baseline:** <strategy used> — <commit/branch description>
**Artifacts read:** proposal, design, tasks, specs (list which were available)
**Files inspected:** <count> files in diff, <count> surrounding files traced

Open in browser: file://<absolute-path-to-docs/wiki/YYYY-MM-DD-<change-name>/index.html>
```

---

## Diff baseline inference details

The diff baseline is the trickiest part. OpenSpec does not track which commits belong to a change. Use this decision tree:

```
                    ┌─ git merge-base main HEAD
                    │  yields focused diff?
                    │
              ┌─────┴─────┐
              │   YES     │   NO
              ▼           ▼
         Use merge-base   Extract file paths from
         diff as baseline  tasks.md + specs
                          │
              ┌───────────┴───────────┐
              │                       │
         git log -- <paths>     git diff HEAD
         yields coherent         (working tree)
         commit set?             has matching changes?
              │                       │
         ┌────┴────┐             ┌────┴────┐
         YES       NO            YES       NO
         │         │             │         │
    Use earliest   ──────────────┴──── Ask user to
    commit parent  (combine A/B/C)    specify baseline
```

**State the chosen strategy** in the HTML Background section so readers understand what diff they're looking at.

---

## Guardrails

- **Self-contained only.** No external dependencies. The HTML must work fully offline.
- **No ASCII diagrams.** All diagrams must be HTML/CSS.
- **Chinese content.** The HTML page content is in Chinese. The skill spec itself is in English (consistent with other skills).
- **Quiz is mandatory.** Exactly 5 questions with the quality rules above. Do not skip or reduce.
- **Ground in reality.** Every claim about behavior must trace back to inspected source code. Distinguish observed facts from interpretation.
- **Don't dump the diff.** Walk through changes conceptually, not file-by-file hunk-by-hunk.
- **Escalate baseline ambiguity.** If auto-inference fails and the user can't specify a baseline, stop and explain the problem rather than guessing.
- **Non-blocking.** This skill never modifies code or change artifacts. It only reads and produces the HTML file.

---

## Fluid Workflow Integration

This skill supports the "actions on a change" model:

- **Can be invoked anytime after apply**: Whether tasks are all done or partially done, the explanation can be generated. If partial, note which tasks remain.
- **Can be invoked after verify**: If verification found issues, the explanation can still be generated but should note the verification status.
- **Can be re-run**: If implementation changed after a previous explanation, re-running overwrites the HTML with an updated version.
- **Archive soft prompt**: When archive runs, it checks if any `docs/wiki/*-<change-name>/index.html` exists (glob match, since the date prefix may differ). If not, it suggests running `/opsx:explain` but does not block.
