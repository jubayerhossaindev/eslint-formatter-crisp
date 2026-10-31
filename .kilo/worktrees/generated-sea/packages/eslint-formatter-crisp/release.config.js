/* eslint-disable no-template-curly-in-string */

/** @type {import('semantic-release').GlobalConfig} */
module.exports = {
  branches: ['main'],

  repositoryUrl: 'https://github.com/jubayerhossaindev/eslint-formatter-crisp.git',

  tagFormat: 'v${version}',

  plugins: [
    [
      '@semantic-release/commit-analyzer',
      {
        preset: 'conventionalcommits',
        releaseRules: [
          { breaking: true, scope: 'package', release: 'major' },
          { type: 'feat', scope: 'package', release: 'minor' },
          { type: 'fix', scope: 'package', release: 'patch' },
          { type: 'perf', scope: 'package', release: 'patch' },
          { type: 'revert', scope: 'package', release: 'patch' },
          { scope: '!package', release: false },
          { scope: null, release: false },
        ],
      },
    ],

    [
      '@semantic-release/release-notes-generator',
      {
        preset: 'conventionalcommits',
      },
    ],

    [
      '@semantic-release/changelog',
      {
        changelogFile: 'CHANGELOG.md',
      },
    ],

    [
      '@semantic-release/npm',
      {
        pkgRoot: '.',
      },
    ],

    '@semantic-release/github',

    [
      '@semantic-release/git',
      {
        assets: [
          'CHANGELOG.md',
          'package.json',
          'package-lock.json',
        ],
        message: 'chore(release): ${nextRelease.version} [skip ci]\n\n${nextRelease.notes}',
      },
    ],
  ],
};
