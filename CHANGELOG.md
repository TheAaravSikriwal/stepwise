# Changelog

This project follows [semantic versioning](https://semver.org): 0.1 is the MVP, 1.0 is the
public launch.

## 0.2.0 (unreleased): Electric, "where from?", and the call galaxy

### Added
- **The call galaxy:** a "Call galaxy" tab beside "Your code" shows the whole run in 3D. Every
  function call is a bubble (bigger means more steps ran in it), the calls it made sit around it,
  and recursion curls into a chain. The call running now glows yellow, finished calls fade, and
  the running calls' variables orbit as moons. Drag to look around, scroll to zoom, hover for
  details, click a bubble to jump there. It follows every step. Past 2,000 calls, later ones fold
  into their callers. three.js loads only when the tab first opens.
- **"Where from?"** on every variable: jumps to the step that gave it its value, marks that line in
  blue, and says so ("Line 9 called `factorial` and passed in `n` = 3"). Exact under recursion.
- **Light and dark themes:** follows the computer's setting, with a toggle that's remembered.

### Changed
- **The Electric look:** GitHub-dark colours, blue for actions, yellow for "now"; rounded cards,
  pill buttons and soft transitions.
- **Code is coloured by what it is:** declarations, control flow, functions where they're defined
  and where they're called, parameters, new variables, types, numbers, booleans, `print`,
  operators and comments.

## 0.1.0 (unreleased): the MVP

### Added
- **The compiler** (Rust → WebAssembly): lexer, parser, checker and codegen, with trace
  instrumentation at every step. Error messages are written for beginners and snapshot-tested.
- **The replay engine:** step forwards and backwards through any run, and jump anywhere instantly
  (snapshots every 1,000 steps).
- **"What just happened":** every step explained in plain English, including new and changed
  variables, variables going away, calls and returns, what printed, and what each `if`/`while`
  decided.
- **Playground:** CodeMirror editor with highlighting and error underlines; Run, Next step,
  Back, Start and End; a timeline; More moves (skip over calls, finish a function, jump between
  stops, all in both directions); Variables, Output and Functions running panels, each explained;
  keyboard shortcuts.
- Programs run in a Web Worker, with a limit on runaway programs and plain-English runtime
  errors.
- The look: cobalt industrial blue, white, safety yellow and black. A black version for
  wearechintu.com.
- Example gallery with 10 commented programs, including two "find the bug" exercises.
- Shareable links: the program is stored in the URL.
- The language reference and "How it works" pages.
- Release through wearechintu.com: `npm run export-site`.
