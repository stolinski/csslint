<script>
  const items = [
    { label: "Signals", value: "96%" },
    { label: "Throughput", value: "124 req/s" },
    { label: "Error Budget", value: "99.96%" }
  ];
</script>

<section class="widget">
  <header>
    <h3>Modern Widget</h3>
    <button type="button" class="pill">Refresh</button>
  </header>

  <ul>
    {#each items as item}
      <li>
        <span>{item.label}</span>
        <strong>{item.value}</strong>
      </li>
    {/each}
  </ul>
</section>

<style>
.widget {
  --bg: oklch(97% 0.015 252);
  --fg: oklch(29% 0.03 252);
  --brand: oklch(68% 0.2 258);
  --line: color-mix(in oklch, var(--fg), transparent 84%);

  container: widget / inline-size;
  display: grid;
  gap: 0.8rem;
  color: var(--fg);
  border: 1px solid var(--line);
  border-radius: 1rem;
  padding: 1rem;
  background:
    radial-gradient(35rem 30rem at 100% -35%, color-mix(in oklch, var(--brand), transparent 86%), transparent),
    linear-gradient(160deg, var(--bg), color-mix(in oklch, var(--bg), white 22%));

  & > header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  & > ul {
    display: grid;
    gap: 0.55rem;
    list-style: none;
  }

  & li {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0.75rem;
    border: 1px solid var(--line);
    border-radius: 0.75rem;
    padding: 0.6rem 0.7rem;
    background: light-dark(white, oklch(24% 0.01 252));
  }
}

.pill {
  appearance: none;
  border: 1px solid color-mix(in oklch, var(--brand), transparent 40%);
  border-radius: 999px;
  font: inherit;
  color: color-mix(in oklch, var(--brand), black 15%);
  background: color-mix(in oklch, var(--brand), white 84%);
  padding: 0.35rem 0.7rem;
  cursor: pointer;

  &:hover {
    background: color-mix(in oklch, var(--brand), white 75%);
  }

  &:focus-visible {
    outline: 2px solid color-mix(in oklch, var(--brand), white 12%);
    outline-offset: 2px;
  }
}

@container widget (width < 22rem) {
  .widget li {
    grid-template-columns: 1fr;
    gap: 0.15rem;

    & > strong {
      color: color-mix(in oklch, var(--brand), black 12%);
    }
  }
}
</style>
