import { expect, test, type Page } from '@playwright/test';

async function chooseLanguage(page: Page, language: string) {
  await page.locator('.preferences-trigger').click();
  await page.getByRole('combobox').selectOption(language);
  await page.keyboard.press('Escape');
}

test.beforeEach(async ({ page }, testInfo) => {
  if (testInfo.title.includes('falls back safely') || testInfo.title === 'English is the default language') return;
  await page.addInitScript(() => {
    if (!localStorage.getItem('superftp-language')) localStorage.setItem('superftp-language', 'ca');
  });
});

test('English is the default language', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await page.locator('.preferences-trigger').click();
  await expect(page.getByRole('combobox')).toHaveValue('en');
});

test('third-party notices are accessible in the help dialog', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Ajuda', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByRole('button', { name: 'Comprova si hi ha actualitzacions' })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Llicències de tercers' }).click();
  const notices = dialog.getByRole('region', { name: 'Avisos de llicències de tercers (inventari de Windows)' });
  await expect(notices).toContainText('SIL OPEN FONT LICENSE');
  await expect(notices).toContainText('Lucide Icons and Contributors');
  await expect(notices).toContainText('option-ext 0.2.0');
  await expect(notices).toContainText('https://crates.io/api/v1/crates/option-ext/0.2.0/download');
});

test('startup checks and manual checks respect preview selection', async ({ page }) => {
  await mockDesktop(page);
  await page.goto('/');
  await chooseLanguage(page, 'en');
  await page.getByRole('button', { name: 'Preferences', exact: true }).click();
  const dialog = page.getByRole('dialog');
  await expect(dialog.getByRole('region', { name: 'Updates' })).toBeVisible();
  await expect.poll(() => page.evaluate(() => (window as any).__superftpTest.calls.filter((call: any) => call.command === 'check_updates').length)).toBe(1);
  await dialog.getByRole('button', { name: 'Check for updates' }).click();
  await expect(dialog.getByRole('status')).toHaveText('No matching public releases were found.');
  await dialog.getByLabel('Include preview releases').check();
  await dialog.getByRole('button', { name: 'Check for updates' }).click();
  await expect(dialog.getByRole('status')).toContainText('A newer version is available: 0.2.0');
  await expect(dialog.getByRole('status')).toContainText('Preview release');
  await expect(dialog.getByText('Preview test notes')).toBeVisible();
  await dialog.getByRole('button', { name: 'Open GitHub releases' }).click();
  const calls = await page.evaluate(() => (window as any).__superftpTest.calls.filter((call: any) => ['check_updates', 'open_releases_page'].includes(call.command)));
  expect(calls).toEqual([
    { command: 'check_updates', args: { includePrereleases: false } },
    { command: 'check_updates', args: { includePrereleases: false } },
    { command: 'check_updates', args: { includePrereleases: true } },
    { command: 'open_releases_page', args: {} },
  ]);
  await page.evaluate(() => {
    (window as any).__superftpTest.setTransfer({ id: 'waiting', sessionId: 'session-1', name: 'photo.png', direction: 'upload', source: 'C:\\Test\\photo.png', destination: '/remote', status: 'awaitingDecision', bytesTransferred: 0, totalBytes: 1, filesTotal: 1, filesDone: 0, skipped: 0, bytesPerSecond: 0, error: null,
      conflict: { id: 'waiting-conflict', source: 'C:\\Test\\photo.png', destination: '/remote/photo.png', name: 'photo.png', direction: 'upload', sourceSize: 1, destinationSize: 1, destinationModified: null } });
  });
  await expect(page.getByRole('dialog', { name: 'The file already exists' })).toHaveCount(0);
  await dialog.getByRole('button', { name: 'Close', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'The file already exists' })).toBeVisible();
});

