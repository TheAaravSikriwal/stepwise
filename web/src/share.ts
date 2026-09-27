// Shareable links: the program lives in the URL's hash as
// `#code=<base64url(deflate-raw(utf8 source))>`. The hash never reaches a
// server, so sharing needs no backend and nothing is stored anywhere.

const PREFIX = "code=";

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
    const raw = await pipe(bytes, new DecompressionStream("deflate-raw"));
    return new TextDecoder("utf-8", { fatal: true }).decode(raw);
  } catch {
    return null;
  }
}

async function pipe(data: Uint8Array, transform: CompressionStream | DecompressionStream): Promise<Uint8Array> {
  const stream = new Blob([data as BlobPart]).stream().pipeThrough(transform);
  return new Uint8Array(await new Response(stream).arrayBuffer());
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
