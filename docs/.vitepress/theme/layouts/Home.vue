<script setup lang="ts">
import Hero from '../components/home/Hero.vue';
import PartnerLogos from '../components/home/PartnerLogos.vue';
import HeadingSection1 from '../components/home/HeadingSection1.vue';
import HeadingSection2 from '../components/home/HeadingSection2.vue';
import HeadingSection3 from '../components/home/HeadingSection3.vue';
import HeadingSection4 from '../components/home/HeadingSection4.vue';
import Terminal from '../components/home/Terminal.vue';
import CoreFeature3Col from '../components/home/CoreFeature3Col.vue';
import ProductivityGrid from '../components/home/ProductivityGrid.vue';
import FeatureToolbar from '../components/home/FeatureToolbar.vue';
import Fullstack2Col from '../components/home/Fullstack2Col.vue';
import Spacer from '@components/shared/Spacer.vue';
import Testimonials from '../components/home/Testimonials.vue';
import Pricing from '../components/home/Pricing.vue';
import FeatureDevBuild from '../components/home/FeatureDevBuild.vue';
import FeatureTest from '../components/home/FeatureTest.vue';
import FeatureLint from '../components/home/FeatureLint.vue';
import FeatureFormat from '../components/home/FeatureFormat.vue';
import FeatureRun from '../components/home/FeatureRun.vue';
import FeatureUI from '../components/home/FeatureUI.vue';
import FeatureLib from '../components/home/FeatureLib.vue';
import HeroRive from '../components/home/HeroRive.vue';
</script>

<template>
  <Hero />
  <!-- <PartnerLogos /> -->
  <!-- <HeadingSection1 /> -->
  <Terminal />
  <CoreFeature3Col />
  <!-- <HeadingSection2 /> -->
  <ProductivityGrid />
  <section id="features" data-theme="dark" class="bg-primary">
    <HeadingSection4 />
    <FeatureToolbar />
    <FeatureDevBuild />
    <FeatureTest />
    <FeatureLint />
    <FeatureFormat />
    <FeatureRun />
    <FeatureUI />
    <FeatureLib />
  </section>
  <HeadingSection3 />
  <Fullstack2Col />
  <Spacer />
  <HeroRive />
  <Spacer />
  <!--
  <Testimonials/>
  <Spacer/>
  -->
  <!-- <Pricing /> -->
</template>