test('startup announces a newer preview when previews were selected', async ({ page }) => {
  await mockDesktop(page);
  await page.addInitScript(() => {
    localStorage.setItem('superftp-language', 'en');
    localStorage.setItem('superftp-include-previews', 'true');
  });
  await page.goto('/');
  const toast = page.locator('.toast').filter({ hasText: 'A newer version is available: 0.2.0' });
  await expect(toast).toBeVisible();
  await page.evaluate(() => { (window as any).__superftpTest.holdInstall = true; });
  await toast.getByRole('button', { name: 'Install update' }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__superftpTest.calls.some((call: any) => call.command === 'install_signed_update' && call.args.expectedVersion === '0.2.0' && call.args.includePrereleases))).toBe(true);
  await expect(toast.getByRole('button', { name: 'Installing update…' })).toBeDisabled();
  await toast.getByRole('button', { name: 'Updates' }).click();
  const preferences = page.getByRole('dialog', { name: 'Preferences' });
  await expect(preferences.getByLabel('Include preview releases')).toBeChecked();
  await expect(preferences.getByRole('button', { name: 'Installing update…' })).toBeDisabled();
  await page.evaluate(() => (window as any).__superftpTest.finishInstall());
  await expect(preferences.getByRole('button', { name: 'Install update' })).toBeEnabled();
});

test('interface text size scales, persists and resets', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Preferències' }).click();
  const increase = page.getByRole('button', { name: 'Augmenta la mida del text' });
  const reset = page.getByRole('button', { name: 'Restableix la mida del text' });
  const body = page.locator('body');
  const original = await body.evaluate(element => parseFloat(getComputedStyle(element).fontSize) * parseFloat(getComputedStyle(document.documentElement).zoom));
  await increase.click();
  await increase.click();
  await expect(reset).toHaveText('120%');
  await expect.poll(() => page.evaluate(() => localStorage.getItem('superftp-font-size'))).toBe('120');
  const enlarged = await body.evaluate(element => parseFloat(getComputedStyle(element).fontSize) * parseFloat(getComputedStyle(document.documentElement).zoom));
  expect(enlarged).toBeGreaterThan(original);
  await page.reload();
  await page.getByRole('button', { name: 'Preferències' }).click();
  await expect(reset).toHaveText('120%');
  await page.getByRole('dialog').getByRole('button', { name: 'Tanca', exact: true }).click();
  await page.getByRole('button', { name: 'Nova connexió', exact: false }).click();
  await expect(page.getByRole('dialog')).toBeVisible();
  await page.keyboard.press('Escape');
  await page.getByRole('button', { name: 'Preferències' }).click();
  await reset.click();
  await expect(reset).toHaveText('100%');
});

test('file context menus use the clicked item, preserve selections and confirm deletion', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  await mockDesktop(page);
  await page.goto('/');
  const local = page.getByRole('region', { name: 'Fitxers locals', exact: true });
  await expect(local.getByText(/^5[.\s]?000 elements$/)).toBeVisible();
  await local.getByLabel('Filtra els fitxers locals').fill('fitxer-2.txt');
  await local.getByRole('row', { name: /fitxer-2.txt/ }).click({ button: 'right' });
  const menu = page.getByRole('menu', { name: 'Menú contextual' });
  await expect(menu.getByRole('menuitem', { name: 'Puja la selecció' })).toBeDisabled();
  await menu.getByRole('menuitem', { name: 'Propietats' }).click();
  const properties = page.getByRole('dialog', { name: 'Propietats' });
  await expect(properties).toContainText('fitxer-2.txt');
  await expect(properties).toContainText('C:\\Test\\fitxer-2.txt');
  await properties.getByRole('button', { name: 'Tanca', exact: true }).click();
  await local.getByRole('row', { name: /fitxer-2.txt/ }).click({ button: 'right' });
  await menu.getByRole('menuitem', { name: 'Elimina', exact: true }).click();
  const confirmation = page.getByRole('dialog', { name: 'Elimina aquest element?' });
  await expect(confirmation).toContainText('permanentment');
  expect(
    await page.evaluate(() =>
      (window as any).__superftpTest.calls.some((call: any) => call.command === 'delete_local'),
    ),
  ).toBe(false);
  await confirmation.getByRole('button', { name: 'Elimina de l’ordinador' }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__superftpTest.calls.findLast(
            (call: any) => call.command === 'delete_local',
          )?.args.paths,
      ),
    )
    .toEqual(['C:\\Test\\fitxer-2.txt']);

  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  const remote = page.getByRole('region', { name: 'Fitxers remots', exact: true });
  await expect(remote.getByRole('row', { name: /readme.md/ })).toBeVisible();
  await remote.getByRole('row', { name: /readme.md/ }).click({ button: 'right' });
  await expect(menu.getByRole('menuitem', { name: 'Baixa la selecció' })).toBeEnabled();
  await menu.getByRole('menuitem', { name: 'Reanomena' }).click();
  const rename = page.getByRole('dialog', { name: 'Un nom nou.' });
  await expect(rename.getByLabel('Nom de l’element')).toHaveValue('readme.md');
  await rename.getByRole('button', { name: 'Cancel·la' }).click();
  await remote.getByRole('row', { name: /documents/ }).click({ button: 'right' });
  await menu.getByRole('menuitem', { name: 'Obre', exact: true }).click();
  await expect(page.getByLabel('Ruta remota', { exact: true })).toHaveValue('/home/test/documents');
  expect(errors).toEqual([]);
});

