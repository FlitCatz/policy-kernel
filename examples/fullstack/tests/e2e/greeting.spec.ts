import { expect, test } from '@playwright/test';

test('renders the greeting returned by the live API', async ({ page }) => {
  await page.goto('http://127.0.0.1:5186');

  await expect(page.getByRole('heading', { name: 'Hello policy' })).toBeVisible();
});
