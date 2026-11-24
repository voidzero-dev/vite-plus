process.env.FIXTURE_URL = 'from-load-env';
((globalThis as { order?: string[] }).order ??= []).push('load-env');