test('background menus create folders in the correct pane and support keyboard dismissal', async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto('/');
  const local = page.getByRole('region', { name: 'Fitxers locals', exact: true });
  await expect(local.getByText(/^5[.\s]?000 elements$/)).toBeVisible();
  await local.getByLabel('Filtra els fitxers locals').fill('not-present');
  await local.locator('.file-scroll').click({ button: 'right' });
  let menu = page.getByRole('menu', { name: 'Menú contextual' });
  await expect(menu.getByRole('menuitem', { name: 'Crea una carpeta' })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(menu.getByRole('menuitem', { name: 'Actualitza la carpeta' })).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(menu).toHaveCount(0);
  await local.locator('.file-scroll').click({ button: 'right' });
  await menu.getByRole('menuitem', { name: 'Crea una carpeta' }).click();
  let dialog = page.getByRole('dialog', { name: 'Una carpeta nova.' });
  await dialog.getByLabel('Nom de la carpeta').fill('projecte');
  await dialog.getByRole('button', { name: 'Crea la carpeta' }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__superftpTest.calls.findLast(
            (call: any) => call.command === 'create_local_directory',
          )?.args,
      ),
    )
    .toEqual({ parent: 'C:\\Test', name: 'projecte' });
  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  const remote = page.getByRole('region', { name: 'Fitxers remots', exact: true });
  await expect(remote.getByRole('row', { name: /readme.md/ })).toBeVisible();
  await remote.getByLabel('Filtra els fitxers remots').fill('not-present');
  await remote.locator('.file-scroll').click({ button: 'right' });
  menu = page.getByRole('menu', { name: 'Menú contextual' });
  await menu.getByRole('menuitem', { name: 'Crea una carpeta' }).click();
  dialog = page.getByRole('dialog', { name: 'Una carpeta nova.' });
  await dialog.getByLabel('Nom de la carpeta').fill('remot');
  await dialog.getByRole('button', { name: 'Crea la carpeta' }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__superftpTest.calls.findLast(
            (call: any) => call.command === 'create_remote_directory',
          )?.args,
      ),
    )
    .toEqual({ sessionId: 'session-1', parent: '/home/test', name: 'remot' });
});

test('right-click preserves multi-selection and never offers single-item properties for it', async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto('/');
  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  const remote = page.getByRole('region', { name: 'Fitxers remots', exact: true });
  const directory = remote.getByRole('row', { name: /documents/ });
  const file = remote.getByRole('row', { name: /readme.md/ });
  await directory.click();
  await file.click({ modifiers: ['Control'] });
  await file.click({ button: 'right' });
  const menu = page.getByRole('menu', { name: 'Menú contextual' });
  await expect(menu.getByRole('menuitem', { name: 'Propietats' })).toHaveCount(0);
  await expect(menu.getByRole('menuitem', { name: 'Reanomena' })).toHaveCount(0);
  await menu.getByRole('menuitem', { name: 'Elimina', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'Elimina aquests elements?' });
  await expect(dialog).toContainText('2 elements');
  await dialog.getByRole('button', { name: 'Elimina del servidor' }).click();
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as any).__superftpTest.calls.findLast(
            (call: any) => call.command === 'delete_remote',
          )?.args.paths,
      ),
    )
    .toEqual(['/home/test/documents', '/home/test/readme.md']);
});

