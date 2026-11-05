import { expect, test } from '@playwright/test';

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('superftp-language', 'en'));
});

test('connection settings fit side by side without scrolling at desktop size', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 740 });
  await page.goto('/');
  await page.getByRole('button', { name: 'New connection', exact: false }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByLabel('Connection timeout (seconds)', { exact: false })).toHaveValue('20');
  await expect(dialog.getByLabel('Starting remote folder')).toBeVisible();
  await expect(dialog.getByRole('textbox', { name: 'Starting local folder', exact: false })).toBeVisible();
  for (const method of ['Password', 'SSH key', 'SSH agent']) {
    await dialog.getByRole('button', { name: method, exact: true }).click();
    const geometry = await dialog.evaluate(element => {
      const body = element.querySelector('.modal-body')!;
      const details = element.querySelector('.connection-details')!.getBoundingClientRect();
      const settings = element.querySelector('.connection-settings')!.getBoundingClientRect();
      return {
        overflow: body.scrollHeight - body.clientHeight,
        dialogOverflow: element.scrollHeight - element.clientHeight,
        detailsRight: details.right,
        settingsLeft: settings.left,
      };
    });
    expect(geometry.overflow, method).toBeLessThanOrEqual(1);
    expect(geometry.dialogOverflow, method).toBeLessThanOrEqual(1);
    expect(geometry.settingsLeft).toBeGreaterThan(geometry.detailsRight);
    await expect(dialog.getByRole('button', { name: 'Save and connect' })).toBeInViewport();
  }
  await dialog.getByRole('button', { name: 'Password', exact: true }).click();
  await page.screenshot({ path: 'test-results/connection-layout-desktop.png' });
});

test('narrow and enlarged layouts keep actions visible and scroll only the body', async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 600 });
  await page.goto('/');
  await page.evaluate(() => document.documentElement.style.zoom = '1.2');
  await page.getByRole('button', { name: 'New connection', exact: false }).click();
  const dialog = page.getByRole('dialog');
  await dialog.getByRole('button', { name: 'SSH key', exact: true }).click();
  const geometry = await dialog.evaluate(element => {
    const body = element.querySelector('.modal-body')!;
    const details = element.querySelector('.connection-details')!.getBoundingClientRect();
    const settings = element.querySelector('.connection-settings')!.getBoundingClientRect();
    return {
      horizontalOverflow: body.scrollWidth - body.clientWidth,
      dialogOverflow: element.scrollHeight - element.clientHeight,
      detailsBottom: details.bottom,
      settingsTop: settings.top,
    };
  });
  expect(geometry.horizontalOverflow).toBeLessThanOrEqual(1);
  expect(geometry.dialogOverflow).toBeLessThanOrEqual(1);
  expect(geometry.settingsTop).toBeGreaterThan(geometry.detailsBottom);
  await expect(dialog.getByRole('button', { name: 'Save and connect' })).toBeInViewport();
  await dialog.getByLabel('Connection timeout (seconds)', { exact: false }).fill('60');
  await expect(dialog.getByRole('button', { name: 'Save and connect' })).toBeInViewport();
  await page.screenshot({ path: 'test-results/connection-layout-narrow.png' });
});
