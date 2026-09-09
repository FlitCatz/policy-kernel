// The consumer provisions its real frontend/API servers and authors journey tests.
export default {
  testDir: './tests/e2e',
  forbidOnly: true,
  retries: 0,
  reporter: [['json', { outputFile: '.policy-reports/e2e.json' }]],
};