test('right-click suppresses browser menu but preserves text editing menus', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    (window as any).contextMenuChecks = [];
    document.addEventListener('contextmenu', (event) => {
      (window as any).contextMenuChecks.push({
        target: (event.target as Element).tagName,
        prevented: event.defaultPrevented,
      });
    });
  });
  await page.locator('.workspace-intro h1').click({ button: 'right' });
  await page.locator('.new-connection').click({ button: 'right' });
  await page.getByRole('textbox', { name: 'Cerca connexions' }).click({ button: 'right' });
  await page.locator('.new-connection').click();
  await page.getByLabel('Nom de la connexió', { exact: true }).click({ button: 'right' });
  await page.evaluate(() => {
    const field = document.querySelector<HTMLInputElement>('input[autocomplete="new-password"]')!;
    field.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true }));
  });
  expect(await page.evaluate(() => (window as any).contextMenuChecks)).toEqual([
    { target: 'H1', prevented: true },
    { target: 'BUTTON', prevented: true },
    { target: 'INPUT', prevented: false },
    { target: 'INPUT', prevented: false },
    { target: 'INPUT', prevented: false },
  ]);
});

// Test-only IPC fixture. No mocks, saved sample profiles, or fake transfers
// are included in the shipped application. Real SSH is tested in Rust.
async function mockDesktop(page: Page, challenge = false) {
  await page.addInitScript(
    ({ challenge }) => {
      const app = window as any;
      app.isTauri = true;
      const callbacks = new Map<number, (data: unknown) => void>();
      const listeners = new Map<string, number[]>();
      let nextCallback = 1;
      let nextSession = 1;
      let nextTransfer = 1;
      let trusted = !challenge;
      const profiles = [
        {
          id: 'test-a',
          name: 'Entorn de proves',
          host: 'proves.example.test',
          port: 22,
          username: 'test',
          authMethod: 'password',
          keyPath: null,
          rememberSecret: false,
          remotePath: '.',
          localPath: null,
          color: '#b5e878',
        },
        {
          id: 'test-b',
          name: 'Còpies de seguretat',
          host: 'backup.example.test',
          port: 2222,
          username: 'test',
          authMethod: 'agent',
          keyPath: null,
          rememberSecret: false,
          remotePath: '.',
          localPath: null,
          color: '#7cb7ff',
        },
      ];
      const calls: { command: string; args: any }[] = [];
      const tasks: any[] = [];
      function emit(event: string, payload: unknown) {
        for (const handler of listeners.get(event) ?? [])
          callbacks.get(handler)?.({ event, id: handler, payload });
      }
      app.__superftpTest = { calls, emit, setTransfer(task: any) { const index = tasks.findIndex(item => item.id === task.id); if (index >= 0) tasks[index] = task; else tasks.push(task); emit('transfer-update', task); } };
      app.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      app.__TAURI_INTERNALS__ = {
        metadata: {
          currentWindow: { label: 'main' },
          currentWebview: { label: 'main', windowLabel: 'main' },
        },
        transformCallback(callback: (data: unknown) => void) {
          const id = nextCallback++;
          callbacks.set(id, callback);
          return id;
        },
        unregisterCallback(id: number) {
          callbacks.delete(id);
        },
        async invoke(command: string, args: any = {}) {
          calls.push({ command, args });
          if (command === 'plugin:event|listen') {
            listeners.set(args.event, [...(listeners.get(args.event) ?? []), args.handler]);
            return args.handler;
          }
          if (command === 'plugin:event|unlisten') return;
          if (command === 'get_connections') return profiles;
          if (command === 'check_updates') return args.includePrereleases
            ? { currentVersion: '0.1.0', latestVersion: '0.2.0', updateAvailable: true, prerelease: true, releaseNotes: 'Preview test notes' }
            : { currentVersion: '0.1.0', latestVersion: null, updateAvailable: false, prerelease: false, releaseNotes: null };
          if (command === 'check_signed_update') return args.includePrereleases ? '0.2.0' : null;
          if (command === 'install_signed_update') {
            if (app.__superftpTest.holdInstall) await new Promise<void>(resolve => { app.__superftpTest.finishInstall = resolve; });
            return;
          }
          if (command === 'open_releases_page') return;
          if (command === 'plugin:dialog|open') return 'C:\\Test';
          if (
            [
              'create_local_directory',
              'delete_local',
              'create_remote_directory',
              'delete_remote',
              'rename_remote',
            ].includes(command)
          )
            return;
          if (command === 'get_transfers') return tasks;
          if (command === 'get_local_home') return 'C:\\Test';
          if (command === 'list_local')
            return {
              path: args.path,
              parent: args.path === 'C:\\Test' ? null : 'C:\\Test',
              entries: Array.from({ length: 5000 }, (_, i) => ({
                name: `fitxer-${i}.txt`,
                path: `${args.path}\\fitxer-${i}.txt`,
                kind: 'file',
                size: i * 16,
                modified: 1780000000,
                permissions: null,
                hidden: false,
              })),
            };
          if (command === 'connect_session') {
            if (!trusted)
              throw {
                code: 'HOST_KEY_UNKNOWN',
                message: 'Verify the server',
                hostKey: {
                  id: 'challenge',
                  host: 'proves.example.test',
                  port: 22,
                  algorithm: 'ssh-ed25519',
                  fingerprint: 'SHA256:TEST_ONLY_NOT_A_REAL_HOST_KEY',
                  previousFingerprint: null,
                  changed: false,
                },
              };
            return {
              id: `session-${nextSession++}`,
              connectionId: args.connectionId,
              homePath: '/home/test',
              fingerprint: 'SHA256:TEST',
            };
          }
          if (command === 'trust_host_key') {
            trusted = true;
            return;
          }
          if (command === 'list_remote' && args.path === '/blocked')
            throw { code: 'INVALID_PATH', message: 'The remote path is invalid.' };
          if (command === 'list_remote')
            return {
              path: args.path,
              parent: args.path === '/' ? null : '/',
              entries: [
                {
                  name: 'documents',
                  path: `${args.path}/documents`,
                  kind: 'directory',
                  size: 0,
                  modified: 1780000000,
                  permissions: 0o40755,
                  hidden: false,
                },
                {
                  name: 'readme.md',
                  path: `${args.path}/readme.md`,
                  kind: 'file',
                  size: 2048,
                  modified: 1780000000,
                  permissions: 0o100644,
                  hidden: false,
                },
              ],
            };
          if (command === 'enqueue_transfers') {
            const result = args.sources.map((source: string) => ({
              id: `transfer-${nextTransfer++}`,
              sessionId: args.sessionId,
              name: source.split(/[/\\]/).pop(),
              direction: args.direction,
              source,
              destination: args.destination,
              status: 'queued',
              bytesTransferred: 0,
              totalBytes: 0,
              filesTotal: 0,
              filesDone: 0,
              skipped: 0,
              bytesPerSecond: 0,
              error: null,
            }));
            tasks.push(...result);
            return result;
          }
          if (command === 'cancel_transfer') {
            const task = tasks.find((task) => task.id === args.transferId);
            task.status = 'cancelled';
            emit('transfer-update', task);
            return;
          }
          if (command === 'resolve_transfer_conflict') {
            const task = tasks.find((task) => task.id === args.transferId);
            if (!task || task.conflict?.id !== args.conflictId) throw { code: 'CONFLICT_EXPIRED', message: 'This decision is no longer available.' };
            task.conflict = null;
            task.status = args.decision.action === 'cancel' ? 'cancelled' : 'transferring';
            emit('transfer-update', { ...task });
            return;
          }
          if (command === 'clear_finished_transfers' || command === 'disconnect_session') return;
          throw new Error(`Unexpected command in test: ${command}`);
        },
      };
    },
    { challenge },
  );
}

