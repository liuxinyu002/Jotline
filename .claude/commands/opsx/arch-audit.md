---
name: "OPSX: Arch Audit"
description: Four-phase architecture audit (context gathering, read-only deep-module walkthrough, HTML report, grilling loop) with a four-target write boundary that bootstraps CONTEXT.md / ADR decision records
category: Workflow
tags: [workflow, architecture, audit]
---

Run a four-phase architecture audit of the existing codebase. The audit is diagnosis-only: it never modifies application code, and every refactor it recommends lands through the project's OpenSpec propose entry as a regular change.

**Input**: Optionally accept a focus — a direction, module name, or pain point. With a focus, treat it as the primary scan direction; without one, infer scan priorities from git hotspots.

**Language & host adaptation**: Conduct all interaction and write the report in the language established by the target project's `openspec/config.yaml` conventions and the user's language; this document deliberately hardcodes none. For the Phase 1 walkthrough use the host's read-only subagent capability (if the host offers none, walk the code yourself, still strictly read-only).

## Steps

### 1. Phase 0 — Context gathering (strictly read-only)

Collect four inputs before scanning anything:

1. **Domain vocabulary** — `CONTEXT.md` (repository root). If it exists, read it in; if not, mark it "to be seeded" — it may be lazily created in Phase 3, never in this phase.
2. **Decision base map** — architecture decisions that future proposals must not silently contradict:
   - If `<repo>/adr/` exists: read every ADR; treat `accepted` ADRs that are not superseded by another as in-force.
   - Otherwise: fall back to the `Decisions` sections of archived changes (`openspec/changes/archive/*/design.md`) as a stand-in corpus, and note that no persistent ADRs exist yet.
3. **Avoidance zones** — run `openspec list --json`; every in-progress change defines an avoidance zone. Derive the file areas each one touches (from its proposal/design/tasks) and record them.
4. **Hotspots** — rank files and directories by commit frequency × recency (`git log --name-only` over a reasonable window, e.g. the last six months). These set the scan priority.

A user-supplied focus goes first in the scan priority.

### 2. Phase 1 — Read-only deep-module walkthrough

Dispatch read-only subagents over the scan priority to hunt for architecture friction. Use the vocabulary and criteria below — they are inlined on purpose, so do not look for any external skill.

**Deep-module vocabulary:**

| Term | Meaning | Audit question |
|------|---------|----------------|
| Module | A cohesive unit of code behind an interface | Deep or shallow? |
| Interface | Everything a caller must know: signatures, types, invariants, error modes | Could it be smaller? |
| Depth | A small interface hiding a large amount of value | Does this module earn its interface? |
| Seam | A boundary where alternative implementations could exist without callers caring | Is this seam real? |
| Adapter | One concrete implementation sitting on a seam | Apply the two-adapter criterion |
| Leverage | Value per edit: one place changes, many call sites improve | Is this the highest-leverage intervention point? |
| Locality | Related code lives together, so related changes edit one place | Did extraction improve locality or just scatter the concept? |

**Judgement criteria:**

- **Deletion test** — "If I deleted this abstraction, what breaks, and what gets simpler?" If deletion simplifies things or nobody would notice, the abstraction fails the test and is a merge candidate.
- **Two-adapter criterion** — one adapter on a seam is a hypothetical seam, probably premature; two adapters with real callers make the seam real. Never propose a new seam for hypothetical variation.

**Friction to record** (each finding with file paths and concrete evidence):

- Shallow modules — the interface is nearly as complex as the implementation (pass-through wrappers, parameter soup, pure delegation layers)
- Seam leaks — a boundary exists but callers reach around it (concrete types leaking through, ordering assumptions, side-channel configuration)
- Extraction without locality — a concept scattered across many files in the name of cleanliness, so one logical change edits many places
- Hard-to-test interfaces — seams that force mocking the world instead of substituting an adapter

Phases 0 and 1 never write: `CONTEXT.md`, `adr/`, `openspec/config.yaml`, and all source code are read-only here.

### 3. Phase 2 — HTML report

