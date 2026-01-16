---
name: Conventional Commits
description: Ensures all commit messages and PR titles follow the Conventional Commits specification.
---

# Conventional Commits Standard

This skill ensures that project history is clean, readable, and machine-parseable.

## Format Structure

Messages must match the regex:
`^(build|chore|ci|docs|feat|fix|perf|refactor|revert|style|test)(\([a-z0-9-]+\))?!?: .+$`

## Types

- **feat**: A new feature (correlates with MINOR in SemVer).
- **fix**: A bug fix (correlates with PATCH in SemVer).
- **docs**: Documentation only changes.
- **style**: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc).
- **refactor**: A code change that neither fixes a bug nor adds a feature.
- **perf**: A code change that improves performance.
- **test**: Adding missing tests or correcting existing tests.
- **build**: Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm).
- **ci**: Changes to our CI configuration files and scripts (example scopes: Travis, Circle, BrowserStack, SauceLabs).
- **chore**: Other changes that don't modify src or test files.
- **revert**: Reverts a previous commit.

## Breaking Changes

- Append `!` after the type/scope (e.g., `feat(api)!: change user id to string`) OR include `BREAKING CHANGE:` in the footer.
- This correlates to MAJOR in SemVer.

## Rules

1. **Imperative Mood**: "add feature" not "added feature".
2. **No Period**: Do not end the subject line with a period.
3. **Lowercase**: Keep the subject lowercase (unless proper nouns).
