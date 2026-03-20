testRule({
  config: [true],
  fix: true,
  accept: [
    {
      code: "a { transform: rotate(0); }",
      description: "accept unprefixed property",
      fast: true
    },
    {
      code: "a { /* stylelint-disable-line property-no-vendor-prefix */ -webkit-transform: rotate(0); }",
      description: "respect stylelint disable-line comment",
      fast: true
    }
  ],
  reject: [
    {
      code: "a { -webkit-transform: rotate(0); }",
      description: "reject vendor-prefixed property",
      message: "Legacy vendor-prefixed property '-webkit-transform'",
      line: 1,
      column: 4,
      fixed: "a { transform: rotate(0); }",
      fast: true
    }
  ]
});
