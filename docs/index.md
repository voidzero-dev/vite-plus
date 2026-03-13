---
home: true
layout: home
theme: light
titleTemplate: The Unified Toolchain for the Web
head:
  - - script
    - id: home-theme-init
    - |
        document.documentElement.setAttribute('data-theme', 'light');
---

<script setup>
import Home from '@layouts/Home.vue'
</script>

<Home />
