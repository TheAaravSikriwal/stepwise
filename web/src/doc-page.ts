// Renders one of the Markdown files in docs/ as a page of the site, so the
// docs and the site never drift apart. The page's <body data-doc="..."> picks
// which one.

import { marked } from "marked";
import howItWorks from "../../docs/how-it-works.md?raw";
import language from "../../docs/language.md?raw";

const DOCS: Record<string, string> = { language, "how-it-works": howItWorks };

const article = document.querySelector<HTMLElement>("article")!;
const markdown = DOCS[document.body.dataset.doc ?? ""] ?? "# Page not found";
// Our own trusted Markdown, bundled at build time.
article.innerHTML = await marked.parse(markdown);
document.title = `${article.querySelector("h1")?.textContent ?? "Stepwise"} · Stepwise`;
