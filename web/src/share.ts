// Shareable links: the program lives in the URL's hash as
// `#code=<base64url(deflate-raw(utf8 source))>`. The hash never reaches a
// server, so sharing needs no backend and nothing is stored anywhere.

const PREFIX = "code=";

/**
 * The biggest program a link may hold, unpacked. Far more than anyone
 * writes by hand, and it keeps a crafted link (a few kilobytes that inflate
 * to gigabytes) from freezing the tab of whoever opens it.
 */
export const MAX_PROGRAM_BYTES = 256 * 1024;

export async function encodeProgram(source: string): Promise<string> {
  const compressed = await pipe(new TextEncoder().encode(source), new CompressionStream("deflate-raw"));
  return PREFIX + toBase64Url(compressed);
}

/** Returns the program in a hash like `#code=...`, or `null` if there isn't a valid one. */
export async function decodeProgram(hash: string): Promise<string | null> {
  const h = hash.startsWith("#") ? hash.slice(1) : hash;
  if (!h.startsWith(PREFIX)) return null;
  try {
    const bytes = fromBase64Url(h.slice(PREFIX.length));
    const raw = await pipe(bytes, new DecompressionStream("deflate-raw"), MAX_PROGRAM_BYTES);
    return raw && new TextDecoder("utf-8", { fatal: true }).decode(raw);
  } catch {
    return null;
  }
}

/** Runs `data` through the stream; with a `limit`, gives up (null) as soon as the output passes it. */
async function pipe(data: Uint8Array, transform: CompressionStream | DecompressionStream): Promise<Uint8Array>;
async function pipe(
  data: Uint8Array,
  transform: CompressionStream | DecompressionStream,
  limit: number,
): Promise<Uint8Array | null>;
async function pipe(
  data: Uint8Array,
  transform: CompressionStream | DecompressionStream,
  limit = Infinity,
): Promise<Uint8Array | null> {
  const reader = new Blob([data as BlobPart]).stream().pipeThrough(transform).getReader();
  const chunks: Uint8Array[] = [];
  let size = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.length;
    if (size > limit) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const out = new Uint8Array(size);
  let at = 0;
  for (const c of chunks) {
    out.set(c, at);
    at += c.length;
  }
  return out;
}

function toBase64Url(bytes: Uint8Array): string {
  let bin = "";
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function fromBase64Url(text: string): Uint8Array {
  const b64 = text.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(b64 + "=".repeat((4 - (b64.length % 4)) % 4));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}
