// A real OpenSSH server on a disposable Linux CI runner. No user server,
// ~/.ssh changes, passwords, or persistent service configuration are involved.
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir, userInfo } from 'node:os';
import { join, resolve } from 'node:path';
import { createServer } from 'node:net';

if (process.platform !== 'linux' || process.env.GITHUB_ACTIONS !== 'true') {
  throw new Error(
    'This privileged OpenSSH fixture runs only on a disposable Linux GitHub Actions runner.',
  );
}
function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: 'utf8', ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed: ${result.stderr ?? result.status}`);
  return result.stdout;
}
const directory = mkdtempSync(join(tmpdir(), 'superftp-openssh-'));
const pidFile = join(directory, 'sshd.pid');
const logFile = join(directory, 'sshd.log');
try {
  const socket = createServer();
  await new Promise((done, reject) => {
    socket.once('error', reject);
    socket.listen(0, '127.0.0.1', done);
  });
  const port = socket.address().port;
  await new Promise((done) => socket.close(done));
  const hostKey = join(directory, 'host');
  const clientKey = join(directory, 'client');
  run('ssh-keygen', ['-q', '-t', 'ed25519', '-N', '', '-f', hostKey]);
  run('ssh-keygen', ['-q', '-t', 'rsa', '-b', '2048', '-N', '', '-f', clientKey]);
  const user = userInfo().username;
  if (!/^[a-z_][a-z0-9_-]*\$?$/i.test(user)) throw new Error('Unsupported fixture username');
  const remote = join(directory, 'remote');
  mkdirSync(remote);
  const quote = (path) => `"${path.replace(/[\\"]/g, '\\$&')}"`;
  const config = join(directory, 'sshd_config');
  writeFileSync(logFile, '');
  writeFileSync(
    config,
    [
      `Port ${port}`,
      'ListenAddress 127.0.0.1',
      `HostKey ${quote(hostKey)}`,
      `PidFile ${quote(pidFile)}`,
      `AuthorizedKeysFile ${quote(`${clientKey}.pub`)}`,
      `AllowUsers ${user}`,
      'StrictModes no',
      'PubkeyAuthentication yes',
      'PasswordAuthentication no',
      'KbdInteractiveAuthentication no',
      'UsePAM yes',
      'Subsystem sftp internal-sftp',
      'LogLevel VERBOSE',
      '',
    ].join('\n'),
  );
  run('sudo', ['-n', 'mkdir', '-p', '/run/sshd']);
  run('sudo', ['-n', '/usr/sbin/sshd', '-t', '-f', config]);
  run('sudo', ['-n', '/usr/sbin/sshd', '-f', config, '-E', logFile]);
  run(
    'cargo',
    [
      'test',
      '--manifest-path',
      'src-tauri/Cargo.toml',
      '--lib',
      '--locked',
      'integration_tests::real_openssh_rsa_authentication_and_sftp_round_trip',
      '--',
      '--ignored',
      '--exact',
      '--nocapture',
    ],
    {
      stdio: 'inherit',
      env: {
        ...process.env,
        SUPERFTP_OPENSSH_PORT: String(port),
        SUPERFTP_OPENSSH_USER: user,
        SUPERFTP_OPENSSH_HOST_PUBLIC_KEY: `${hostKey}.pub`,
        SUPERFTP_OPENSSH_CLIENT_KEY: clientKey,
        SUPERFTP_OPENSSH_REMOTE: remote,
      },
    },
  );
  console.log('Real OpenSSH RSA authentication, overwrite and byte-for-byte round trip passed.');
} catch (error) {
  if (existsSync(logFile)) process.stderr.write(readFileSync(logFile, 'utf8'));
  throw error;
} finally {
  if (existsSync(pidFile)) {
    const pid = readFileSync(pidFile, 'utf8').trim();
    if (!/^[1-9][0-9]*$/.test(pid)) throw new Error('Invalid fixture sshd PID');
    run('sudo', ['-n', 'kill', '-TERM', pid]);
  }
  // The only recursive removal is the uniquely created fixture directory.
  if (!resolve(directory).startsWith(`${resolve(tmpdir())}/superftp-openssh-`))
    throw new Error('Unsafe fixture cleanup path');
  rmSync(directory, { recursive: true, force: true });
}
