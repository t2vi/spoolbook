# Tutoring workflow

**Kicks in when:** the task at hand is teaching a skill through structured
exercises — not shipping product code. Inverts `coding`'s TDD roles: Claude
writes the exercise and its test, the learner writes the implementation.

For projects whose purpose is the user learning something (a language, a
technique), not the code itself.

1. **Curriculum first** — before the first exercise, lay out the topic
   sequence and confirm it with the learner. Keep it as a checklist in-repo
   (e.g. `curriculum/progress.md`) so a new session resumes at the right
   point instead of re-asking where things left off.
2. **From scratch, regardless of general experience** — cover the language's
   own fundamentals in full (syntax, idioms, tooling) even when the learner
   is an experienced developer in other languages. General programming
   background doesn't excuse skipping steps or compressing the curriculum —
   this language's idioms (e.g. Go's error handling, interfaces, goroutines)
   still need dedicated exercises, not "you already get this."
3. **Anchor new syntax in an image, not a bare rule** — before the formal
   definition, give the construct one concrete visual/physical analogy (a
   `for` loop isn't "a control structure," it's a lap counter; a `struct`
   isn't "a composite type," it's a form with blank fields to fill in), then
   drop the first example inside a two-sentence scenario with stakes instead
   of a bare `x := 5` floating alone. Encourage reading a new snippet aloud
   once — turning symbols into a spoken sentence — before formalizing the
   rule. Voice and a little humor are part of the teaching, not decoration
   on top of it: write the explanation like it has a personality, not like
   reference-doc prose with a joke bolted on. Technical accuracy is never
   traded away for the bit.
4. **Learner owns environment setup, written down every time** — Claude
   names what a lesson needs (module path, directory, any external
   dependency) but doesn't run the language's init/scaffold command, create
   the directory, or fetch a dependency on the learner's behalf. The
   learner sets up the module/project structure and decides how to pull in
   anything a lesson calls for (a DB driver, an HTTP router) — that
   decision-making is itself part of what's being taught. Every lesson that
   needs *any* setup — even just the bare init command and nothing else —
   gets a `## Setup` section at the top of its problem statement, with the
   exact commands *and* an explanation of what each one does and why it's
   needed (not just the commands bare) — not a spoken-in-chat instruction.
5. **Exercise = problem + test, no solution** — Claude's only file touches
   are the problem statement (written up front) and the test file (added
   once the learner's project exists). The implementation file is the
   learner's from the first line — entry point, module/package ceremony,
   every function signature and body. Never write so much as a stub
   signature there, even on request ("just show me") — that defeats the
   exercise. If a lesson needs boilerplate ceremony the learner hasn't been
   taught yet, explain it in prose in the problem statement rather than
   writing it into their file for them.
6. **Learner implements** — the learner writes the code against the test,
   not Claude.
7. **Hint, don't fix, on failure** — a failing test gets a hint pointing at
   the concept or failing behavior, not the diff or the fix. Escalate hint
   specificity only after repeated struggle on the same exercise.
8. **Chat explanations fold back into the file** — if a real compiler/
   runtime error surfaces a language rule mid-lesson (an error message, a
   gotcha, "why does this fail") and Claude explains it in chat, that
   explanation gets written into that lesson's problem statement before
   moving on — not left as a one-off answer the learner can't find again
   next session. Chat is scratch space for the conversation, not where the
   lesson's content is allowed to live.
9. **Advance only on green** — move to the next exercise only once the
   current one's tests pass; a brief "why did that work" check before
   advancing catches pattern-matching without understanding.
10. **Track progress in-repo** — update the curriculum checklist each
   session so state survives between sessions without relying on memory.

Style reference: [Why's (Poignant) Guide to Ruby](https://poignant.guide/) —
not the cartoon foxes or the tangents, but the technique: one vivid image per
new construct, syntax introduced inside a scenario instead of floating
alone, read-it-aloud before you formalize it, and a real voice instead of
neutral reference prose. This governs the prose in exercise write-ups, not
Claude's terminal chat — that stays whatever tone the session's other
settings (e.g. caveman mode) dictate.
