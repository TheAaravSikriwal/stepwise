// The example gallery: every program in docs/examples, bundled at build time.
// compiler/tests/examples.rs checks that they all compile and run.

const files = import.meta.glob("../../docs/examples/*.step", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

/** Gallery order and titles. Files not listed here are appended at the end. */
const TITLES: [string, string][] = [
  ["hello", "Hello"],
  ["factorial", "Factorial (loops and recursion)"],
  ["fibonacci", "Fibonacci (recursion)"],
  ["countdown", "Countdown (step back through returns)"],
  ["primes", "Prime numbers"],
  ["gcd", "Greatest common divisor"],
  ["collatz", "Collatz sequence"],
  ["short_circuit", "How && and || skip work"],
  ["find_the_bug", "Find the bug: the wrong sum"],
  ["infinite_loop", "Find the bug: a loop that never ends"],
];

export interface Example {
  id: string;
  title: string;
  source: string;
}

export const EXAMPLES: Example[] = (() => {
  const byId = new Map(
    Object.entries(files).map(([path, source]) => [path.split("/").pop()!.replace(/\.step$/, ""), source]),
  );
  const list: Example[] = [];
  for (const [id, title] of TITLES) {
    const source = byId.get(id);
    if (source !== undefined) list.push({ id, title, source });
    byId.delete(id);
  }
  for (const [id, source] of byId) list.push({ id, title: id.replace(/_/g, " "), source });
  return list;
})();
