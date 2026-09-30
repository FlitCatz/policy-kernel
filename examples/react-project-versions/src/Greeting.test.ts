import { expect, test } from 'vitest';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { Greeting } from './Greeting';
test('renders the greeting', () => expect(renderToStaticMarkup(createElement(Greeting, {message:'Hello policy'}))).toBe('<h1>Hello policy</h1>'));
