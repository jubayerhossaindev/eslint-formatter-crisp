/* eslint-disable no-template-curly-in-string */

/** @type {import('semantic-release').GlobalConfig} */

// ============================================================
// Shared Release Configuration
// ============================================================

// Public npm package-এর পরিবর্তনকারী scopes।
const RELEASE_SCOPES = '{package,crisp-core,crisp-native}';

// যেসব scope-এর পরিবর্তন npm release trigger করবে না।
const NON_RELEASE_SCOPES = '{repo,ci,test,tooling,docs,maintainers}';

// Commitlint এবং Semantic Release-এর জন্য একই commit convention।
const PRESET_CONFIG = {
  types: [
    { type: 'feat', section: 'Features' },
    { type: 'fix', section: 'Bug Fixes' },
    { type: 'perf', section: 'Performance Improvements' },
    { type: 'revert', section: 'Reverts' },
    { type: 'security', section: 'Security' },
  ],
};

// ============================================================
// Semantic Release
// ============================================================

export default {
  // শুধুমাত্র main branch থেকে production release।
  branches: ['main'],

  // GitHub repository।
  repositoryUrl: 'https://github.com/jubayerhossaindev/eslint-formatter-crisp.git',

  // Release tag format: v1.0.0
  tagFormat: 'v${version}',

  plugins: [
    // ----------------------------------------------------------
    // 1. Commit Analyzer
    // ----------------------------------------------------------

    [
      '@semantic-release/commit-analyzer',
      {
        preset: 'conventionalcommits',

        // Commitlint-এর সঙ্গে একই preset configuration।
        presetConfig: PRESET_CONFIG,

        // কোন commit-এ কোন ধরনের release হবে।
        releaseRules: [
          // Breaking changes: Major release
          {
            breaking: true,
            scope: RELEASE_SCOPES,
            release: 'major',
          },

          // New features: Minor release
          {
            type: 'feat',
            scope: RELEASE_SCOPES,
            release: 'minor',
          },

          // Bug fixes: Patch release
          {
            type: 'fix',
            scope: RELEASE_SCOPES,
            release: 'patch',
          },

          // Performance improvements: Patch release
          {
            type: 'perf',
            scope: RELEASE_SCOPES,
            release: 'patch',
          },

          // Reverts: Patch release
          {
            type: 'revert',
            scope: RELEASE_SCOPES,
            release: 'patch',
          },

          // Security fixes: Patch release
          {
            type: 'security',
            scope: RELEASE_SCOPES,
            release: 'patch',
          },

          // Repository এবং development-only changes release করবে না।
          {
            scope: NON_RELEASE_SCOPES,
            release: false,
          },

          // Scope ছাড়া commit release করবে না।
          {
            scope: null,
            release: false,
          },
        ],
      },
    ],

    // ----------------------------------------------------------
    // 2. Release Notes Generator
    // ----------------------------------------------------------

    [
      '@semantic-release/release-notes-generator',
      {
        preset: 'conventionalcommits',
        presetConfig: PRESET_CONFIG,
      },
    ],

    // ----------------------------------------------------------
    // 3. Changelog Generator
    // ----------------------------------------------------------

    [
      '@semantic-release/changelog',
      {
        changelogFile: 'CHANGELOG.md',
      },
    ],

    // ----------------------------------------------------------
    // 4. npm Publisher
    // ----------------------------------------------------------

    [
      '@semantic-release/npm',
      {
        pkgRoot: '.',
        npmPublish: true,
      },
    ],

    // ----------------------------------------------------------
    // 5. Git Release Commit
    // ----------------------------------------------------------

    [
      '@semantic-release/git',
      {
        assets: ['CHANGELOG.md', 'package.json', 'package-lock.json'],

        // Commitlint-compatible automated release commit।
        message: 'chore(package): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}',
      },
    ],

    // ----------------------------------------------------------
    // 6. GitHub Release
    // ----------------------------------------------------------

    '@semantic-release/github',
  ],
};
