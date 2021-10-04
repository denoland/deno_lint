// This module adds twind support.

import { h, setup, twTypography, virtualSheet } from "../deps.ts";
import { RenderContext, RenderFn } from "../server_deps.ts";

const sheet = virtualSheet();
sheet.reset();
setup({
  sheet,
  plugins: {
    ...twTypography(),
  },
  darkMode: "media",
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
