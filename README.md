# Trackline

Trackline is a project management app in the style of Jira or GitHub Projects. Teams organize work into projects, track it as issues on a kanban board, and discuss it in comments. The app uses [Topcoat](https://github.com/tokio-rs/topcoat) with Toasty and SQLite, session cookies for sign-in, and Topcoat UI components. It is available in English and German.

Install the Topcoat command-line tool and start the app:

```sh
cargo install topcoat-cli
topcoat dev
```

The app runs at `http://127.0.0.1:3000`. Set `PORT` to use another port. On startup, it applies database migrations and adds sample data if the database is empty. The database is stored in `data.db`. The sign-in page shows the demo accounts and password. Stop the app and delete the database to reset it.

## What it does

- **Dashboard**: open, created, completed, and overdue issue counts with two weeks of history, the issues assigned to you, and progress per project.
- **Projects**: each project has a key such as `WEB`, a lead, and a board. Drag a card to another column to change its status, or use the arrow buttons on the card.
- **Issues**: a searchable list across all projects with filters for project, status, and priority. Each issue has a type, priority, assignee, due date, description, and comments. Its key, such as `WEB-12`, counts up within the project.
- **Team**: every member with their open and done issues.

There are three roles. Admins create projects and change their settings. Members create, edit, move, and delete issues and write comments. Viewers can read everything and change nothing.

## Where to look

| Path | Contents |
| --- | --- |
| [`src/app.rs`](src/app.rs) | Router setup, the shared HTML layout, and the layer that sets each request's language. |
| [`src/app/_common.rs`](src/app/_common.rs) | Timestamps, role names, and other text shared by several pages. |
| [`src/app/language.rs`](src/app/language.rs) | Language menu and the route that saves the choice. |
| [`src/app/login.rs`](src/app/login.rs) | Sign-in form and request handler. |
| [`src/app/_workspace.rs`](src/app/_workspace.rs) | Sign-in check, sidebar, and breadcrumbs. |
| [`src/app/_workspace/_dashboard.rs`](src/app/_workspace/_dashboard.rs) | Workspace figures, your issues, and project progress. |
| [`src/app/_workspace/projects.rs`](src/app/_workspace/projects.rs) | Project list and the project form fields. |
| [`src/app/_workspace/projects/project_id.rs`](src/app/_workspace/projects/project_id.rs) | The board, moving cards, and project settings. |
| [`src/app/_workspace/issues.rs`](src/app/_workspace/issues.rs) | Issue search, filters, and pagination. Also the badges, icons, and form fields that other pages reuse. |
| [`src/app/_workspace/issues/new.rs`](src/app/_workspace/issues/new.rs) | Creating an issue and form errors. |
| [`src/app/_workspace/issues/issue_id.rs`](src/app/_workspace/issues/issue_id.rs) | Issue details, status changes, comments, editing, and deleting. |
| [`src/app/_workspace/team.rs`](src/app/_workspace/team.rs) | Team list and each member's assigned issues. |
| [`src/features/`](src/features) | Database models, queries, validation, dashboard figures, and the theme and language preferences. |
| [`src/seed.rs`](src/seed.rs) | Sample accounts, projects, issues, and comments. |
| [`src/components/`](src/components) | Copies of registry components, plus custom charts and pagination. |
| [`toasty/`](toasty) | Database migrations and schema snapshots. |

## Translations

Each page module and the custom components keep their [Fluent](https://projectfluent.org/) files in an `l10n/` directory next to them, such as `src/app/_workspace/issues/l10n/`. To add a language, add a file next to each existing one and add its locale to `LOCALES` in [`src/features/locale.rs`](src/features/locale.rs). Run `topcoat l10n check` to find missing messages.

## Changing a model

When you change a model, generate a migration:

```sh
cargo run -- toasty migration generate --name <name>
```

Rebuild and restart the app to apply the new migration. Migrations are included in the compiled app.

## Updating components

Run `topcoat ui add <name> --overwrite` to update a component from the registry. `cargo test` checks that the installed components and theme match the registry. Custom components are excluded from this check through `OWN_COMPONENTS` in [`tests/registry_sync.rs`](tests/registry_sync.rs).

## Formatting

Run `cargo fmt` and then `topcoat fmt`, which formats the bodies of macros such as `view!`.
