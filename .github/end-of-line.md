# End of Line

**A session-ending protocol for AI coding assistants.**

Every session with an AI assistant ends the same way: you close the tab and everything it learned about you, your codebase, and its own mistakes evaporates. Tomorrow it makes the same mistakes.

This file fixes that. Drop it into a repo and ask your agent to set it up.

**What it needs:** a git repo, an assistant that reads a project instruction file, and permission to write files. Setup takes about a minute. The protocol runs at the end of a session and takes two.

---

## Quickstart

Paste this to your agent at the end of your next session:

> Read `.github/end-of-line.md` in this repo and set it up. Then run it on this session.

Everything below is what your agent will do. You do not need to read it.

---

## For the agent: set this up

Configure the End of Line protocol in this workspace. Do this once.

**Before you start:** if `.tracking/friction.jsonl` already exists, setup has already run. Report that and stop. Do not repeat it.

### 1. Find the instruction file

Check for these, in order:

| Assistant | File |
|---|---|
| GitHub Copilot | `.github/copilot-instructions.md` |
| Claude Code | `CLAUDE.md` |
| Cursor | `.cursor/rules/` **(a directory — create `end-of-line.mdc` inside it)** |
| Anything else | `AGENTS.md` |

If **none** exists, create `AGENTS.md`.
If **more than one** exists, ask the user which their assistant actually reads. Do not guess, and do not edit all of them.

### 2. Check who else this affects

These files are usually **committed and shared with the team**. Adding the block below grants standing self-edit permission to every teammate's assistant, not just the user's.

Ask the user which they want:

- **Shared** — commit it. The whole team gets the protocol. Appropriate for a team that has agreed to this.
- **Personal** — add the file to `.git/info/exclude` so it stays local and uncommitted. Appropriate for trying it alone first.

Do not decide this yourself.

### 3. Add the trigger

Append to the chosen file:

```markdown
## End of Line

When the user says **"end of line"**, run the protocol in `.github/end-of-line.md`.
Read that file in full first.

You are ALLOWED to edit this instructions file as part of that protocol,
and only as part of that protocol. Show the user the diff before saving.
```

That permission sentence is the unlock. Without it the assistant describes changes instead of making them.

### 4. Create the tracking files

```
.tracking/friction.jsonl     append-only, one JSON object per line
.tracking/sessions/          one markdown record per session
```

If `.tracking/` is already taken, use `.eol/` and say so.

Never rewrite `friction.jsonl`. Append only. The history is the entire value.

### 5. Confirm and stop

Report which file you modified, whether it is shared or personal, and what you created. **Do not run the protocol yet** unless the user asked you to.

---

## The protocol

Runs when the user says the trigger phrase. Four steps, in order.

> **If you cannot complete a step, say so and name the step.** Never report "clean session" or "no friction" for a step you were unable to perform. A review that silently passes when it is broken is worse than no review, and it will stay broken for weeks before anyone notices.

### Step 1 — Friction analysis

**Friction is what the human had to say to steer you.**

**First, read `.tracking/friction.jsonl`** and list the `category` values already in use. You need them before you write anything.

Now review this session for every message where the user corrected, redirected, or repeated themselves. Ignore genuinely new questions. You want the moments they had to push back.

For each one:

1. Pick the category. **Reuse an existing slug verbatim if one fits.** Only invent a new slug if nothing matches. Drifting slugs are the most common way this protocol quietly stops working. Starter vocabulary: `wrong-source`, `wrong-tool`, `premature-conclusion`, `missed-existing-rule`, `repeated-request`, `unverified-claim`.
2. Answer: what instruction would have prevented this?
3. Append one line:

```json
{"ts":"2026-07-28T14:02:00Z","category":"premature-conclusion","correction":"user asked 'have you looked at the PRs?'","rule":"enumerate merged PRs before naming a root cause"}
```

**Then count.** For each category you touched, count its total occurrences in the file, including today.

- **Fewer than 3** → log only. Write no rule. Say which categories are sitting at 1 or 2.
- **3 or more** → propose exactly one rule for that category, show the diff, and let the user approve before saving.

