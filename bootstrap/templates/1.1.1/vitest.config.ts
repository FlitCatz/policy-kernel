export default {
  test: {
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    passWithNoTests: false,
    retry: 0,
    clearMocks: true,
  },
};
