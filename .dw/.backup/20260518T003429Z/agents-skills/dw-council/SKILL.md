---
name: dw-council
description: |
  Orchestrates a multi-advisor debate (3-5 archetypes) to stress-test
  high-stakes product, architecture, or scope decisions. Uses parallel
  subagents with steel-manning, concession tracking, and dissent-preserving
  synthesis. Invoked opt-in via `--council` flag from dw-brainstorm and
  dw-create-techspec, or standalone when the user explicitly needs a
  rigorous debate. Do not use for small decisions, routine implementation,
  or when a single answer is already obvious.
allowed-tools:
  - Read
  - Task
  - Write
---

# dw-council — Multi-Advisor Debate

A real embedded subagent workflow — not inline roleplay. Each archetype is dispatched as an independent subagent with the archetype's priorities and debate protocol.

## When to Use

- High-impact product, architecture, or scope decisions with real trade-offs
- Stress-testing a techspec proposal before committing
- Comparing multiple viable options where stakeholder priorities differ
- Preserving dissent instead of collapsing tension into false consensus

## When NOT to Use

- Low-stakes or obviously-reversible decisions (a council is expensive; reserve for meaningful debates)
- Decisions already covered by an existing ADR
- When a single specialized skill suffices (e.g., `security-review` for auth concerns, `ui-ux-pro-max` for visual direction)

## Required Inputs

- **Dilemma** (from caller or user): a refined problem statement with explicit constraints and the 2+ candidate paths
- **Context**: links to PRD/TechSpec/task files, rules, prior ADRs
- **Roster size**: 3, 4, or 5 advisors (see selection rules below)

## Archetype Roster

The five archetypes live in `agents/*.md` (relative to this SKILL.md). Each is an independent subagent persona. Dispatch by id:

| Id | Focus |
|----|-------|
| `pragmatic-engineer` | Delivery, maintenance, boring tech that ships |
| `architect-advisor` | Boundaries, coupling/cohesion, compounding debt |
| `security-advocate` | Threat model, attack surface, blast radius |
| `product-mind` | User impact, business value, opportunity cost |
| `devils-advocate` | Stress-tester, hidden assumptions, edge cases |

## Roster Selection

- **3 advisors** for binary choices or a narrow trade-off axis
- **4 advisors** for multi-factor decisions with 2-3 competing concerns
- **5 advisors** for broad, multi-faceted dilemmas

Rules:
- Always include `devils-advocate` when consensus forms quickly or the user explicitly asks for stress-testing
- Prefer the smallest roster that still produces real tension
- Match archetypes to the dilemma: security decision → include `security-advocate`; product scope → include `product-mind`; etc.

## Phase 1: Opening Statements (parallel dispatch)

Dispatch all selected advisors **in parallel** using the Task/Agent tool. Each receives:

1. The refined dilemma and explicit constraints
2. The roster of other advisors in the session
3. The instruction: *"Deliver your opening statement (2-3 paragraphs) ending with a one-line **Key Point**. Argue from your archetype's real priorities defined in `agents/<your-id>.md`."*

Render as:

```markdown
## Opening Statements

### [Advisor Name] — [Archetype]
[Opening statement]

**Key Point:** [one-line summary]
```

## Phase 2: Tensions and Rebuttals

Read the openings and identify 2-4 **genuine tensions** (Side A, Side B, meaningful stakes — not cosmetic disagreements).

For each tension, re-dispatch the two opposing advisors (can be parallel within a tension, sequential across tensions) with this prompt:

```
Steel-man [opponent]'s position in 1-2 sentences, then deliver your rebuttal
in 1 paragraph. State whether you concede, partially concede, or hold firm,
and why. Reference your priorities from agents/<your-id>.md.
```

Record as:

```markdown
## Core Tensions

| Tension | Side A (Advisor) | Side B (Advisor) | Facilitator Note |
|---------|------------------|------------------|------------------|
| ...     | ...              | ...              | ...              |

### Key Concessions
- **[Advisor A]** concedes to **[Advisor B]** on [point] because [reason]
- **[Advisor C]** holds firm on [point] because [reason]
```

## Phase 3: Position Evolution

```markdown
## Position Evolution

| Advisor | Initial Position | Final Position | Changed? |
|---------|------------------|----------------|----------|
| ...     | ...              | ...            | Yes/No   |

**Key Shifts:**
- [Who changed and why]
```

## Phase 4: Synthesis

```markdown
## Council Synthesis

### Points of Consensus
- ...

### Unresolved Tensions
| Tension | Position A | Position B | Trade-off |
|---------|------------|------------|-----------|
| ...     | ...        | ...        | ...       |

### Recommended Path Forward
**Primary Recommendation:** ...

**Rationale:** ...

**Dissenting View:** [preserve the loudest holdout — do not flatten to consensus]

### Risk Mitigation
- ...

### What Would Change the Recommendation
- [condition X would flip the recommendation because ...]
```

## Output Location

- **Embedded mode** (invoked by `/dw-brainstorm --council` or `/dw-create-techspec --council`): return the synthesis inline; the caller extracts what it needs for the parent artifact (PRD, techspec, ADR).
- **Standalone mode**: save to `.dw/spec/<prd-slug>/council-YYYYMMDD.md` (if a PRD is active) or present inline if no PRD context exists. If the decision warrants a permanent record, suggest `/dw-adr` as the next step.

## Debate Protocols (non-negotiable)

- **Steel-man first**: every rebuttal starts with the strongest version of the opposing case
- **Evidence required**: no bare assertions — point to the PRD, code, rule, or prior decision
- **No false consensus**: preserve live disagreement in the synthesis
- **Authentic voices**: each advisor argues from its real priorities, not a diplomatic middle
- **Concession discipline**: if an advisor moves, record what changed their mind

## Failure Handling

- If an advisor returns out-of-character content, re-dispatch once with a protocol reminder
- If the failure repeats, note it in the synthesis and proceed with remaining advisors
- If fewer than 2 real tensions emerge: the dilemma may be lower-stakes than expected. Note that and produce a condensed synthesis. **This is a signal that a council may have been overkill.**

## Integration With Other dw-* Commands

- **`/dw-brainstorm --council`** (opt-in): invokes the council after the normal brainstorm to stress-test the top 2-3 options before recommending
- **`/dw-create-techspec --council`** (opt-in): invokes the council on the primary architectural decision of the techspec before finalizing
- **Standalone** `/dw-council "<dilemma>"` (if registered as a command — currently this is a bundled skill invoked by the two above; it can be promoted to a command in a future release if direct usage becomes common)

The `--council` flag is **additive**: omitting it produces the normal brainstorm/techspec flow. Including it adds a debate section to the output.

## Inspired by

Ported from Compozy's `cy-idea-factory` council subsystem:

- Protocol: `/tmp/compozy/.agents/skills/cy-idea-factory/references/council.md`
- Archetypes: `/tmp/compozy/extensions/cy-idea-factory/agents/*/AGENT.md`

Adaptations for dev-workflow:

- Five-archetype roster (dropped `the-thinker` to keep the roster lean for the initial release; can be added later)
- No dependency on a host `run_agent` registry: archetypes are plain markdown files that the orchestrator reads and passes to subagents via the Task/Agent tool
- Output paths use `.dw/spec/<prd>/council-*.md` instead of `.compozy/tasks/<name>/`
- Integration points map to dev-workflow commands (`dw-brainstorm`, `dw-create-techspec`) instead of Compozy's idea-factory pipeline

Credit: Compozy project (https://github.com/compozy/compozy).
