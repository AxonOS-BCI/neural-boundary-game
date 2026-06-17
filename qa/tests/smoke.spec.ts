// BLOCKED_BY_ENVIRONMENT: Playwright browsers unavailable in Termux/sandbox.
// These specs run in CI (GitHub Actions ubuntu-24.04) where browsers are installed.
import { test, expect } from '@playwright/test';

test('landing loads, CTA visible, no boot error', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#boot-error')).toBeHidden({ timeout: 10_000 });
  await expect(page.locator('text=RUN BOUNDARY')).toBeVisible();
  await expect(page.locator('text=v7.3.0')).toBeVisible();
});

test('honesty line visible', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('text=No sensor')).toBeVisible();
});

test('run starts on CTA click', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#boot-error')).toBeHidden({ timeout: 10_000 });
  await page.click('[data-cmd="start"]');
  await expect(page.locator('#game-shell')).toBeVisible();
  await expect(page.locator('#landing-screen')).toBeHidden();
});

test('DOGE address present in footer', async ({ page }) => {
  await page.goto('/');
  await page.click('[data-cmd="start"]');
  await expect(page.locator('text=DMwHAhqVNWf7dyEznukxCufNS5rjuP5MTp')).toBeVisible();
});
