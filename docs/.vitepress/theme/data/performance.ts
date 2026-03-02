export interface PerformanceData {
  name: string;
  percentage: number;
  time: string;
  isPrimary?: boolean;
}

export const devPerformance: PerformanceData[] = [
  {
    name: 'Vite Dev',
    percentage: 15,
    time: '102MS',
    isPrimary: true,
  },
  {
    name: 'Webpack',
    percentage: 50,
    time: '2.38S',
  },
  {
    name: 'Rspack',
    percentage: 60,
    time: '2.38S',
  },
  {
    name: 'Vite 7',
    percentage: 90,
    time: '2.38S',
  },
  {
    name: 'NextJS',
    percentage: 100,
    time: '2.38S',
  },
];

export const buildPerformance: PerformanceData[] = [
  {
    name: 'Vite Build',
    percentage: 20,
    time: '1.2S',
    isPrimary: true,
  },
  {
    name: 'Webpack',
    percentage: 75,
    time: '8.4S',
  },
  {
    name: 'Rspack',
    percentage: 45,
    time: '3.1S',
  },
  {
    name: 'Vite 7',
    percentage: 85,
    time: '7.2S',
  },
  {
    name: 'NextJS',
    percentage: 100,
    time: '9.8S',
  },
];

export const testPerformance: PerformanceData[] = [
  {
    name: 'Vite Test',
    percentage: 15,
    time: '102MS',
    isPrimary: true,
  },
  {
    name: 'Jest+SWC',
    percentage: 45,
    time: '2.38S',
  },
  {
    name: 'Jest+TS-Jest',
    percentage: 55,
    time: '2.38S',
  },
  {
    name: 'Jest+Babel',
    percentage: 75,
    time: '2.38S',
  },
];

export const lintSyntaticPerformance: PerformanceData[] = [
  {
    name: 'Syntatic Mode',
    percentage: 10,
    time: '102MS',
    isPrimary: true,
  },
  {
    name: 'ESLint',
    percentage: 50,
    time: '2.38S',
  },
  {
    name: 'Biome',
    percentage: 45,
    time: '2.38S',
  },
];

export const lintTypeAwarePerformance: PerformanceData[] = [
  {
    name: 'Type-Aware Mode',
    percentage: 25,
    time: '380MS',
    isPrimary: true,
  },
  {
    name: 'ESLint',
    percentage: 85,
    time: '4.2S',
  },
  {
    name: 'TypeScript',
    percentage: 70,
    time: '3.1S',
  },
  {
    name: 'Biome',
    percentage: 60,
    time: '2.8S',
  },
];

export const formatPerformance: PerformanceData[] = [
  {
    name: 'Vite Format',
    percentage: 10,
    time: '102MS',
    isPrimary: true,
  },
  {
    name: 'ESLint',
    percentage: 50,
    time: '2.38S',
  },
  {
    name: 'Biome',
    percentage: 45,
    time: '2.38S',
  },
];
