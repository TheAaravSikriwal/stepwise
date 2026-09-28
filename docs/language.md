# The Stepwise language

Stepwise is deliberately small: you can learn all of it in ten minutes, and then spend your time
watching your programs run.

## A complete program

```
fn square(n: int) -> int {
    return n * n;
}

fn main() {
    let mut i = 1;
    while i <= 3 {
        print(square(i));   // prints 1, 4, 9
        i = i + 1;
    }
}
```

Every program starts at `fn main()`.

## Values

| Type | Values | Examples |
|---|---|---|
| `int` | whole numbers from −2147483648 to 2147483647 | `0`, `42`, `-7` |
| `bool` | `true` or `false` | `true`, `x > 3` |

## Variables

```
let x = 5;             // can't be changed
let mut count = 0;     // `mut` means it can change
count = count + 1;
let done: bool = false; // you can write the type, but you don't have to
```

- A variable exists from its `let` to the end of the `{ }` block it's in.
- Each name can only be used once at a time: you can't declare a second `x` while the first
  one is still around.

## Operators

| Operator | Meaning | Works on |
|---|---|---|
| `+ - * /` | add, subtract, multiply, divide | `int` |
| `%` | remainder: `7 % 3` is `1` | `int` |
| `< <= > >=` | compare | `int` |
| `== !=` | equal, not equal | two `int`s or two `bool`s |
| `&& \|\| !` | and, or, not | `bool` |

- `/` drops the fraction: `7 / 2` is `3`, and `-7 / 2` is `-3`.
- `&&` and `||` stop early when they already know the answer.
- Comparisons can't be chained: write `1 < x && x < 5`, not `1 < x < 5`.

## Decisions and loops

```
if x > 0 {
    print(1);
} else if x < 0 {
    print(-1);
} else {
    print(0);
}

while n > 0 {
    n = n - 1;
}
```

Conditions must be `bool`: write `if n != 0`, not `if n`.

## Functions

```
fn add(a: int, b: int) -> int {   // takes two ints, gives back an int
    return a + b;
}

fn greet(times: int) {            // no `->`: gives nothing back
    print(times);
}
```

- Parameters can't be changed. Copy one into a `let mut` if you need to.
- A function with `->` must `return` a value on every path.
- Functions can call themselves (recursion) and each other, in any order.

## Printing and comments

```
print(42);       // prints 42
print(3 < 4);    // prints true
// Everything after two slashes is a comment.
```

## When things go wrong

- **Mistakes in your code** are underlined before the program runs, with a message explaining
  the problem.
- **Dividing by zero**, or recursion that never stops, ends the program with a message. You can
  still step through everything that happened before.
- A program that runs for a very long time (hundreds of thousands of steps) is stopped, since it's probably an
  infinite loop, and you can step backward to find out why.
