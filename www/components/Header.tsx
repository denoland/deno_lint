import IconBrandGithub from "https://deno.land/x/tabler_icons_tsx@0.0.5/tsx/brand-github.tsx";

export function Header({ active }: { active: string }) {
  const items = [
    { href: "/", label: "Rule overview" },
    { href: "/ignoring-rules", label: "Ignoring rules" },
  ];

  return (
    <section class="my-8">
      <h1 class="text-3xl font-bold flex items-center gap-1">
        <img
          src="/logo.svg"
          alt="deno_lint logo"
          class="h-12 w-12 inline-block mr-2"
        />
        deno_lint
      </h1>
      <div class="flex flex-wrap justify-between w-full">
        <div class="mt-4 flex gap-4">
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
        <div class="mt-4">
          <a
            href="https://github.com/denoland/deno_lint"
            class="hover:underline flex gap-1 items-center"
          >
            <IconBrandGithub class="w-6 h-6" />
            View on GitHub
          </a>
        </div>
      </div>
    </section>
  );
}