One bad session is a bad day, not a pattern. A rulebook that grows on every mistake becomes noise nobody loads.

### Step 2 — Toil analysis

**Toil is what you wasted without being told.**

Review your tool calls this session. A call is wasted if it errored, returned empty or zero rows, was immediately reissued with a corrected shape, or fetched something you already had.

Report `wasted / total`. **If you cannot enumerate your own calls reliably, say that instead of estimating.** A fabricated ratio is worse than no ratio.

> **Empty is not success.** A query returning zero rows does not error. Nothing goes red. It still cost a round trip and taught you nothing. Sessions dominated by empty results routinely score "clean" and are the least clean sessions you will have.

For each recurring miss, append the **verified** call sequence and the specific trap to a `## Verified recipes` section in the same instruction file you edited during setup. Put it where the work happens, not in a notes file nobody opens.

**Toil has no threshold. Fix it this session.**

| | Friction | Toil |
|---|---|---|
| What it is | a preference | a fact |
| Why wait | they may just have caught you on a bad day | a wrong table name is wrong every time |
| Threshold | three strikes | none |

### Step 3 — Write the record

Create `.tracking/sessions/YYYY-MM-DD-short-topic.md`:

```markdown
# <what this session was about>

**Date:** YYYY-MM-DD

## What was investigated and why
## What was found
## Decisions made, and what was rejected
## Artifacts (PRs, commits, files changed)
## Still open
```

A record of what happened, not a to-do list. Write it for someone with no memory of the session, because in a week that is you.

> ⚠️ **Scrub before you save.** This summarises a conversation that may contain tokens, connection strings, internal URLs, customer names, or unreleased detail. Remove them. If you are unsure whether the repo is safe for any of it, add `.tracking/sessions/` to `.gitignore` and tell the user why.

### Step 4 — Commit

If this is a git repo, stage **explicitly by path**: the friction log, the session record, and any instruction file you edited. Never `git add -A` — in an active repo that sweeps up unrelated work in progress.

One commit. **Do not push, and do not commit to a protected branch** unless the user says to. If it is not a git repo, skip this step and say so. Do not run `git init`.

---

## Where a lesson lives

Writing a lesson down is not the same as wiring it in. Put it on the wrong shelf and it may as well not exist.

| Scope | Where it goes | What belongs there |
|---|---|---|
| **Everywhere** | your assistant's user or global settings, outside the repo | preferences that follow you. "Always hyperlink a PR." "Never ask permission for read-only work." |
| **This repo** | the project instruction file you edited during setup | hard-won specifics. Which table holds the error text. Which API silently returns nothing. |
| **This task** | nowhere. Let it go. | working state. Deliberately disposable. |

Repo scope is the highest-value and the least obvious. A specific table name is worthless in any other codebase, which is exactly why it does not belong with your preferences.

**Before writing any new rule, search for an existing one.** Grep the instruction file and any memory or rules directory for the same subject.

- **Nothing found** → write it. Real gap.
- **Something found** → the failure is *loading*, not knowledge. Do not write a second note saying the same thing. Add a pointer to it from the place you actually looked first. Never delete the original.

Most repeat failures are the second kind, and writing another note is the reflex that hides them.

---

## Two rules worth keeping

**No self-granted exceptions.** Given an expensive required step, an assistant will produce a reasoned case, in complete sentences, for why *this particular session* is exempt. The argument will be good. That is the problem. Only the human waives a step.

**Every finding gets an action.** For each item, either make the edit now or write `Skip — reason`. A list of findings with no edits is a failed review.

---

## Once it is running

Three additions that become obvious after a handful of sessions:

- **Routing rules.** Every time you reach for the wrong tool and then find the right one, record the pair.
- **Extracted workflows.** A multi-step procedure done three times is a named procedure. Write it once, reference it after.
- **An adversarial reviewer.** Before an instruction edit lands, have a second agent argue against it: over-fitting, scope creep, duplicating a rule that already exists. Let it say no. Then you decide.

Start with Step 3. Just have your agent write down what happened. Everything else is that, compounding.

---

*Shared from the End of Line protocol · Windows Engineering Systems · AI Day, July 2026*
