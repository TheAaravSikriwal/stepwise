# Status

_Last updated: 2026-09-27_

## Where we are

**Stepwise works end to end.** Write a program, press Run, and step through it forwards and
backwards, with a plain-English explanation of every step.

- **Compiler** (Rust → WebAssembly): lexer, parser, checker and codegen with trace instrumentation.
  Friendly error messages before anything runs (15 of them are snapshot-tested).
- **Runtime:** runs in a Web Worker, with a limit on runaway programs and plain-English runtime
  errors (division by zero, recursion that never stops).
- **Replay engine:** snapshots every 1,000 steps; any seek, in either direction, loads the nearest
  snapshot and replays forward. Fast on a million-event run.
- **Playground:** black, white and gold (dark and light), nothing boxed in, code coloured by kind. A "What just
  happened" explainer for every step, "where from?" on every variable, the 3D call galaxy, variables with change highlights, functions running, output synced to the step,
  step / skip over / finish function / stops in both directions, a timeline, the example gallery,
  share links, and the language and "How it works" pages.

**Tests:** everything is required in CI.
- Rust (`cargo test --workspace`): lexer 29, parser 47, checker 59, error messages 15, codegen
  (7 exact traces and 23 golden programs, including the "tour" program), the 10 gallery examples,
  and trace-consistency checks on every run.
- Web (`npm test` in `web/`): 93 tests, including the replay engine (18 correctness, 4 speed, 5
  "where from?"), the call galaxy's tree, layout and hover details (11), the explainer, step navigation, the
  highlighter, share links, and all 23 golden programs again in the browser runtime.

**Checked by hand in the browser** (2026-09-27): all 10 gallery examples run and step both ways;
the tour program gives exactly its expected output in 137 steps; the explainer was read at every
one of those steps (which found and fixed two bugs); step over, step out and stops work on the tour;
typos, division by zero, endless loops and endless recursion all stop cleanly with clear messages.

**v0.2 checked in the browser** (2026-09-27): dark, light and the ~960px frame. "Where from?" on
the tour (`total` → line 93; recursive `n` → "called factorial and passed in n = 3"). The galaxy on
factorial, fibonacci and fib(16) (3,194 calls, 1,194 folded, 4 ms per step): the layout, labels,
hover, click-to-jump and theme switching.

## Releasing on wearechintu.com

**Live since 2026-09-28** at https://wearechintu.com/stepwise (Stepwise 51b5e06, site commit
f01f55b). The portfolio line reads "Stepwise: a language that runs backward".

Stepwise ships as part of the site, the way the site itself is released (Vercel: every pushed
branch gets a preview, and merging to `main` publishes). **Nothing is published until you've tested
it and said so.**

Ready: the site's local branch **`stepwise`** in the worktree
`D:\VisualStudioProjects\gitbuddywebsite-stepwise` (not pushed). It has the portfolio line
"Stepwise: a debugger that runs in reverse", the Stepwise side, and `/stepwise` with the framed
playground under its own security policy.

To release:
1. **Test the app yourself:** `npm run dev` in `web/`, then http://localhost:5173.
2. **Export into the site:** `npm run export-site -- D:\VisualStudioProjects\gitbuddywebsite-stepwise`
   in `web/` (it runs every test first).
3. **Test inside the site locally:** `npm run dev` in the site worktree, then open
   http://localhost:3000/stepwise.
4. **Commit** `public/stepwise-app/` on the site's `stepwise` branch and **push it**. Vercel builds a
   preview URL, and you test there: that's the real production setup.
5. **Merge to `main`** when you're happy. That publishes it.

For later updates, repeat steps 2–5.

## Open questions

- **Social preview image:** once the final URL is live.

## Notes

- Smart App Control was turned off on 2026-09-27 (it blocked rustc's own DLLs).
- GitHub: https://github.com/TheAaravSikriwal/stepwise (public). Pushes use
  `git -c credential.helper= -c "credential.helper=!gh auth git-credential" push`, or run
  `gh auth setup-git` once to make plain `git push` work.
- Later (v1.0): arrays and strings touch every stage, and "where did this value come from?"
