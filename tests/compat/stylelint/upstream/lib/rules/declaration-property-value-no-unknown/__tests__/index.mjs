testRule({
  config: [true],
  accept: [
    {
      code: "a { display: flex; }",
      description: "accept known display value",
      fast: true
    },
    {
      code: "/* stylelint-disable declaration-property-value-no-unknown */\na { display: squish; opacity: 2; }\n/* stylelint-enable declaration-property-value-no-unknown */",
      description: "respect stylelint disable block comment",
      fast: true
    }
  ],
  reject: [
    {
      code: "a { display: squish; }",
      description: "reject unknown display value",
      message: "Invalid value 'squish' for property 'display'",
      line: 1,
      column: 4,
      fast: true
    },
    {
      code: "a { opacity: 2; }",
      description: "reject out of range opacity",
      message: "Invalid value '2' for property 'opacity'",
      line: 1,
      column: 4
    }
  ]
});
