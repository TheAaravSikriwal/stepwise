// Page-wide switches, applied before anything renders.
//
//   ?embed       hide the title and nav: wearechintu.com/stepwise has them
//   ?theme=dark  always dark (the page it's framed in is dark)
//
// Otherwise the theme follows the computer's light/dark setting, unless the
// theme button has picked one, which is remembered. The CSS does the
// following on its own (light-dark() + color-scheme); `data-theme` only
// appears when a choice overrides it.

const params = new URLSearchParams(location.search);
const root = document.documentElement;
const STORAGE_KEY = "stepwise-theme";

export type Theme = "light" | "dark";

export const embedded = params.has("embed");
if (embedded) root.classList.add("embedded");

/** A theme forced by the URL, which the toggle can't change. */
export const forcedTheme: Theme | null = params.get("theme") === "dark" ? "dark" : null;

function storedTheme(): Theme | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    return null; // storage can be blocked, e.g. in some framed contexts
  }
}

const chosen = forcedTheme ?? storedTheme();
if (chosen) root.dataset.theme = chosen;

/** The theme on screen now. */
export function currentTheme(): Theme {
  if (root.dataset.theme === "light" || root.dataset.theme === "dark") return root.dataset.theme;
  return matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

/** Picks a theme and remembers it. */
export function setTheme(theme: Theme): void {
  root.dataset.theme = theme;
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // Not remembered, but still applied for this visit.
  }
}
