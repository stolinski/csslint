# csslint-semantic

Single-pass semantic model construction from parsed CSS inputs.

Responsibilities:

- Normalize selectors and annotate scope context.
- Build reusable semantic indexes for rule execution.
- Prevent re-parsing by downstream rule crates.
