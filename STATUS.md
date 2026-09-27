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

## Going live on wearechintu.com (once the MVP works)

Decided 2026-09-27: Stepwise becomes the third line on the wearechintu.com portfolio,
"Stepwise: a debugger that runs in reverse". The app is served from **stepwise.wearechintu.com**
and framed at **wearechintu.com/stepwise**. Nothing is published until the compiler and replay
engine pass their tests.

Already done:
- This repo: `deploy.yml` (GitHub Pages, manual-only for now, runs the web tests first),
  `web/public/CNAME`, and `?embed&theme=dark` for the framed view.
- The site: commit `6f59ff2` on the local branch **`stepwise`**, in the worktree
  `D:\VisualStudioProjects\gitbuddywebsite-stepwise`. Not pushed. It adds the portfolio line, a
  Stepwise side (header, transition, dark page) and `/stepwise` with the framed playground. The
  site's 413 tests pass, and its build passes apart from `/api/bundles`, which needs the Supabase
  keys that only Vercel has.

To go live, in order:
1. Stepwise repo: Settings → Pages → Source: **GitHub Actions**, then run **Deploy site** from the
   Actions tab.
2. DNS (Cloudflare): `CNAME stepwise → theaaravsikriwal.github.io`, DNS only (grey cloud), so
   GitHub can issue the HTTPS certificate.
3. Check https://stepwise.wearechintu.com, then add `push: branches: [main]` to `deploy.yml`.
4. Site: push the `stepwise` branch for a Vercel preview, try it, then merge into `main`.

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
