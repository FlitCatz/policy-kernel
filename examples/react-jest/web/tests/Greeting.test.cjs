const React = require('react');
const { renderToStaticMarkup } = require('react-dom/server');
const { Greeting } = require('../src/Greeting.cjs');

test('renders the selected greeting', () => {
  expect(renderToStaticMarkup(React.createElement(Greeting, { name: 'policy' })))
    .toBe('<h1>Hello policy</h1>');
});

test('rejects an empty greeting', () => {
  expect(() => renderToStaticMarkup(React.createElement(Greeting, { name: '' })))
    .toThrow('A nonempty name is required');
});
