import assert from 'node:assert/strict';
import { appendFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';
import semanticRelease from 'semantic-release';
import { generateReleaseNotes } from './notes.mjs';

const require = createRequire(import.meta.url);
const mode = process.argv[2];
assert(['plan', 'publish'].includes(mode), 'Use release.mjs plan|publish');
assert.equal(process.env.GITHUB_REF_NAME, 'main', 'Release from main only');
const channel = process.env.RELEASE_CHANNEL;
assert(['nightly', 'production'].includes(channel), 'Choose nightly or production');
const planPath = 'target/release-plan.json';
const plan = mode === 'publish' ? JSON.parse(readFileSync(planPath, 'utf8')) : null;
if (plan) assert.equal(channel, plan.channel, 'Release channel changed');

// A rerun after interrupted publishing must not create the release a second time.
function releaseExists() {
  try {
    execFileSync('gh', ['release', 'view', plan.gitTag], { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}
const resumed = plan !== null && releaseExists();

const conventional = { preset: 'conventionalcommits' };
const plugins = [
  [require.resolve('@semantic-release/commit-analyzer'), conventional],
  [{ generateNotes: (config, context) => generateReleaseNotes(channel, config, context) }, conventional],
];
if (plan && channel === 'production') {
  plugins.push(
    [require.resolve('@semantic-release/git'), {
      assets: ['Cargo.toml', 'Cargo.lock', 'fuzz/Cargo.lock'],
      message: 'chore(release): ${nextRelease.version} [skip ci]',
    }],
    [require.resolve('@semantic-release/github'), {
      assets: ['release-assets/*'],
      successComment: false,
      failComment: false,
      releasedLabels: false,
      releaseNameTemplate: 'Serein <%= nextRelease.version %>',
    }],
  );
}
// Scheduled nightlies skip a commit that already has a nightly tag; manual dispatches still rebuild.
const nightlyUnchanged = mode === 'plan' && channel === 'nightly'
  && process.env.GITHUB_EVENT_NAME === 'schedule'
  && execFileSync('git', ['tag', '--points-at', 'HEAD', '--list', 'v*-nightly.*'], { encoding: 'utf8' }).trim() !== '';
if (nightlyUnchanged) console.log('HEAD already has a nightly release; skipping');
// Nightly assets are published from the immutable plan commit; no branch mutation needs semantic-release.
const result = nightlyUnchanged ? null : mode === 'publish' && (channel === 'nightly' || resumed)
  ? { nextRelease: { version: plan.stableVersion, gitHead: plan.gitHead } }
  : await semanticRelease({
    branches: ['main'],
    tagFormat: 'v${version}',
    plugins,
    dryRun: mode === 'plan' || channel === 'nightly',
    verifyRelease: async (_config, { nextRelease }) => {
      if (plan) {
        assert.equal(nextRelease.version, plan.stableVersion, 'Release changed while packages were building');
        assert.equal(nextRelease.gitHead, plan.gitHead, 'Release commit changed while packages were building');
      }
    },
  });
if (mode === 'plan') {
  appendFileSync(process.env.GITHUB_OUTPUT, `release=${Boolean(result)}\n`);
  if (result) {
    const { version: stableVersion, gitHead, notes } = result.nextRelease;
    const date = new Date().toISOString().slice(0, 10).replace(/-/g, '');
    const runNumber = process.env.GITHUB_RUN_NUMBER || '1';
    const version = channel === 'nightly'
      ? `${stableVersion}-nightly.${date}.${runNumber}`
      : stableVersion;
    const gitTag = `v${version}`;
    mkdirSync('target', { recursive: true });
    writeFileSync(planPath, JSON.stringify({ version, stableVersion, gitTag, gitHead, notes, channel }));
    appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\ntag=${gitTag}\n`);
  }
} else {
  assert(result, 'No release published; branch or tags changed after planning');
  if (channel === 'nightly') {
    // Nightlies do not create stable tags or version commits that consume production changes.
    if (!resumed) {
      writeFileSync('target/release-notes.md', plan.notes);
      const assets = readdirSync('release-assets')
        .map(name => `release-assets/${name}`);
      execFileSync('gh', ['release', 'create', plan.gitTag, ...assets,
        '--target', plan.gitHead, '--title', `Serein ${plan.version}`,
        '--notes-file', 'target/release-notes.md', '--prerelease', '--latest=false', '--draft'],
      { stdio: 'inherit' });
    }
    execFileSync('gh', ['release', 'edit', plan.gitTag, '--draft=false', '--prerelease', '--latest=false'],
    { stdio: 'inherit' });
  }
}
