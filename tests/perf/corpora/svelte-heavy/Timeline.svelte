<script>
  const events = [
    { time: "08:00", title: "Design review", kind: "base" },
    { time: "10:30", title: "Runtime profiling", kind: "accent" },
    { time: "13:00", title: "CI benchmark", kind: "success" },
    { time: "16:45", title: "Release candidate", kind: "warning" }
  ];
</script>

<section class="timeline">
  <h3>Today</h3>
  <ol>
    {#each events as event}
      <li class={event.kind}>
        <time>{event.time}</time>
        <div>
          <strong>{event.title}</strong>
          <small>Nested selectors and color-mix stress fixture</small>
        </div>
      </li>
    {/each}
  </ol>
</section>

<style>
.timeline {
  --bg: oklch(98% 0.01 260);
  --fg: oklch(30% 0.03 260);
  --line: color-mix(in oklch, var(--fg), transparent 82%);
  --accent: oklch(64% 0.2 258);
  --success: oklch(70% 0.16 160);
  --warning: oklch(80% 0.17 90);

  color: var(--fg);
  border: 1px solid var(--line);
  border-radius: 1rem;
  padding: 1rem;
  background: var(--bg);

  & > ol {
    list-style: none;
    display: grid;
    gap: 0.55rem;
    margin-block-start: 0.75rem;
  }

  & li {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.6rem;
    align-items: start;
    border: 1px solid var(--line);
    border-radius: 0.8rem;
    padding: 0.55rem 0.65rem;
    background: light-dark(white, oklch(23% 0.01 260));

    & > time {
      font-variant-numeric: tabular-nums;
      font-size: 0.78rem;
      color: color-mix(in oklch, var(--fg), transparent 34%);
    }

    & > div {
      display: grid;
      gap: 0.2rem;
    }

    & small {
      color: color-mix(in oklch, var(--fg), transparent 44%);
    }

    &.accent {
      border-color: color-mix(in oklch, var(--accent), transparent 58%);
    }

    &.success {
      border-color: color-mix(in oklch, var(--success), transparent 58%);
    }

    &.warning {
      border-color: color-mix(in oklch, var(--warning), transparent 58%);
    }
  }
}
</style>
