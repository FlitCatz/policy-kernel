const React = require('react');

function Greeting({ name }) {
  if (typeof name !== 'string' || name.trim() === '') {
    throw new Error('A nonempty name is required');
  }
  return React.createElement('h1', null, `Hello ${name}`);
}

module.exports = { Greeting };
