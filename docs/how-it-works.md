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
million entries. If a program is still going after that, Stepwise stops it, because it's
almost certainly an infinite loop.

## 3. The debugger replays the recording

When you step forward, the debugger reads the next diary entries and updates what you see.

Stepping **back** uses a trick. While reading the recording for the first time, the debugger saves
a full picture of the program's state (every function running and every variable's value) once
every thousand steps. To show any moment, whether one step back or a jump from step 900,000 to step
12, it takes the nearest picture from before that moment and replays at most a thousand diary
entries from there. It works the same in both directions, and it's fast enough to feel instant even
on a million-step run.

The recording also keeps each change's *old* value ("x changed from 5 to 6"). That's how the
"What just happened" box can tell you what changed and what it was before.

Real debuggers use the same idea. This way of doing it is called *record and replay*.

## What's in the box

| Part | Written in | What it does |
|---|---|---|
| Compiler | Rust → WebAssembly | checks your code and turns it into a recording program |
| Runtime | TypeScript | runs the program in a background thread and collects the recording |
| Replay engine | TypeScript | rebuilds the program's state at any step, forward or backward |
| Playground | TypeScript + CodeMirror | the editor and the debugger panels |

The source code is on [GitHub](https://github.com/TheAaravSikriwal/stepwise).
