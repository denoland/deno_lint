import IconBrandGithub from "https://deno.land/x/tabler_icons_tsx@0.0.5/tsx/brand-github.tsx";
import IconExternalLink from "https://deno.land/x/tabler_icons_tsx@0.0.5/tsx/external-link.tsx";

const headerLinks = [
  { href: "/", label: "Rule overview", isInternal: true },
  { href: "/ignoring-rules", label: "Ignoring rules", isInternal: true },
  { href: "/playground", label: "Playground", isInternal: true },
  {
    href: "https://docs.deno.com/runtime/manual/tools/linter",
    label: "Docs",
    isInternal: false,
  },
] as const;

type ActivePathOptions = typeof headerLinks[number]["href"];

export function Header({ active }: { active: ActivePathOptions }) {
  return (
    <section class="my-8 flex md:flex-row flex-col justify-between md:items-center gap-y-4">
      <a href="/">
        <h1 class="flex text-3xl font-bold flex items-center gap-1">
          <img
            src="/logo.svg"
            alt="deno_lint logo"
            class="h-12 w-12 inline-block mr-2"
          />
          deno_lint
        </h1>
      </a>
      <div class="flex items-center gap-3">
        <div class="flex gap-4">
          {headerLinks.map((item) => (
            <a
              href={item.href}
              class={`inline-flex gap-1 items-center hover:underline ${
                active === item.href ? "font-bold" : ""
              }`}
              target={!item.isInternal ? "_blank" : undefined}
            >
              {item.label}
              {!item.isInternal && <IconExternalLink class="w-4 h-4" />}
            </a>
          ))}
        </div>
        <div>
          <a
            href="https://github.com/denoland/deno_lint"
            class="hover:underline flex gap-1 items-center"
          >
            <IconBrandGithub title="View on GitHub" class="w-5 h-5" />
          </a>
        </div>
      </div>
    </section>
  );
}
