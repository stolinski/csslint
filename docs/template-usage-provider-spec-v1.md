# Template Usage Provider Spec (v1 Design)

## Purpose

Define a typed contract for supplying template usage facts to plugin-path rules such as `no_unused_scoped_selectors`.

This is a design spec for the extension surface. It does not require runtime JS plugin loading.

## Scope

In scope:

- usage facts for class and id selectors
- confidence scoring for each usage fact
- deterministic provider output shape

Out of scope:

- full template semantic execution
- runtime simulation of framework compilers
- guaranteed perfect dead-selector detection

## Provider ID and Contract

Provider id: `template_usage`

```rust
pub trait UsageProvider {
    fn id(&self) -> &'static str;
    fn collect(&self, input: &TemplateUsageInput) -> TemplateUsageOutput;
}
```

## Input Types

```rust
pub struct TemplateUsageInput {
    pub file_id: FileId,
    pub file_path: Arc<str>,
    pub framework: FrameworkKind, // Vue | Svelte
    pub source: Arc<str>,
    pub styles: Vec<StyleBlockRef>,
}

pub struct StyleBlockRef {
    pub block_index: u32,
    pub start_offset: usize,
    pub end_offset: usize,
    pub scoped: bool,
    pub module: bool,
}
```

Notes:

- provider reads full component source and can parse template regions
- `styles` allows provider output to be aligned to scoped/module style blocks

## Output Types

```rust
pub enum ProviderStatus {
    Complete,
    Partial,
    FailedRecoverable,
}

pub enum UsageKind {
    Class,
    Id,
}

pub enum Confidence {
    High,
    Medium,
    Low,
}

pub enum UsageSource {
    StaticAttribute,
    FrameworkDirectiveLiteral,
    BindingLiteralBranch,
    DynamicExpressionHeuristic,
}

pub struct UsageFact {
    pub kind: UsageKind,
    pub name: InternedStr,
    pub confidence: Confidence,
    pub source: UsageSource,
    pub span: Span,
}

pub struct ProviderDiagnostic {
    pub message: String,
    pub span: Option<Span>,
}

pub struct TemplateUsageOutput {
    pub status: ProviderStatus,
    pub facts: Vec<UsageFact>,
    pub unknown_regions: Vec<Span>,
    pub diagnostics: Vec<ProviderDiagnostic>,
}
```

## Confidence Model

- `High`: direct static evidence from template AST (for example `class="foo bar"`, `id="hero"`, `class:active` in Svelte)
- `Medium`: finite literal branches from bindings (for example ternary with string literals)
- `Low`: heuristic extraction from dynamic expressions that cannot be proven statically

## Rule Consumption Contract (`no_unused_scoped_selectors`)

Default behavior for low-noise reporting:

1. Treat `High` and `Medium` matches as used.
2. Treat `Low` matches as unknown, not as unused proof.
3. If provider status is `Partial` or `FailedRecoverable`, do not report unused selector diagnostics for that file.
4. Only report for simple scoped selector candidates in plugin v1 (`.class` and `#id` without complex combinator logic).

This keeps false positives low and allows confidence to improve over time.

## Determinism Requirements

- `facts` must be sorted by `(kind, name, span.start, span.end, confidence)`.
- `diagnostics` must be sorted by span then message.
- no dependence on hash-map iteration order.

## Error Handling

- provider parser failures must be converted to `ProviderDiagnostic`
- provider failures are recoverable and must not crash lint run
- engine behavior remains deterministic regardless of provider failure

## Performance Budget (Design Target)

- linear in template size for standard inputs
- bounded allocations for fact collection
- no repeated reparsing per rule; provider runs once per component file

## Test Requirements

- Vue and Svelte fixtures for static class/id usage
- fixtures for dynamic bindings producing `Medium` and `Low`
- recoverable parse-failure fixtures
- deterministic snapshot for provider outputs

Fixture format reference:

- `docs/template-usage-fixtures-v1.md`
