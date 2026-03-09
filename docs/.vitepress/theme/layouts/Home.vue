<script setup lang="ts">
import Spacer from '@components/shared/Spacer.vue';

import CoreFeature3Col from '../components/home/CoreFeature3Col.vue';
import FeatureCheck from '../components/home/FeatureCheck.vue';
import FeatureDevBuild from '../components/home/FeatureDevBuild.vue';
import FeaturePack from '../components/home/FeaturePack.vue';
import FeatureRun from '../components/home/FeatureRun.vue';
import FeatureTest from '../components/home/FeatureTest.vue';
import FeatureToolbar from '../components/home/FeatureToolbar.vue';
import Fullstack2Col from '../components/home/Fullstack2Col.vue';
import HeadingSection2 from '../components/home/HeadingSection2.vue';
import HeadingSection3 from '../components/home/HeadingSection3.vue';
import HeadingSection4 from '../components/home/HeadingSection4.vue';
import Hero from '../components/home/Hero.vue';
import HeroRive from '../components/home/HeroRive.vue';
import InstallCommand from '../components/home/InstallCommand.vue';
import PartnerLogos from '../components/home/PartnerLogos.vue';
import ProductivityGrid from '../components/home/ProductivityGrid.vue';
import Terminal from '../components/home/Terminal.vue';
import Testimonials from '../components/home/Testimonials.vue';
</script>

<template>
  <Hero />
  <Terminal />
  <InstallCommand />
  <CoreFeature3Col />
  <ProductivityGrid />
  <section id="features" data-theme="dark" class="bg-primary">
    <HeadingSection4 />
    <FeatureToolbar />
    <FeatureDevBuild />
    <FeatureCheck />
    <FeatureTest />
    <FeatureRun />
    <FeaturePack />
  </section>
  <HeadingSection3 />
  <Fullstack2Col />
  <Spacer />
  <HeroRive />
</template>
