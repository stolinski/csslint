testRule({
  config: [true],
  accept: [
    {
      code: "a { color: red; } b { color: blue; }",
      description: "accept unique selectors",
      fast: true
    },
    {
      code: "/* stylelint-disable no-duplicate-selectors */\na { color: red; }\na { color: blue; }\n/* stylelint-enable no-duplicate-selectors */",
      description: "respect stylelint disable comment",
      fast: false
    }
  ],
  reject: [
    {
      code: "a { color: red; }\na { color: blue; }",
      description: "reject duplicate selector",
      message: "Duplicate selector 'a'",
      line: 2,
      column: 1,
      fast: true
    }
  ]
});
