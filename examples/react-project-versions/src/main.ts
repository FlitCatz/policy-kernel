import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { Greeting } from './Greeting';
createRoot(document.getElementById('root')!).render(createElement(Greeting, {message: 'Hello policy'}));
