export * from "https://raw.githubusercontent.com/lucacasonato/fresh/d1076b0ad1b420aec689324b3342e543c0d5a591/runtime.ts";
export { default as marked } from "https://esm.sh/marked@3.0.4";
export * as Prism from "https://esm.sh/prismjs@1.25.0";
import "https://esm.sh/prismjs@1.25.0/components/prism-javascript.js?no-check";
import "https://esm.sh/prismjs@1.25.0/components/prism-typescript.js?no-check";
import { IS_BROWSER } from "https://raw.githubusercontent.com/lucacasonato/fresh/d1076b0ad1b420aec689324b3342e543c0d5a591/runtime.ts";
import { setup, tw } from "https://esm.sh/twind@0.16.16";
export {
  getStyleTagProperties,
  virtualSheet,
} from "https://esm.sh/twind@0.16.16/sheets";
export { default as twTypography } from "https://esm.sh/@twind/typography@0.0.2?deps=twind@0.16.16&no-check";

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