const languageCases = [
  [
    'ca',
    'Nova connexió',
    'Fitxers en bones mans.',
    'Servidor remot',
    'Nom de la connexió',
    'Desa',
    'Obre l’aplicació d’escriptori',
  ],
  [
    'es',
    'Nueva conexión',
    'Archivos en buenas manos.',
    'Servidor remoto',
    'Nombre de la conexión',
    'Guardar',
    'Abre la aplicación de escritorio',
  ],
  [
    'en',
    'New connection',
    'Your files, in good hands.',
    'Remote server',
    'Connection name',
    'Save',
    'Open the desktop app',
  ],
  [
    'fr',
    'Nouvelle connexion',
    'Vos fichiers, entre de bonnes mains.',
    'Serveur distant',
    'Nom de la connexion',
    'Enregistrer',
    'Ouvrez l’application de bureau',
  ],
  [
    'de',
    'Neue Verbindung',
    'Deine Dateien in guten Händen.',
    'Remote-Server',
    'Verbindungsname',
    'Speichern',
    'Öffne die Desktop-App',
  ],
  [
    'it',
    'Nuova connessione',
    'File in buone mani.',
    'Server remoto',
    'Nome della connessione',
    'Salva',
    'Apri l’app desktop',
  ],
  [
    'pt',
    'Nova ligação',
    'Ficheiros em boas mãos.',
    'Servidor remoto',
    'Nome da ligação',
    'Guardar',
    'Abre a aplicação',
  ],
  [
    'nl',
    'Nieuwe verbinding',
    'Je bestanden in goede handen.',
    'Externe server',
    'Verbindingsnaam',
    'Opslaan',
    'Open de desktopapp',
  ],
] as const;

