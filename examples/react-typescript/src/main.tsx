import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';

async function renderGreeting() {
  const response = await fetch('http://127.0.0.1:5187/api/greeting', {
    headers: { Authorization: 'Bearer acceptance-token' },
  });
  if (!response.ok) {
    throw new Error(`Greeting request failed with status ${response.status}`);
  }

  const root = document.getElementById('root');
  if (root === null) {
    throw new Error('Application root is missing');
  }

  createRoot(root).render(
    <StrictMode>
      <App payload={await response.json()} />
    </StrictMode>,
  );
}

void renderGreeting();
