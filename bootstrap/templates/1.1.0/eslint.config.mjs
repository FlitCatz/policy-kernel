// Declarative native-tool configuration; installer and execution logic are Rust.
import tsParser from '@typescript-eslint/parser';

export default [
  { ignores: ['node_modules/**', 'dist/**', '.policy-reports/**'] },
  {
    files: ['**/*.{js,jsx,mjs,cjs,ts,tsx}'],
    languageOptions: {
      parser: tsParser,
      ecmaVersion: 2022,
      sourceType: 'module',
      parserOptions: { ecmaFeatures: { jsx: true } },
    },
    rules: {
      'no-debugger': 'error',
      'no-constant-condition': 'error',
      'no-duplicate-case': 'error',
      eqeqeq: ['error', 'always'],
    },
  },
];