test('conflicts are queued, localized and resolved without losing the requested destination', async ({ page }) => {
  await mockDesktop(page);
  await page.goto('/');
  await chooseLanguage(page, 'en');
  await page.evaluate(() => {
    const task = { id: 'conflict-1', sessionId: 'session-1', name: 'photo.png', direction: 'upload', source: 'C:\\Test\\photo.png', destination: '/remote', status: 'awaitingDecision', bytesTransferred: 0, totalBytes: 100, filesTotal: 1, filesDone: 0, skipped: 0, bytesPerSecond: 0, error: null,
      conflict: { id: 'one', source: 'C:\\Test\\photo.png', destination: '/remote/photo.png', name: 'photo.png', direction: 'upload', sourceSize: 100, destinationSize: 200, destinationModified: null } };
    (window as any).__superftpTest.setTransfer(task);
    (window as any).__superftpTest.setTransfer({ ...task, id: 'conflict-2', name: 'other.txt', direction: 'download', conflict: { ...task.conflict, id: 'two', name: 'other.txt', direction: 'download', destination: 'C:\\Test\\other.txt' } });
  });
  const first = page.getByRole('dialog', { name: 'The file already exists' });
  await expect(first).toContainText('/remote/photo.png');
  await first.getByRole('button', { name: 'Rename the new file' }).click();
  await first.getByLabel('New name').fill('photo (new).png');
  await first.getByRole('button', { name: 'Continue with this name' }).click();
  await expect.poll(() => page.evaluate(() => (window as any).__superftpTest.calls.find((call: any) => call.command === 'resolve_transfer_conflict')?.args)).toEqual({ transferId: 'conflict-1', conflictId: 'one', decision: { action: 'rename', name: 'photo (new).png' } });
  await expect(first).toContainText('C:\\Test\\other.txt');
  await first.getByRole('button', { name: 'Skip this file' }).click();
  await expect(first).toHaveCount(0);
  expect(await page.evaluate(() => (window as any).__superftpTest.calls.filter((call: any) => call.command === 'resolve_transfer_conflict')[1].args.decision)).toEqual({ action: 'skip' });
});

