import * as React from 'react';
import { render, screen } from '@testing-library/react';
import '@testing-library/jest-dom/vitest';
import { expect, test } from 'vitest';
import { App } from './App';

test('renders a valid greeting payload supplied through component props', () => {
  render(<App payload={{ message: 'Hello policy' }} />);

  expect(screen.getByRole('heading', { name: 'Hello policy' })).toBeInTheDocument();
});
