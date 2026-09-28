import { describe, expect, it } from "vitest";
import { MAX_PROGRAM_BYTES, decodeProgram, encodeProgram } from "../src/share";

describe("shareable links", () => {
  it("round-trips a program", async () => {
    const src = "fn main() {\n    let x = 5;\n    print(x * 2);\n}\n";
    const hash = await encodeProgram(src);
    expect(hash.startsWith("code=")).toBe(true);
    expect(await decodeProgram("#" + hash)).toBe(src);
    expect(await decodeProgram(hash)).toBe(src);
  });

  it("round-trips non-ASCII text and empty programs", async () => {
    for (const src of ["// café 😀\nfn main() {}", ""]) {
      expect(await decodeProgram(await encodeProgram(src))).toBe(src);
    }
  });

  it("uses only URL-safe characters", async () => {
    const hash = await encodeProgram("fn main() { print(1 + 2 * 3); } // ???>>>~~~".repeat(20));
    expect(hash).toMatch(/^code=[A-Za-z0-9_-]+$/);
  });

  it("compresses repetitive code", async () => {
    const src = "fn main() {\n" + "    print(1);\n".repeat(200) + "}\n";
    expect((await encodeProgram(src)).length).toBeLessThan(src.length / 5);
  });

  it("rejects hashes that aren't programs", async () => {
    expect(await decodeProgram("")).toBeNull();
    expect(await decodeProgram("#other=1")).toBeNull();
    expect(await decodeProgram("#code=not!valid!base64")).toBeNull();
    expect(await decodeProgram("#code=AAAA")).toBeNull();
  });

  it("refuses a link that unpacks to something huge, without unpacking it all", async () => {
    // A few kilobytes of link that would inflate to 50 MB.
    const bomb = await encodeProgram("a".repeat(50 * 1024 * 1024));
    expect(bomb.length).toBeLessThan(200_000);
    const start = performance.now();
    expect(await decodeProgram(bomb)).toBeNull();
    expect(performance.now() - start).toBeLessThan(1000);
  });

  it("still opens a program right up to the limit", async () => {
    const src = "// " + "x".repeat(MAX_PROGRAM_BYTES - 3);
    expect(await decodeProgram(await encodeProgram(src))).toBe(src);
  });
});
