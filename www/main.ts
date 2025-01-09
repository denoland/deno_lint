const rulePat = new URLPattern({
  pathname: "/rules/:rule",
}, {
  ignoreCase: true,
});

Deno.serve((req) => {
  const url = new URL(req.url);

  const ruleMatch = rulePat.exec(req.url);
  const maybeRule = ruleMatch?.pathname.groups.rule;

  if (maybeRule) {
    return Response.redirect(
      `https://docs.deno.com/lint/rules/${maybeRule}`,
      301,
    );
  }

  if (url.pathname.startsWith("/ignoring-rules")) {
    // TODO(bartlomieju): verify the anchor is not changed or use
    // "go" url
    return Response.redirect(
      `https://docs.deno.com/go/lint-ignore`,
      301,
    );
  }

  return Response.redirect(
    "https://docs.deno.com/lint/",
    301,
  );
});
