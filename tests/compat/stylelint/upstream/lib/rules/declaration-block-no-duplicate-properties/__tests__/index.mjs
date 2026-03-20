testRule({
  config: [true],
  fix: true,
  accept: [
    {
      code: "a { color: red; color: blue; }",
      description: "accept non-duplicate declaration values",
      fast: true
    },
    {
      code: "a { /* stylelint-disable-line declaration-block-no-duplicate-properties */ color: red; color: red; }",
      description: "respect stylelint disable-line comment",
      fast: true
    },
    {
      code: "a { color: red; color: blue; }",
      description: "ignore duplicate values with stylelint option",
      message: "Unexpected duplicate declaration",
      line: 1,
      column: 16,
      skipReason: "unsupported_option",
      skipNote: "ignore options for duplicate declaration handling are out of scope for v1."
    }
  ],
  reject: [
    {
      code: "a { color: red; color: red; }",
      description: "reject duplicate declaration",
      message: "Duplicate declaration 'color: red'",
      line: 1,
      column: 16,
      fixed: "a { color: red; }",
      fast: true
    }
  ]
});
