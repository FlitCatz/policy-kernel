import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { Greeting } from './Greeting';
const data = await fetch('/api/greeting').then(r => r.json());
createRoot(document.getElementById('root')!).render(createElement(Greeting, data));
