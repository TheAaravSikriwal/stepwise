// URL switches for when the site is shown inside wearechintu.com/stepwise:
//
//   ?embed       hide the title and nav; the host page's header has them
//   ?theme=dark  use the dark theme whatever the OS prefers
//
// Imported first by every page, so the classes are set before anything renders.

const params = new URLSearchParams(location.search);
const root = document.documentElement;

if (params.has("embed")) root.classList.add("embedded");
if (params.get("theme") === "dark") root.classList.add("theme-dark");

export const embedded = params.has("embed");