for (const [language, newConnection, heading, remote, nameLabel, save, error] of languageCases) {
  test(`language ${language}: live UI, forms, error messages and persistence`, async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', (error) => errors.push(error.message));
    page.on('console', (message) => {
      if (message.text().includes('[svelte]')) errors.push(message.text());
    });
    await page.goto('/');
    await chooseLanguage(page, language);
    await expect(page.locator('html')).toHaveAttribute('lang', language);
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(heading);
    await expect(page.getByRole('heading', { name: remote, exact: true })).toBeVisible();
    await expect(page.locator('.new-connection')).toContainText(newConnection);
    expect(await page.evaluate(() => localStorage.getItem('superftp-language'))).toBe(language);
    await page.reload();
    await expect(page.getByRole('heading', { level: 1 })).toHaveText(heading);
    await page.locator('.preferences-trigger').click();
    await expect(page.getByRole('combobox')).toHaveValue(language);
    await page.keyboard.press('Escape');
    await page.locator('.new-connection').click();
    const dialog = page.getByRole('dialog');
    await dialog.getByLabel(nameLabel, { exact: true }).fill('Profile <test> {name}');
    await dialog.locator('input[placeholder="sftp.exemple.cat"]').fill('example.test');
    await dialog.locator('input[autocomplete="username"]').fill('test');
    await expect(dialog.locator('button[value="save"]')).toHaveText(save);
    await dialog.locator('button[value="save"]').click();
    await expect(dialog.getByRole('alert')).toContainText(error);
    await expect(dialog.getByLabel(nameLabel, { exact: true })).toHaveValue(
      'Profile <test> {name}',
    );
    if (language === 'de' || language === 'en')
      await page.screenshot({ path: `test-results/language-${language}.png` });
    expect(errors).toEqual([]);
  });
}

test('changing language preserves live sessions, localizes existing errors and uses locale formatting', async ({
  page,
}) => {
  await mockDesktop(page);
  await page.goto('/');
  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/home/test',
  );
  await chooseLanguage(page, 'en');
  await expect(page.getByRole('tab', { name: 'Entorn de proves' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await page.getByRole('button', { name: 'Choose a local folder', exact: true }).click();
  const pickerTitle = await page.evaluate(
    () =>
      (window as any).__superftpTest.calls.findLast(
        (call: any) => call.command === 'plugin:dialog|open',
      ).args.options.title,
  );
  expect(pickerTitle).toBe('Select a folder');
  const local = page.getByRole('region', { name: 'Local files', exact: true });
  await local.getByRole('textbox', { name: 'Filter local files' }).fill('fitxer-96.txt');
  await expect(local.locator('.file-row .size-cell')).toHaveText('1.5 KB');
  await page.getByRole('textbox', { name: 'Remote path', exact: true }).fill('/blocked');
  await page.getByRole('textbox', { name: 'Remote path', exact: true }).press('Enter');
  await expect(page.getByRole('alert')).toHaveText('The remote path is invalid.');
  await chooseLanguage(page, 'es');
  await expect(page.getByRole('alert')).toHaveText('La ruta remota no es válida.');
  await expect(
    page
      .getByRole('region', { name: 'Archivos locales', exact: true })
      .locator('.file-row .size-cell'),
  ).toHaveText('1,5 KB');
  await expect(page.getByRole('tab', { name: 'Entorn de proves' })).toHaveAttribute(
    'aria-selected',
    'true',
  );
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/blocked',
  );
  expect(
    await page.evaluate(
      () =>
        (window as any).__superftpTest.calls.filter(
          (call: any) => call.command === 'connect_session',
        ).length,
    ),
  ).toBe(1);
  expect(
    await page.evaluate(() =>
      (window as any).__superftpTest.calls.some(
        (call: any) => call.command === 'disconnect_session',
      ),
    ),
  ).toBe(false);
});

test('an unsupported saved language falls back safely to English', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.setItem('superftp-language', '../../unknown'));
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en');
  await expect(page.getByRole('heading', { level: 1 })).toHaveText('Your files, in good hands.');
});

