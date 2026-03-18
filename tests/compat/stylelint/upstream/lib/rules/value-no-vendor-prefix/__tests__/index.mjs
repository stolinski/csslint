testRule({
  config: [true],
  fix: true,
  accept: [
    {
      code: "a { display: flex; }",
      description: "accept unprefixed value",
      fast: true
    }
  ],
  reject: [
    {
      code: "a { display: -webkit-flex; }",
      description: "reject vendor-prefixed value",
      message: "Legacy vendor-prefixed value '-webkit-flex'",
      line: 1,
      column: 4,
      fixed: "a { display: flex; }",
      fast: true
    },
    {
      code: "a { display: value(var(--dynamic)); }",
      description: "reject postcss value parser integration case",
      message: "Unexpected unknown value",
      line: 1,
      column: 14,
      skipReason: "postcss_integration",
      skipNote: "PostCSS value parser integration cases are deferred in v1."
    }
  ]
});
