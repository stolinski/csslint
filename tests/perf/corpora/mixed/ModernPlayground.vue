<template>
  <section class="playground">
    <header class="playground-header">
      <h2>Modern Playground</h2>
      <button class="toggle" type="button">Preview</button>
    </header>

    <div class="panel-grid">
      <article class="panel">
        <h3>Container Queries</h3>
        <p>Adaptive card layout with nested selectors and progressive enhancement.</p>
      </article>
      <article class="panel warning">
        <h3>Color Systems</h3>
        <p>Using oklch and color-mix for cleaner contrast ramps.</p>
      </article>
    </div>
  </section>
</template>

<style scoped>
@layer base, components;

@property --tilt {
  syntax: "<angle>";
  inherits: false;
  initial-value: 0deg;
}

@layer base {
  .playground {
    --surface: oklch(97% 0.02 258);
    --ink: oklch(28% 0.03 258);
    --brand: oklch(66% 0.2 268);
    --line: color-mix(in oklch, var(--ink), transparent 84%);

    color: var(--ink);
    background: linear-gradient(165deg, var(--surface), color-mix(in oklch, var(--surface), white 30%));
    border: 1px solid var(--line);
    border-radius: 1rem;
    padding: clamp(1rem, 1.4vw + 0.5rem, 1.75rem);
  }
}

@layer components {
  .playground-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    margin-block-end: 0.9rem;
  }

  .toggle {
    appearance: none;
    border: 1px solid color-mix(in oklch, var(--brand), transparent 45%);
    background: color-mix(in oklch, var(--brand), white 84%);
    color: color-mix(in oklch, var(--brand), black 18%);
    border-radius: 999px;
    padding: 0.45rem 0.85rem;
    font: inherit;
    cursor: pointer;
    transition: transform 140ms ease, background 180ms ease;

    &:hover {
      background: color-mix(in oklch, var(--brand), white 76%);
    }

    &:active {
      transform: translateY(1px) scale(0.99);
    }

    &:focus-visible {
      outline: 2px solid color-mix(in oklch, var(--brand), white 18%);
      outline-offset: 2px;
    }
  }

  .panel-grid {
    container-type: inline-size;
    display: grid;
    gap: 0.8rem;
  }

  .panel {
    border: 1px solid var(--line);
    border-radius: 0.85rem;
    padding: 0.8rem;
    background: light-dark(white, oklch(24% 0.01 258));
    transform: rotate(var(--tilt));

    &:has(h3) {
      box-shadow: 0 10px 30px -24px color-mix(in oklch, black, transparent 55%);
    }

    &.warning {
      border-color: color-mix(in oklch, oklch(82% 0.16 88), transparent 55%);
      background: color-mix(in oklch, oklch(98% 0.04 92), white 25%);
    }
  }

  @container (width > 32rem) {
    .panel-grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }

  @starting-style {
    .panel {
      opacity: 0;
      transform: translateY(6px) scale(0.98);
    }
  }
}
</style>
