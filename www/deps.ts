export * from "https://raw.githubusercontent.com/lucacasonato/fresh/5bc35b8b955c143654d22936ad5618274cbe2a28/runtime.ts";
import MarkdownIt from "https://dev.jspm.io/markdown-it@12.0.4";
export { MarkdownIt };
import { IS_BROWSER } from "https://raw.githubusercontent.com/lucacasonato/fresh/5bc35b8b955c143654d22936ad5618274cbe2a28/runtime.ts";
import { setup, tw } from "https://esm.sh/twind";

export { setup, tw };
if (IS_BROWSER) {
  setup({
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
}
