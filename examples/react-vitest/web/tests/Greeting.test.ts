import { test, expect } from 'vitest';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { Greeting } from '../src/Greeting.cjs';

test('renders the selected greeting', () => {
  expect(renderToStaticMarkup(React.createElement(Greeting, { name: 'policy' })))
    .toBe('<h1>Hello policy</h1>');
});

test('rejects an empty greeting', () => {
  expect(() => renderToStaticMarkup(React.createElement(Greeting, { name: '' })))
    .toThrow('A nonempty name is required');
});
