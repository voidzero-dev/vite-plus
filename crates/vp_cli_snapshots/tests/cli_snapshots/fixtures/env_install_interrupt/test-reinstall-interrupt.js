const fs = require('fs');
const path = require('path');
const { spawn, spawnSync } = require('child_process');
const { promisify } = require('util');

const sleep = promisify(setTimeout);
const readyPath = path.join(process.env.VP_HOME, 'env-install-interrupt-ready');
fs.rmSync(readyPath, { force: true });

// Stage markers narrate the driver on the PTY. `snapshot = false` hides them
// on success; on a failure or timeout they name the stage that hung.
console.log('interrupt driver: start');

// Spawn the real CLI binary, not the `vp` on PATH. On Windows, PATH resolves
// to the trampoline in VP_HOME/bin, which runs current/bin/vp.exe as a child
// and waits. With the trampoline as child.pid, killInstall killed the wrong
// process first: the real vp.exe stayed alive, watched its npm child die in
// the tree sweep, and removed the partial install dir that the check-stale
// step needs ("interrupted stale package removed" flake). Spawning
// current/bin/vp directly makes child.pid the process that runs the install
// on every platform (on Unix, bin/vp is a symlink to the same binary).
const vpBinary = path.join(
  process.env.VP_HOME,
  'current',
  'bin',
  process.platform === 'win32' ? 'vp.exe' : 'vp',
);
const child = spawn(vpBinary, ['install', '-g', './long-time-install-package'], {
  env: { ...process.env, VP_TEST_INTERRUPT_INSTALL: '1' },
  stdio: 'inherit',
});
console.log(`interrupt driver: spawned vp (pid ${child.pid})`);

function killInstall() {
  if (process.platform === 'win32') {
    // Kill vp.exe first so it cannot react: while vp.exe is alive, a dying
    // postinstall child makes it treat the reinstall as failed and remove
    // the partial install dir, leaving no stale dir for the check-stale
    // step. Then kill the postinstall subtree via the PID from the ready
    // file, so no process enumeration is needed (PowerShell/CIM cold start
    // on a loaded runner can outlast the step timeout). The npm layers
    // between vp.exe and postinstall exit on their own once the postinstall
    // script dies; with vp.exe already dead, nobody reacts to that.
    let postinstallPid = '';
    try {
      postinstallPid = fs.readFileSync(readyPath, 'utf8').trim();
    } catch {
      // Ready file absent: the error paths call this without a handshake.
    }
    const killed =
      spawnSync('taskkill.exe', ['/PID', String(child.pid), '/F'], {
        stdio: 'ignore',
      }).status === 0;
    if (postinstallPid) {
      spawnSync('taskkill.exe', ['/PID', postinstallPid, '/T', '/F'], {
        stdio: 'ignore',
      });
    }
    return killed;
  }
  return child.kill('SIGKILL');
}

// The runner's step timeout reports whatever is left on the PTY screen,
// which can be blank after vp's spinner erased its line. Fail from inside
// the driver first so the failure always carries the stage markers.
const watchdog = setTimeout(() => {
  console.error('interrupt driver: watchdog fired after 45s; forcing exit');
  killInstall();
  process.exit(1);
}, 45_000);
watchdog.unref();

let interrupted = false;

(async () => {
  for (let attempt = 0; attempt < 500; attempt++) {
    if (fs.existsSync(readyPath)) {
      console.log('interrupt driver: postinstall reached, killing install');
      if (!killInstall()) {
        throw new Error('failed to interrupt reinstall');
      }
      interrupted = true;
      console.log('interrupt driver: install killed');
      return;
    }
    if (child.exitCode !== null || child.signalCode !== null) {
      throw new Error('reinstall exited before reaching postinstall');
    }
    await sleep(20);
  }
  throw new Error('timed out waiting for reinstall postinstall');
})().catch((error) => {
  console.error(error.message);
  killInstall();
  process.exitCode = 1;
});

child.on('close', (code) => {
  console.log(`interrupt driver: vp closed (code ${code})`);
  fs.rmSync(readyPath, { force: true });
  if (!interrupted) {
    process.exitCode ||= code || 1;
  }
});
