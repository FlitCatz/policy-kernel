export default {
  test: {
    environment: 'node',
    include: ['tests/contracts/**/*.{test,spec}.ts'],
    passWithNoTests: false,
    retry: 0,
  },
};