Write a self-contained report to `docs/audit/<YYYY-MM-DD>_architecture-review.html` (today's date; create the directory if needed). A same-day rerun overwrites the same file — never accumulate multiple same-day reports. Tailwind and Mermaid load from a CDN; the file must open without any repository-local resource.

**Header**: a generated-at anchor of the form `<date> @ commit <sha>` — prefix it with the wording for "generated at" in the report language — plus a note that the report is a point-in-time snapshot, not current truth. Get the short sha via `git rev-parse HEAD`.

**One card per candidate, every field present:**

- **Files involved**
- **Problem** — why this is friction, using the Phase 1 vocabulary
- **Solution** — what to change, at proposal level
- **Benefit** — expressed through locality and leverage
- **Before/after** — a small Mermaid diagram of the shape of the change
- **Strength badge** — exactly one of `Strong` (clear friction, real seam, high leverage) / `Worth exploring` (promising but needs validation) / `Speculative` (hypothetical seam, borderline deletion test)

**Mandatory annotations:**

- A card overlapping an avoidance zone must say so and name the active change it conflicts with.
- A candidate that contradicts an in-force ADR is listed only when the friction is real enough to justify reopening that ADR — and then only with a prominent warning box: "contradicts ADR-NNNN, but worth reopening because …". Otherwise drop it silently.

Close the report with **Top recommendation**: the first candidate to tackle, and why.

Whether the report gets committed is decided by the project's `.gitignore`; the audit neither checks nor cares.

### 4. Phase 3 — Grilling loop (the only writing phase)

Present the candidates as a numbered list and let the user pick (`1`, `1,3`, `all`, …):

```
Architecture candidates:
1. <title> — <files> — <badge>
2. <title> — <files> — <badge>
...
```

For each selected candidate, grill before proposing anything: what breaks if we don't do it; who actually calls this; is the seam real (two-adapter criterion); does it pass the deletion test; what is the load-bearing reason on each side. If grilling coins a new concept or sharpens a vague term, capture it per the rules below.

**Write boundary — the audit's only permissible repository writes, ever.** Anything outside this list, above all refactoring application code, is refused and redirected to the project's OpenSpec propose entry:

| # | Target | Rule |
|---|--------|------|
| 1 | `docs/audit/<date>_architecture-review.html` | The Phase 2 report |
| 2 | `CONTEXT.md` | Lazy creation + incremental vocabulary (below) |
| 3 | `adr/NNNN-kebab-title.md` | Only when the user accepts the ADR proposal (below) |
| 4 | `openspec/config.yaml` | One idempotent `rules.design` line, only when the first ADR lands (below) |

**CONTEXT.md (vocabulary):**

- Owns naming and vocabulary; behavior stays in `openspec/specs/<capability>/spec.md`. The split is fixed — a conflict between the two is a division of labor, not file corruption.
- First creation: seed the glossary with the capability names under `openspec/specs/` (each with a one-line role), then append the new concepts from this audit.
- Incremental: whenever grilling names a concept outside the glossary or sharpens a vague term, append `term — one-line definition`.
- Writes happen only here in Phase 3 — never during Phases 0–2.

**ADR (decisions):**

- Trigger: the user rejects a candidate with a **load-bearing** reason — one a future audit would need so it doesn't re-propose the same thing. A temporary reason ("not worth it right now") is not recorded; do not propose an ADR for it.
- Propose exactly once, e.g.: "Record this reason as an ADR so future audits won't re-propose it?" Write only on acceptance.
- File: `<repo>/adr/NNNN-kebab-title.md`, MADR-short format:

  ```markdown
  # NNNN. <Title>

  ## Status
  accepted — <YYYY-MM-DD>
  Supersedes: ADR-NNNN        <!-- only when replacing an in-force ADR -->

  ## Context
  <the problem and the load-bearing reason>

  ## Decision
  <what was decided>

  ## Consequences
  <what becomes easier, what becomes harder>
  ```

- Numbering: rescan `adr/` immediately before writing; the new number is the current maximum + 1, globally monotonic and never reused.
- Replacement: when the new decision supersedes an in-force ADR, set the new ADR's status to `accepted, supersedes ADR-NNNN` and fill in `Supersedes:`. Never modify the old ADR file.

**config.yaml (ADR reading rule):** when the first ADR of this audit has landed, check `rules.design` in `openspec/config.yaml`:

- If it already contains an ADR-reading rule, or no ADR was written this run, do nothing.
- Otherwise propose appending one line: "If `<repo>/adr/` exists, the design phase must first read the in-force ADRs and stay consistent with them" (worded in the language the existing rules use). Write only on acceptance; every existing entry stays untouched.
- If the active schema has no `design` artifact, skip this mechanism entirely.

**Exit:** a candidate the user wants to pursue lands exclusively as a change proposal through the project's OpenSpec propose entry (`/opsx:propose` where the opsx command set is installed; otherwise the host's equivalent). The audit itself never implements a refactor.

