<template>
  <section id="error-404">
    <div class="container grid-background">
      <h2>Error 404 | Page not found</h2>
      <h4>Should have caught this with Vitest, whoops 🤷</h4>
      <a href="/"><button>Return to home</button></a>
    </div>
  </section>
</template>

<style scoped>
#error-404 {
  .container {
    padding: 200px 40px;
    text-align: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    flex-wrap: nowrap;
    gap: 40px;
  }
}
</style>
