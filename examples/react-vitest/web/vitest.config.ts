import { defineConfig } from 'vitest/config';
export default defineConfig({ test: { include: ['tests/*.test.ts'], retry: 0, environment: 'node' } });