**Change naming when one run proposes multiple changes:** create the changes in the recommended implementation order and prefix each change name with its lowercase position letter (`a-…`, `b-…`, `c-…`; double letters past `z-` if ever needed). The prefix is load-bearing, not cosmetic: OpenSpec rejects digit-leading change names ("must start with a letter"), so letters are the only ordering marker that survives into `openspec list`, and they keep the grilling outcome traceable to the proposals. Grouping several candidates into one change is fine — the letter marks the change's position in the recommended order, not the candidate's number. Conversation-time codes (A/B/C, "candidate 3") are ephemeral presentation only: artifact text MUST reference full change names, never bare letters.

### 5. Wrap-up — optional C-tier schema upgrade (one-time proposal)

After the loop, propose upgrading the project to the `spec-driven-with-adr` schema (the C tier: persistent `<repo>/adr/` decision records, plus a per-change adr artifact that the design phase reads automatically and that tasks depend on) only when **all** of the following hold:

- **Trigger**: this audit landed at least one ADR or recorded at least one load-bearing rejection.
- **Gate**: `openspec list --json` shows no in-progress changes.
- **Schema-aware**: the active schema does not already provide an adr artifact (check the schema's artifact set, e.g. via `openspec status --json` on any active change). If it does, never propose.
- **One-time**: ask at most once per run; a refusal is final — do not raise it again on later runs.

When the gate is not met, do not propose; at most add one line at the report tail saying the upgrade was not proposed and why.

For the actual switch, point the user to the schema's `AGENT_INSTALL.md` installation flow — the audit never switches schemas itself. Existing artifacts migrate seamlessly: ADRs already live in `<repo>/adr/` in MADR-short, which is exactly the C-tier layout, so accumulated files become the decision history as-is.

## Guardrails

- **Four writes, nothing else.** The write-boundary table is exhaustive. Application and source code is never modified by the audit.
- **Read-only until Phase 3.** Even permitted targets are not written during Phases 0–2.
- **Refactors exit through propose.** Diagnosis here, change process there — the audit never implements its own recommendations.
- **No schema switching, no dependencies.** The audit never switches schemas; the report's Tailwind/Mermaid come from a CDN.
- **Numbered-list selection.** All choice points (candidates, scopes, confirmations) are presented as numbered lists.
- **Letter-prefixed change names.** When one run proposes multiple changes, each change name carries its implementation-order letter prefix (`a-…`, `b-…`); artifact text references full change names, never conversation-only letter codes.
- **Silence over noise on ADR conflicts.** Candidates contradicting in-force ADRs are dropped unless reopening is genuinely justified.
- **Report honesty.** Same-day overwrite; snapshot anchored to date + commit; never present a stale report as current state.

## Interaction Flow Summary

```
/opsx:arch-audit [focus]
    │
    ├─ Phase 0  context gathering (read-only)
    │   ├─ CONTEXT.md (vocabulary | "to be seeded")
    │   ├─ adr/ | archived design.md Decisions → decision base map
    │   ├─ openspec list --json → avoidance zones
    │   └─ git hotspots (+ user focus) → scan priority
    │
    ├─ Phase 1  read-only deep-module walkthrough (host subagents)
    │   └─ friction: shallow modules / seam leaks / no locality / hard-to-test
    │
    ├─ Phase 2  HTML report → docs/audit/<date>_architecture-review.html
    │   └─ candidate cards + avoidance/ADR annotations + Top recommendation
    │
    ├─ Phase 3  grilling loop (the only writing phase)
    │   ├─ new/sharpened terms ───▶ CONTEXT.md (lazy create, capability-name seed)
    │   ├─ load-bearing rejection ─▶ propose ADR → adr/NNNN-*.md (MADR-short)
    │   │                             └─ first ADR → propose rules.design line
    │   ├─ pursued candidates ─────▶ propose entry (multiple ⇒ letter-prefixed names a-… b-…)
    │   └─ wrap-up: ADR/rejection + no in-progress + no adr artifact → propose C-tier
    │
    └─ Report: docs/audit/<date>_architecture-review.html
```
