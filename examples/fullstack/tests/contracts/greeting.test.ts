import { expect, test } from 'vitest';
import { decodeGreeting } from '../../frontend/src/App';

test('the live API response satisfies the frontend greeting decoder', async () => {
  const response = await fetch('http://127.0.0.1:5187/api/greeting', {
    headers: { Authorization: 'Bearer acceptance-token' },
  });

  expect(response.status).toBe(200);
  expect(decodeGreeting(await response.json())).toBe('Hello policy');
});
