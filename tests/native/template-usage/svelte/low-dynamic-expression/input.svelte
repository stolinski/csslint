<script>
  let value = 0;
  const computeClass = (v) => (v > 0 ? 'positive' : `dynamic-${v}`);
</script>

<div class={computeClass(value)}></div>

<style>
  .positive {}
  .dynamic-0 {}
</style>
