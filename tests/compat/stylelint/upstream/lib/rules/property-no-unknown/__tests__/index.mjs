testRule({
  config: [true],
  accept: [
    {
      code: "a { color: red; }",
      description: "accept known property",
      fast: true
    },
    {
      code: "/* stylelint-disable-next-line property-no-unknown */\na { colr: red; }",
      description: "respect stylelint disable-next-line comment",
      fast: true
    },
    {
      code: "a { colr: red; }",
      description: "ignore unknown property via option",
      message: "Unexpected unknown property",
      line: 1,
      column: 4,
      skipReason: "unsupported_option",
      skipNote: "ignoreProperties option is outside the v1 compatibility subset."
    }
  ],
  reject: [
    {
      code: "a { colr: red; }",
      description: "reject unknown property",
      message: "Unknown property 'colr'",
      line: 1,
      column: 4,
      fast: true
    }
  ]
});
