testRule({
  config: [true],
  fix: true,
  accept: [
    {
      code: "a { color: red; }",
      description: "accept non-empty block",
      fast: true
    }
  ],
  reject: [
    {
      code: "a {}",
      description: "reject empty block",
      message: "Empty rule block detected",
      line: 1,
      column: 1,
      fixed: "",
      fast: true
    },
    {
      code: "a { #{$token}: red; }",
      description: "reject scss interpolation",
      message: "Unexpected empty block",
      line: 1,
      column: 1,
      skipReason: "scss_less",
      skipNote: "Requires SCSS syntax support, which is deferred for v1."
    }
  ]
});
