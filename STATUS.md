# Status

_Last updated: 2026-09-27_

## Where we are

**All the plumbing for the MVP and most of 1.0 is built and tested.** What's left is the core,
which is yours: every piece has a spec in `docs/specs/` and a failing test suite waiting for it.

Working now:
- **Playground:** CodeMirror editor with highlighting, error underlines, Ctrl+Enter to run.
  Programs compile and run in a Web Worker, with a million-event limit, friendly runtime errors,
  and real messages if the compiler panics.
- **Debugger UI:** call stack, variables with change highlights, output synced to the step, a
  timeline, step / over / out / breakpoints in both directions, keyboard shortcuts. Preview it at
  `http://localhost:5173/?demo`. With real programs it switches on as soon as `createReplay` works.
- **Example gallery** (10 programs), **shareable links**, and **language reference** and **How it
  works** pages.
- **Staged pipeline:** `PIPELINE` in `compiler/src/lib.rs` lets the playground use your real stages
  one at a time. The rest is a stub that always prints 42.
- **CI** on every push: fmt, clippy, all finished tests, the wasm build, the web tests, the site
  build. Tests for pieces still in progress run and report, but don't fail the build.

## Next: your track, in order

1. **Lexer:** read `docs/specs/lexer.md`, then make `cargo test --test lexer` pass (29 tests).
2. **Parser:** read `docs/specs/parser.md`, then `cargo test --test parser --test parser_depth`
   (44 + 3). Review the draft AST in `compiler/src/ast.rs` first. It's shared, so change what you
   like, but tell me.
3. **Checker:** read `docs/specs/checker.md`, then `cargo test --test checker` (58), then
   `cargo test --test error_messages` and `cargo insta review` to approve your error messages.
4. **Codegen:** read `docs/specs/codegen.md`, then `cargo test --test codegen --test examples`
   (exact traces, 22 golden programs, the 10 gallery examples).
5. **Replay engine** (doesn't depend on the compiler, so do it whenever):
   `docs/specs/replay.md`, then `cd web && npx vitest run tests/replay` (18 correctness + 4 speed).

After each stage passes: bump `PIPELINE` (Lexer, Parser, Checker, Codegen), run `npm run wasm` in
`web/`, try broken programs in the playground, and move that stage's CI step up into the
required tests. When you're stuck, ask for a hint level (DEVPLAN.md §7.2).

Every test suite was checked against a throwaway reference implementation (kept outside the repo,
and deleted), so a failing test means a bug in the code under test, not in the test.

## Releasing through wearechintu.com

Decided 2026-09-27: Stepwise ships **as part of the wearechintu.com site**, the same way the site
itself is released (Vercel: every pushed branch gets a preview, and merging to `main` publishes).
It's the third line on the portfolio, "Stepwise: a debugger that runs in reverse". The built
playground is copied into the site at `public/stepwise-app/` and framed at
**wearechintu.com/stepwise**. Nothing is published until the compiler and replay engine pass
their tests, and until you've tried it on a preview.

Already done:
- **This repo:** `npm run export-site -- <site checkout>` (in `web/`). It builds the compiler,
  runs every Rust and web test, builds the site, and copies it with a `VERSION` stamp. Also the
  `?embed&theme=dark` framed view, and Share links that point at the host page when framed.
- **The site:** the local branch **`stepwise`** in the worktree
  `D:\VisualStudioProjects\gitbuddywebsite-stepwise`. Not pushed. It has the portfolio line, a
  Stepwise side (header, transition, dark page), `/stepwise` with the framed playground, and a
  separate security policy for `/stepwise-app/` that allows WebAssembly and same-site framing
  (every other page keeps the strict one). The site's 413 tests pass; its build passes apart from
  `/api/bundles`, which needs the Supabase keys that only Vercel has.
- **Tested locally** (2026-09-27) with a real export: the framed app compiles and runs programs
  under the new policy, `/stepwise` itself still can't run wasm or be framed, and Share links
  round-trip through `/stepwise#code=…`.

To release (once the MVP passes its tests):
1. **Test the app on its own:** `npm run dev` in `web/` and try the gallery, some broken programs,
   and stepping back and forth.
2. **Export into the site:** `npm run export-site -- D:\VisualStudioProjects\gitbuddywebsite-stepwise`
   (without `STEPWISE_SKIP_TESTS`, so the tests run).
3. **Test inside the site locally:** `npm run dev` in the site worktree, then open
   http://localhost:3000/stepwise.
4. **Commit** `public/stepwise-app/` on the site's `stepwise` branch and **push it**. Vercel builds a
   preview URL, and you test there: that's the real production setup.
5. **Merge to `main`** when you're happy. That publishes it.

For later updates, repeat steps 2–5 (export, test, commit, preview, merge).

## My track

Waiting on your decisions:
- **LICENSE:** MIT, which needs the name to put on it.
- **Social preview image:** after going live, since it needs the final URL.

Later: arrays and strings (v1.0) touch every stage, so they wait until the MVP works.

## Notes

- `docs/how-it-works.md` describes undo plus snapshots. Update it once you've chosen your replay
  design.
- Smart App Control was turned off on 2026-09-27 (it blocked rustc's own DLLs). fmt, clippy and the
  tests all work now.
- GitHub: https://github.com/TheAaravSikriwal/stepwise (public). Git has no credential helper, so
  pushes use `git -c credential.helper= -c "credential.helper=!gh auth git-credential" push`. Run
  `gh auth setup-git` once to make plain `git push` work.
- `cargo test --workspace` currently fails, because the core tests are waiting for your code. Use the
  commands above to run one piece at a time.
