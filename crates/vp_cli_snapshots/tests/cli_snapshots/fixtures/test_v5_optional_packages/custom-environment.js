import assert from 'node:assert/strict';
import { populateGlobal } from 'vite-plus/test/runtime';

export default {
  name: 'descriptor-environment',
  viteEnvironment: 'ssr',
  setup(global) {
    const target = {};
    const getter = () => { throw new Error('The original getter must not run'); };
    Object.defineProperty(target, 'probe', { get: getter, configurable: true });
    const { originals } = populateGlobal(target, { probe: 42 }, { additionalKeys: ['probe'] });
    assert.equal(target.probe, 42);
    assert.equal(originals.get('probe').get, getter);
    Object.defineProperty(target, 'probe', originals.get('probe'));
    assert.equal(Object.getOwnPropertyDescriptor(target, 'probe').get, getter);
    global.__descriptorCheck = true;
    return { teardown() { delete global.__descriptorCheck; } };
  },
};
