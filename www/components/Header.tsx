import IconBrandGithub from "https://deno.land/x/tabler_icons_tsx@0.0.5/tsx/brand-github.tsx";

export function Header({ active }: { active: string }) {
  const items = [
    { href: "/", label: "Rule overview" },
    { href: "/ignoring-rules", label: "Ignoring rules" },
  ];

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
          {items.map((item) => (
            <a
              href={item.href}
              class={`hover:underline ${
                active === item.href ? "font-bold" : ""
              }`}
            >
              {item.label}
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
