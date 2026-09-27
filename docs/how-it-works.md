# How Stepwise works

Stepwise lets you step **backward** through a program. That sounds like it needs a time machine,
but it doesn't: it needs a good recording.

## 1. Your code becomes WebAssembly

When you press Run, a compiler written in Rust (itself compiled to WebAssembly, so it runs in
your browser) turns your program into a small WebAssembly module, the same kind of fast machine
code browsers run for games and video editors. On the way, it checks your code and explains any
mistakes before anything runs.

Everything happens on your computer. There's no server: your code never leaves the page.

## 2. The program records itself as it runs

The compiler does one extra thing: before every statement, and whenever a variable changes, a
function is called, or something is printed, it inserts a tiny "note this down" call. So when
the program runs, it writes a diary of everything it does:

```
line 2 is starting
x was declared as 5
line 3 is starting
x changed from 5 to 6
print: 6
```

The whole program runs to the end first, usually in a few milliseconds. A long run can write a
million entries. If a program is still going at a million steps, Stepwise stops it, because it's
almost certainly an infinite loop.

## 3. The debugger replays the recording

When you step forward, the debugger reads the next diary entries and updates what you see.
When you step **back**, it undoes them. Undoing is cheap because every change was recorded with
its *old* value: "x changed from 5 to 6" can be undone by setting `x` back to 5.

To jump far (say, from step 900,000 to step 12), undoing one entry at a time would be slow. So
while reading the recording for the first time, the debugger saves a full picture of the state
every thousand steps. To jump anywhere, it starts from the nearest saved picture and replays at
most a thousand steps from there.

Real debuggers use the same idea. This way of doing it is called *record and replay*.

## What's in the box

| Part | Written in | What it does |
|---|---|---|
| Compiler | Rust → WebAssembly | checks your code and turns it into a recording program |
| Runtime | TypeScript | runs the program in a background thread and collects the recording |
| Replay engine | TypeScript | rebuilds the program's state at any step, forward or backward |
| Playground | TypeScript + CodeMirror | the editor and the debugger panels |

The source code is on [GitHub](https://github.com/TheAaravSikriwal/stepwise).
