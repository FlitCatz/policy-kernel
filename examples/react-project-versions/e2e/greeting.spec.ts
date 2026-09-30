import { expect, test } from '@playwright/test';
test('browser renders greeting', async ({page}) => {
  await page.goto('/');
  await expect(page.getByRole('heading')).toHaveText('Hello policy');
});
