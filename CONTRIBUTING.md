# Contributing

Thanks for helping improve Projects. Bug reports, ideas, translations, and pull requests are all welcome.

## Ways to help

- **Report a bug** with the [bug report form](https://github.com/clement-scalefast/projects/issues/new?template=bug_report.yml). Say what you did, what you expected, and what happened.
- **Suggest a feature** with the [feature request form](https://github.com/clement-scalefast/projects/issues/new?template=feature_request.yml). For larger changes, open an issue before writing code so we can agree on the approach.
- **Report a security problem** privately. See [SECURITY.md](SECURITY.md).

## Setting up

You need a stable Rust toolchain and the Topcoat command-line tool:

```sh
git clone https://github.com/clement-scalefast/projects.git
cd projects
cargo install topcoat-cli
topcoat dev
```

The app runs at `http://127.0.0.1:3000` and reloads when you change a file. The sign-in page lists the demo accounts. Delete `data.db` to reset the sample data. The [README](README.md) explains where each part of the app lives.

## Before you open a pull request

Run the same checks as CI:

```sh
cargo fmt
topcoat fmt
cargo clippy --all-targets -- -D warnings
cargo test
topcoat l10n check
```

Also check these, where they apply:

- **Translations**: the app is available in English and German. When you add or change text, update both `en.ftl` and `de.ftl` in the module's `l10n/` directory. If you do not speak German, say so in the pull request and someone will help.
- **Database changes**: when you change a model, generate a migration with `cargo run -- toasty migration generate --name <name>` and commit the files it creates in `toasty/`.
- **Permissions**: pages hide controls a role cannot use, but every route, shard, and procedure must check the role itself.
- **Registry components**: files in `src/components/` are copies from the Topcoat UI registry, except those listed in `OWN_COMPONENTS` in `tests/registry_sync.rs`. Do not edit the copies; a test compares them with the registry.

## Pull requests

- Branch from `main` and keep each pull request to one change.
- Describe what changed and why. Add a screenshot for visible changes.
- Add or update tests for logic in `src/features/`.
- A maintainer reviews every pull request. CI must pass before it is merged.

By contributing, you agree that your work is released under the project's [MIT license](LICENSE).

## Conduct

Everyone taking part in this project follows the [code of conduct](CODE_OF_CONDUCT.md).
