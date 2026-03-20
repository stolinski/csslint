testRule({
  config: [true],
  accept: [
    {
      code: ".card { color: red; }",
      description: "accept class selector",
      fast: true
    },
    {
      code: "/* stylelint-disable-next-line selector-no-qualifying-type */\narticle.card { color: red; }",
      description: "respect stylelint disable-next-line comment",
      fast: true
    }
  ],
  reject: [
    {
      code: "article.card { color: red; }",
      description: "reject type qualified class selector",
      message: "Overqualified selector 'article.card'",
      line: 1,
      column: 1,
      fast: true
    },
    {
      code: "&.card { color: red; }",
      description: "reject nested selector syntax",
      message: "Overqualified selector '&.card'",
      line: 1,
      column: 1,
      skipReason: "custom_syntax",
      skipNote: "Nested selector syntax requires custom syntax adapters outside v1 scope."
    }
  ]
});
