# Changelog

This project follows [semantic versioning](https://semver.org): 0.1 is the MVP, 1.0 is the
public launch.

## 0.2.0 (unreleased): black and gold, "where from?", and the call galaxy

### Added
- **The call galaxy:** a "Call galaxy" view beside "Your code" shows the whole run in 3D, filling
  the page, with the panels floating over it as frosted glass. Every
  function call is a bubble, each function in its own clear colour (sapphire, emerald, ruby,
  amethyst, teal and more) (bigger means more steps ran in it), the calls it made sit around it,
  and recursion curls into a chain. The call running now glows gold, finished calls fade, and
  the running calls' variables orbit as moons. Drag to look around and scroll to zoom. Hover a bubble to see
  that call: its arguments (`fib(n = 2)`), the steps it covered, what it gave back, and each of its
  variables with its value and the step it got it. Hover a moon for that one variable. Click
  either to jump there. It follows every step. Past 2,000 calls, later ones fold
  into their callers. three.js loads only when the view first opens.
- **"Where from?"** on every variable: jumps to the step that gave it its value, outlines that line,
  and says so ("Line 9 called `factorial` and passed in `n` = 3"). Exact under recursion.
- **Light and dark themes:** follows the computer's setting, with a toggle that's remembered.

### Changed
- **Black, white and gold:** nothing sits in a box; the parts of the page float on one dark field
  (or ivory, in the light theme), set apart by space and type. Gold means "now" and marks Run.
  The name sits top left, and the two views ("Your code", "Call galaxy") are big buttons at the
  top centre. The Variables panel says whose variables they are and at which step, under Name and
  Value headings.
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