test('web preview is honest, theme works, and secrets are not persisted', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.text().includes('[svelte]')) errors.push(message.text());
  });
  await page.goto('/');
  await expect(page.getByText('Vista web de la interfície.', { exact: false })).toBeVisible();
  await page.screenshot({ path: 'test-results/workspace-dark.png' });
  await page.locator('.preferences-trigger').click();
  await page.getByRole('button', { name: 'Activa el tema clar' }).click();
  await page.keyboard.press('Escape');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.screenshot({ path: 'test-results/workspace-light.png' });
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.getByRole('button', { name: 'Nova connexió', exact: false }).click();
  await page.getByLabel('Nom de la connexió').fill('Prova');
  await page.getByLabel('Servidor', { exact: true }).fill('example.test');
  await page.getByLabel('Usuari', { exact: true }).fill('test');
  await page.locator('input[type="password"]').fill('TEST-ONLY-DO-NOT-PERSIST');
  await page.getByRole('button', { name: 'Desa', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Obre l’aplicació d’escriptori');
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain('TEST-ONLY');
  await page.screenshot({ path: 'test-results/connection-dialog.png' });
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('host trust requires explicit verification before connecting', async ({ page }) => {
  await mockDesktop(page, true);
  await page.goto('/');
  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  await expect(page.getByRole('dialog')).toContainText('Coneguem aquest servidor.');
  await expect(page.getByRole('button', { name: 'Confia i connecta' })).toBeDisabled();
  await page.getByRole('checkbox', { name: 'He verificat l’empremta' }).check();
  await page.getByRole('button', { name: 'Confia i connecta' }).click();
  await expect(page.getByRole('tab', { name: 'Entorn de proves' })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/home/test',
  );
});

test('the connect action remains reachable in a compact desktop window', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 740 });
  await page.goto('/');
  await expect(
    page.getByRole('button', { name: 'Connecta un servidor', exact: true }),
  ).toBeInViewport({ ratio: 1 });
  await page.getByRole('button', { name: 'Transferències 0', exact: true }).click();
  await expect(page.locator('.transfer-panel')).toHaveClass(/collapsed/);
});

test('sessions are independent, listings virtualize, and both drag paths enqueue uploads', async ({
  page,
}) => {
  const errors: string[] = [];
  page.on('pageerror', (error) => errors.push(error.message));
  page.on('console', (message) => {
    if (message.text().includes('[svelte]')) errors.push(message.text());
  });
  await mockDesktop(page);
  await page.goto('/');
  const local = page.getByRole('region', { name: 'Fitxers locals', exact: true });
  await expect(local.getByText(/^5[.\s]?000 elements$/)).toBeVisible();
  expect(await local.locator('.file-row').count()).toBeLessThan(60);
  await page.locator('.connection-main').filter({ hasText: 'Entorn de proves' }).click();
  await expect(page.getByRole('tab', { name: 'Entorn de proves' })).toBeVisible();
  await page.getByRole('textbox', { name: 'Ruta remota', exact: true }).fill('/first-session');
  await page.getByRole('textbox', { name: 'Ruta remota', exact: true }).press('Enter');
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/first-session',
  );
  await page.locator('.connection-main').filter({ hasText: 'Còpies de seguretat' }).click();
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/home/test',
  );
  await page.getByRole('tab', { name: 'Entorn de proves' }).click();
  await expect(page.getByRole('textbox', { name: 'Ruta remota', exact: true })).toHaveValue(
    '/first-session',
  );
  await local.getByRole('textbox', { name: 'Filtra els fitxers locals' }).fill('fitxer-2.txt');
  await expect(local.locator('.file-row')).toHaveCount(1);
  const source = await local.locator('.file-row').boundingBox();
  const target = await page.locator('[data-drop-zone="remote"]').boundingBox();
  if (!source || !target) throw new Error('Missing drag regions');
  await page.mouse.move(source.x + 40, source.y + 20);
  await page.mouse.down();
  await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2, { steps: 12 });
  await page.mouse.up();
  await expect(page.locator('.transfer-row')).toHaveCount(1);
  await local.locator('.file-row').click();
  await expect(local.locator('.file-row')).toHaveClass(/selected/);
  await page.evaluate(
    ({ x, y }) =>
      (window as any).__superftpTest.emit('tauri://drag-drop', {
        paths: ['C:\\Test\\external.txt'],
        position: { x: x * devicePixelRatio, y: y * devicePixelRatio },
      }),
    { x: target.x + 50, y: target.y + 80 },
  );
  await expect(page.locator('.transfer-row')).toHaveCount(2);
  await page.getByRole('button', { name: 'Cancel·la fitxer-2.txt' }).click();
  await expect(page.locator('.transfer-row').filter({ hasText: 'fitxer-2.txt' })).toContainText(
    'Cancel·lada',
  );
  await page.screenshot({ path: 'test-results/connected-workspace.png' });
  expect(errors).toEqual([]);
});
