# Changelog

This project follows [semantic versioning](https://semver.org): 0.1 is the MVP, 1.0 is the
public launch.

## Unreleased

### Added
- Playground: CodeMirror editor with Stepwise highlighting, error underlines, Ctrl+Enter to run
- Programs compile and run in a Web Worker, with a million-event limit and friendly runtime errors
- Debugger UI: call stack, variables with change highlights, output synced to the current step,
  step buttons, timeline, keyboard shortcuts (`?demo` previews it)
- Step over, step out and breakpoints, in both directions (click a line number or press F9;
  Shift+arrows step over, Page Up/Down jump between breakpoints)
- Example gallery with 10 commented programs, including "find the bug" exercises
- Shareable links: the program is stored in the URL
- Language reference and "How it works" pages
- Specs and test suites for every core piece: lexer, parser, checker, codegen, replay engine