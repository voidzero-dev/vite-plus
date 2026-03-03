<script setup lang="ts">
import Spacer from '@components/shared/Spacer.vue';

import CoreFeature3Col from '../components/home/CoreFeature3Col.vue';
import FeatureDevBuild from '../components/home/FeatureDevBuild.vue';
import FeatureFormat from '../components/home/FeatureFormat.vue';
import FeatureLib from '../components/home/FeatureLib.vue';
import FeatureLint from '../components/home/FeatureLint.vue';
import FeatureRun from '../components/home/FeatureRun.vue';
import FeatureTest from '../components/home/FeatureTest.vue';
import FeatureToolbar from '../components/home/FeatureToolbar.vue';
import FeatureUI from '../components/home/FeatureUI.vue';
import Fullstack2Col from '../components/home/Fullstack2Col.vue';
import HeadingSection1 from '../components/home/HeadingSection1.vue';
import HeadingSection2 from '../components/home/HeadingSection2.vue';
import HeadingSection3 from '../components/home/HeadingSection3.vue';
import HeadingSection4 from '../components/home/HeadingSection4.vue';
import Hero from '../components/home/Hero.vue';
import HeroRive from '../components/home/HeroRive.vue';
import PartnerLogos from '../components/home/PartnerLogos.vue';
import Pricing from '../components/home/Pricing.vue';
import ProductivityGrid from '../components/home/ProductivityGrid.vue';
import Terminal from '../components/home/Terminal.vue';
import Testimonials from '../components/home/Testimonials.vue';
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
