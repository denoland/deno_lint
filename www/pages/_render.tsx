// This module adds twind support.

import { getStyleTagProperties, h, setup, virtualSheet } from "../deps.ts";
import { RenderContext, RenderFn } from "../server_deps.ts";

const sheet = virtualSheet();
sheet.reset();
setup({
  sheet,
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          '"Segoe UI"',
          '"Roboto"',
          '"Oxygen"',
          '"Ubuntu"',
          '"Cantarell"',
          '"Fira Sans"',
          '"Droid Sans"',
          '"Helvetica Neue"',
          "sans-serif",
        ],
      },
    },
  },
});

export function render(ctx: RenderContext, render: RenderFn) {
  const snapshot = ctx.state.get("twindSnapshot") as unknown[] | null;
  sheet.reset(snapshot || undefined);
  render();
  ctx.styles.splice(0, ctx.styles.length, ...sheet.target);
  const newSnapshot = sheet.reset();
  ctx.state.set("twindSnapshot", newSnapshot);
}

// export function postRender(ctx: RenderContext) {
//   // do normal stuff
//   ctx.head.push(
//     h("link", {
//       href:
//         "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/styles/monokai-sublime.min.css",
//       rel: "stylesheet",
//     }),
//   );
//   ctx.head.push(
//     h("script", {
//       src:
//         "https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/highlight.min.js",
//     }),
//   );

//   // do the twind stuff
//   const snapshot = ctx.state.get("twindSnapshot") as unknown[] | null;
//   if (snapshot !== null) {
//     sheet.reset(snapshot);
//     const { id, textContent } = getStyleTagProperties(sheet);
//     ctx.head.push(
//       h("style", { id, dangerouslySetInnerHTML: { __html: textContent } }),
//     );
//   }
//   sheet.reset(initial);
// }
