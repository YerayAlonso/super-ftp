// Opt-in Windows smoke test of the actual release executable and Tauri IPC.
// Debugging is enabled only for this child process, never in shipped config.
import { spawn } from 'node:child_process';
import { mkdir, mkdtemp, rm } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { setTimeout } from 'node:timers/promises';
import { chromium, expect } from '@playwright/test';

if (process.platform !== 'win32') throw new Error('This smoke test uses Windows WebView2.');
let portBusy = false;
try {
  await fetch('http://127.0.0.1:9223/json/version');
  portBusy = true;
} catch {
  /* Unused endpoint. */
}
if (portBusy)
  throw new Error('Port 9223 is already in use. Refusing to attach to another process.');
const tempRoot = process.env.LOCALAPPDATA
  ? join(process.env.LOCALAPPDATA, 'Temp', 'opencode')
  : tmpdir();
await mkdir(tempRoot, { recursive: true });
const temporary = await mkdtemp(join(tempRoot, 'superftp-smoke-'));
const executable = resolve('src-tauri/target/release/super-ftp.exe');
const child = spawn(executable, [], {
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9223',
    WEBVIEW2_USER_DATA_FOLDER: `${temporary}/webview`,
  },
  stdio: 'ignore',
});
let browser;
let profileId;
let page;
try {
  let connected = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (child.exitCode !== null) throw new Error(`Native app exited: ${child.exitCode}`);
    try {
      const response = await fetch('http://127.0.0.1:9223/json/version');
      if (response.ok) {
        connected = true;
        break;
      }
    } catch {
      /* Wait for WebView2 startup. */
    }
    await setTimeout(200);
  }
  if (!connected) throw new Error('WebView2 debugging endpoint did not become ready.');
  browser = await chromium.connectOverCDP('http://127.0.0.1:9223');
  const context = browser.contexts()[0];
  page = context.pages()[0] ?? (await context.waitForEvent('page'));
  const errors = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await page.getByRole('heading', { name: 'Fitxers en bones mans.' }).waitFor();
  await page.getByRole('combobox', { name: 'Idioma' }).selectOption('en');
  await expect(page.getByRole('heading', { name: 'Your files, in good hands.' })).toBeVisible();
  await page.getByRole('combobox', { name: 'Language' }).selectOption('ca');
  // Wait until the startup listing is populated before editing the address;
  // a reload can otherwise finish the home request during Playwright's fill.
  await expect(page.locator('.file-pane:not(.remote-pane) .pane-footer')).not.toContainText(
    '0 elements',
  );
  await expect(page.locator('.web-preview-banner')).toHaveCount(0);
  await expect(page.getByRole('textbox', { name: 'Ruta local', exact: true })).not.toHaveValue('');
  await page.getByRole('textbox', { name: 'Ruta local', exact: true }).fill(resolve('.'));
  await expect(page.getByRole('textbox', { name: 'Ruta local', exact: true })).toHaveValue(
    resolve('.'),
  );
  await page.getByRole('textbox', { name: 'Ruta local', exact: true }).press('Enter');
  await expect(
    page
      .getByRole('region', { name: 'Fitxers locals', exact: true })
      .getByText('README.md', { exact: true }),
  ).toBeVisible();
  await page.getByRole('region', { name: 'Fitxers locals', exact: true }).getByText('README.md', { exact: true }).click({ button: 'right' });
  await expect(page.getByRole('menu', { name: 'Menú contextual' }).getByRole('menuitem', { name: 'Propietats' })).toBeVisible();
  await page.keyboard.press('Escape');
  const localOperations = await page.evaluate(async parent => {
    await window.__TAURI_INTERNALS__.invoke('create_local_directory', { parent, name: 'test-folder' });
    const listing = await window.__TAURI_INTERNALS__.invoke('list_local', { path: parent });
    const entry = listing.entries.find(item => item.name === 'test-folder');
    if (!entry) throw new Error('Missing created folder');
    await window.__TAURI_INTERNALS__.invoke('delete_local', { paths: [entry.path] });
    return !(await window.__TAURI_INTERNALS__.invoke('list_local', { path: parent })).entries.some(item => item.name === 'test-folder');
  }, temporary);
  expect(localOperations).toBe(true);
  const saved = await page.evaluate(async () => {
    return window.__TAURI_INTERNALS__.invoke('save_connection', {
      connection: {
        id: '',
        name: 'Temporary native smoke test',
        host: '127.0.0.1',
        port: 22,
        username: 'test',
        authMethod: 'password',
        keyPath: null,
        rememberSecret: false,
        remotePath: '.',
        localPath: null,
        color: '#b5e878',
      },
      secret: null,
    });
  });
  profileId = saved.id;
  const persisted = await page.evaluate(() => window.__TAURI_INTERNALS__.invoke('get_connections'));
  expect(persisted.some((item) => item.id === profileId)).toBe(true);
  await page.evaluate(
    (id) => window.__TAURI_INTERNALS__.invoke('delete_connection', { connectionId: id }),
    profileId,
  );
  profileId = null;
  const validation = await page.evaluate(async () => {
    try {
      await window.__TAURI_INTERNALS__.invoke('list_local', { path: '../outside' });
      return false;
    } catch (error) {
      return error.code === 'INVALID_PATH';
    }
  });
  expect(validation).toBe(true);
  await page.getByRole('combobox', { name: 'Idioma' }).selectOption('en');
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.getByRole('combobox', { name: 'Language' })).toHaveValue('en');
  await mkdir('test-results', { recursive: true });
  await page.screenshot({ path: 'test-results/native-windows.png' });
  expect(errors).toEqual([]);
  console.log(
    JSON.stringify(
      {
        native: 'passed',
        profilePersistence: true,
        localListing: true,
        invalidPathRejected: true,
        languagePersistence: true,
        contextMenuAndLocalOperations: true,
        rendererErrors: errors.length,
      },
      null,
      2,
    ),
  );
} finally {
  if (page && profileId) {
    await page
      .evaluate(
        (id) => window.__TAURI_INTERNALS__.invoke('delete_connection', { connectionId: id }),
        profileId,
      )
      .catch(() => {});
  }
  await browser?.close().catch(() => {});
  child.kill();
  // Only this test's isolated WebView2 cache is removed. App data is untouched
  // except the exact temporary profile created and deleted through public IPC.
  await rm(temporary, { recursive: true, force: true, maxRetries: 10, retryDelay: 300 }).catch(
    () => {},
  );
}
