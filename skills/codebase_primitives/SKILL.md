---
name: codebase primitives
description: "Study a codebase for logical or code duplication and for whether its primitives still describe the underlying problem space directly."
---

# codebase primitives

Study our codebase for any sign of logical or code duplication. Good starting point is running what we have configured as the `code_duplication` github action, but it will only capture obvious cases. Go through the logical and semantic sides as well. We need our codebase as lean and simple as the underlying problem space allows for.

While you're learning about the codebase and the meaning of all primitives (do use /graphify), keep an eye out on if any of the doc files we have here are outdated and need updating. `docs.sh` (next to this file) lists them: `docs/ARCHITECTURE.md` and `docs/spec/` shape the entire project, the rest are per-crate. Update the ones that drifted — that is the only thing you edit.

Your goal is to learn about the current fundamental outline of the codebase, internalize the problem space underneath it and the fundamental axis underlying it, unify your understanding of how we're solving it by measuring existing methods on how they're expressing them, and then figure out if anything is missing or if there are more direct ways of describing some fundamental primitives.

Disregard difficulty of changes — if we can use the typesystem to describe the underlying space as closely as possible, there is no way for us not to figure out a way later to do procedural compilation of whatever optimizations could be added around it. Final goals are obviously expressiveness and performance, but we get to both by rendering the underlying problem space with the type system — keep that in mind.

Report the findings; you do not act on them.
