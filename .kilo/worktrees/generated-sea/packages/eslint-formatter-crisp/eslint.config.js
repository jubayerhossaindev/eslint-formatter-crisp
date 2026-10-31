/* eslint-disable import/no-unresolved */
/* eslint-disable import/no-extraneous-dependencies */

import eslintParser from '@babel/eslint-parser';
import { fixupConfigRules, fixupPluginRules } from '@eslint/compat';
import { FlatCompat } from '@eslint/eslintrc';
import js from '@eslint/js';
import eslintPluginImport from 'eslint-plugin-import';
import eslintPluginN from 'eslint-plugin-n';
import eslintPluginPrettierRecommended from 'eslint-plugin-prettier/recommended';
import eslintPluginPromise from 'eslint-plugin-promise';
import eslintPluginSecurity from 'eslint-plugin-security';
import eslintPluginSonarjs from 'eslint-plugin-sonarjs';
import eslintPluginUnicorn from 'eslint-plugin-unicorn';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const compat = new FlatCompat({
  baseDirectory: __dirname,
  resolvePluginsRelativeTo: __dirname,
});

export default defineConfig([
  // 1. Global Ignores (eslint.config.js সহ সব বিল্ড/কনফিগ ইগনোর)
  {
    ignores: [
      '**/dist/**',
      '**/node_modules/**',
      '**/coverage/**',
      '**/backup/**',
      '**/build/**',
      '**/.next/**',
      '**/.cache/**',
      'eslint.config.js',
      'eslint.config.*',
    ],
  },

  // 2. Base Recommended ESLint Config
  js.configs.recommended,

  // 3. Airbnb Base Compatibility Layer (Node/JS core)
  ...fixupConfigRules(compat.extends('airbnb-base')),

  // 4. Express.js Project Configuration
  {
    name: 'Expressjs Project',
    files: [
      'src/**/*.{js,mjs,cjs}',
      'test/**/*.{js,mjs,cjs}',
      'tests/**/*.{js,mjs,cjs}',
      '**/*.test.{js,mjs,cjs}',
      '**/*.spec.{js,mjs,cjs}',
      '**/*.js',
      '**/*.mjs',
      '**/*.cjs',
    ],
    ignores: [
      '**/node_modules/**',
      '**/dist/**',
      '**/coverage/**',
      'eslint.config.js',
      'eslint.config.*',
    ],

    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      parser: eslintParser,
      parserOptions: {
        requireConfigFile: false,
        babelOptions: {
          babelrc: false,
          configFile: false,
          presets: ['@babel/preset-env'],
        },
      },
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.es2022,
        Express: 'readonly',
      },
    },

    plugins: {
      import: fixupPluginRules(eslintPluginImport),
      promise: eslintPluginPromise,
      security: eslintPluginSecurity,
      sonarjs: fixupPluginRules(eslintPluginSonarjs),
      unicorn: eslintPluginUnicorn,
      n: eslintPluginN,
    },

    settings: {
      'import/resolver': {
        node: {
          extensions: ['.js', '.mjs', '.cjs'],
        },
      },
      'import/extensions': ['.js', '.mjs', '.cjs'],
    },

    rules: {
      // =========================
      // General JS Rules
      // =========================
      quotes: ['error', 'single', { avoidEscape: true }],
      semi: ['error', 'always'],
      eqeqeq: ['error', 'always', { null: 'ignore' }],
      curly: ['error', 'all'],
      'no-var': 'error',
      'prefer-const': ['error', { destructuring: 'all' }],
      'no-console': 'off',
      'no-debugger': 'error',
      'consistent-return': 'error',
      'no-else-return': 'error',
      'no-lonely-if': 'error',
      'no-nested-ternary': 'error',
      'no-param-reassign': ['error', { props: false }],
      'no-shadow': 'off',
      'no-use-before-define': 'off',
      'no-underscore-dangle': 'off',
      'no-unsafe-finally': 'error',
      'no-unused-vars': [
        'error',
        {
          vars: 'all',
          varsIgnorePattern: '^_',
          args: 'after-used',
          argsIgnorePattern: '^_',
          caughtErrors: 'all',
          caughtErrorsIgnorePattern: '^_',
          destructuredArrayIgnorePattern: '^_',
        },
      ],
      'no-multiple-empty-lines': ['error', { max: 1, maxEOF: 1 }],
      'no-duplicate-imports': 'error',
      'no-restricted-syntax': ['error', 'WithStatement', 'LabeledStatement'],
      'no-await-in-loop': 'off',
      'default-case': 'off',
      'max-classes-per-file': 'off',
      'class-methods-use-this': 'off',
      'object-shorthand': 'error',
      'prefer-template': 'error',
      'prefer-destructuring': ['warn', { array: false, object: true }],
      'func-style': ['error', 'declaration', { allowArrowFunctions: true }],
      'max-lines-per-function': ['warn', { max: 150 }],
      complexity: ['warn', 12],

      // =========================
      // Node.js & Express Rules (eslint-plugin-n)
      // =========================
      'n/no-unsupported-features/es-syntax': ['error', { ignores: ['modules'] }],
      'n/no-extraneous-import': 'error',
      'n/no-missing-import': 'error',
      'n/no-unpublished-import': 'warn',
      'n/no-sync': 'warn',

      // =========================
      // Import Plugin Rules
      // =========================
      'import/order': [
        'error',
        {
          groups: [['builtin', 'external'], 'internal', ['parent', 'sibling', 'index']],
          'newlines-between': 'always',
        },
      ],
      'import/no-unresolved': 'error',
      'import/no-extraneous-dependencies': [
        'error',
        {
          devDependencies: [
            '**/test/**',
            '**/tests/**',
            '**/*.test.{js,mjs,cjs}',
            '**/*.spec.{js,mjs,cjs}',
            'eslint.config.{js,mjs,cjs}',
          ],
        },
      ],
      'import/extensions': 'off',
      'import/newline-after-import': 'error',
      'import/no-duplicates': 'error',
      'import/no-cycle': 'warn',

      // =========================
      // Promise / Async Rules
      // =========================
      'promise/always-return': 'error',
      'promise/no-return-wrap': 'error',
      'promise/catch-or-return': 'error',
      'promise/no-nesting': 'warn',
      'promise/param-names': 'error',

      // =========================
      // Security Rules
      // =========================
      'security/detect-object-injection': 'warn',
      'security/detect-non-literal-fs-filename': 'warn',
      'security/detect-non-literal-regexp': 'warn',
      'security/detect-eval-with-expression': 'error',
      'security/detect-non-literal-require': 'warn',
      'security/detect-child-process': 'warn',

      // =========================
      // SonarJS Rules
      // =========================
      'sonarjs/no-duplicate-string': 'warn',
      'sonarjs/cognitive-complexity': ['warn', 15],
      'sonarjs/no-small-switch': 'warn',
      'sonarjs/prefer-immediate-return': 'warn',
      'sonarjs/no-identical-functions': 'warn',
      'sonarjs/no-redundant-boolean': 'warn',

      // =========================
      // Unicorn Plugin Rules
      // =========================
      'unicorn/prefer-node-protocol': 'error',
      'unicorn/prefer-string-replace-all': 'error',
      'unicorn/throw-new-error': 'error',
      'unicorn/no-useless-undefined': 'error',
      'unicorn/prefer-ternary': 'warn',

      // =========================
      // Prettier
      // =========================
      'prettier/prettier': [
        'error',
        {
          semi: true,
          singleQuote: true,
          trailingComma: 'all',
          printWidth: 100,
          tabWidth: 2,
          useTabs: false,
          arrowParens: 'always',
          bracketSpacing: true,
          bracketSameLine: false,
          endOfLine: 'lf',
        },
      ],
    },
  },

  // 5. Test Files Specific Overrides
  {
    files: [
      'test/**/*.{js,mjs,cjs}',
      'tests/**/*.{js,mjs,cjs}',
      '**/*.test.{js,mjs,cjs}',
      '**/*.spec.{js,mjs,cjs}',
    ],
    languageOptions: {
      parser: eslintParser,
      parserOptions: {
        requireConfigFile: false,
        babelOptions: {
          babelrc: false,
          configFile: false,
          presets: ['@babel/preset-env'],
        },
      },
      globals: {
        ...globals.browser,
        ...globals.node,
        ...globals.es2022,
      },
    },
    rules: {
      'no-console': 'off',
      'import/no-extraneous-dependencies': 'off',
    },
  },

  // 6. Prettier Config (MUST be last)
  eslintPluginPrettierRecommended,
]);
