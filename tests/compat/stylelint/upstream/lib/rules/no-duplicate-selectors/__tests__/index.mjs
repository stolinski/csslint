testRule({
  config: [true],
  accept: [
    {
      code: "a { color: red; } b { color: blue; }",
      description: "accept unique selectors",
      fast: true
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
    },
    {
      code: "/* stylelint-disable-next-line no-duplicate-selectors */\na { color: red; }\na { color: blue; }",
      description: "respect stylelint disable comment",
      message: "Duplicate selector 'a'",
      line: 3,
      column: 1,
      skipReason: "directive_comments",
      skipNote: "stylelint-disable directives are deferred from core v1 behavior."
    }
  ]
});
