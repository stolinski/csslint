<template>
  <section class="workspace">
    <header>
      <h3>Workspace</h3>
      <div class="actions">
        <button type="button">Run</button>
        <button type="button">Share</button>
      </div>
    </header>

    <main>
      <article class="surface">
        <h4>Editor</h4>
        <p>Container and nested selector heavy fixture for perf sweeps.</p>
      </article>
      <article class="surface emphasized">
        <h4>Preview</h4>
        <p>Uses light-dark, color-mix, @container, and @starting-style.</p>
      </article>
    </main>
  </section>
</template>

<style scoped>
.workspace {
  --bg: oklch(97% 0.01 257);
  --ink: oklch(29% 0.03 257);
  --line: color-mix(in oklch, var(--ink), transparent 84%);
  --brand: oklch(66% 0.21 264);

  container: workspace / inline-size;
  display: grid;
  gap: 0.9rem;
  border: 1px solid var(--line);
  border-radius: 1rem;
  padding: 1rem;
  color: var(--ink);
  background: linear-gradient(160deg, var(--bg), color-mix(in oklch, var(--bg), white 25%));

  & > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.8rem;
  }

  & .actions {
    display: inline-flex;
    gap: 0.45rem;
  }

  & button {
    appearance: none;
    border: 1px solid color-mix(in oklch, var(--brand), transparent 45%);
    border-radius: 0.7rem;
    background: color-mix(in oklch, var(--brand), white 84%);
    color: color-mix(in oklch, var(--brand), black 18%);
    font: inherit;
    cursor: pointer;
    padding: 0.35rem 0.62rem;

    &:hover {
      background: color-mix(in oklch, var(--brand), white 74%);
    }

    &:focus-visible {
      outline: 2px solid color-mix(in oklch, var(--brand), white 14%);
      outline-offset: 2px;
    }
  }

  & > main {
    display: grid;
    gap: 0.7rem;
  }
}

.surface {
  border: 1px solid var(--line);
  border-radius: 0.85rem;
  padding: 0.8rem;
  background: light-dark(white, oklch(24% 0.01 257));

  &.emphasized {
    border-color: color-mix(in oklch, var(--brand), transparent 60%);
  }
}

@container workspace (width > 42rem) {
  .workspace > main {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@starting-style {
  .surface {
    opacity: 0;
    transform: translateY(4px);
  }
}
</style>
