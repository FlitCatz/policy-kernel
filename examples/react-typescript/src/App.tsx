import * as React from 'react';

const expectedGreeting = 'Hello policy';

export interface AppProps {
  payload: unknown;
}

export function decodeGreeting(payload: unknown): string {
  if (payload === null || typeof payload !== 'object' || !('message' in payload)) {
    throw new Error('Greeting payload is missing message');
  }

  const message = (payload as { message?: unknown }).message;
  if (message !== expectedGreeting) {
    throw new Error('Greeting payload contains an unexpected message');
  }

  return message;
}

export function App({ payload }: AppProps) {
  return <h1>{decodeGreeting(payload)}</h1>;
}
