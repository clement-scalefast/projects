# Projects

Projects is a project management app in the style of Jira or GitHub Projects. Teams organize work into projects, track it as issues on a kanban board, and discuss it in comments. The app uses [Topcoat](https://github.com/tokio-rs/topcoat) with Toasty and PostgreSQL, [Keycloak](https://www.keycloak.org) for sign-in, and Topcoat UI components. It is available in English and German.

[![CI](https://github.com/clement-scalefast/projects/actions/workflows/ci.yml/badge.svg)](https://github.com/clement-scalefast/projects/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

The quickest way to run the app is Docker Compose, which builds the app and starts it with a PostgreSQL database and a Keycloak server:

```sh
git clone https://github.com/clement-scalefast/projects.git
cd projects
docker compose up --build
```

The app runs at `http://localhost:3000`. On startup, it applies database migrations and adds sample data if the database is empty. The sign-in page shows the demo accounts and password. Run `docker compose down --volumes` to delete the database and the Keycloak data and start over.

To work on the app, start only the database and Keycloak, and run the app on your machine with the Topcoat command-line tool:

```sh
docker compose up -d db keycloak
cargo install topcoat-cli
topcoat dev
```

The app connects to the database named by `DATABASE_URL`. Without it, the app uses `postgresql://projects:projects@127.0.0.1:5432/projects`, which is the Compose database. Set `POSTGRES_PORT` to publish the database on another port, and `POSTGRES_PASSWORD` to change its password before the first start.

## Signing in

Keycloak signs users in with OpenID Connect. The app has no passwords of its own: the sign-in page sends the browser to Keycloak, and Keycloak sends it back with the user's name, email, and roles. The app then keeps its own session in a cookie. Signing out ends both the app's session and the Keycloak session.

Compose imports the realm in [`keycloak/realm.json`](keycloak/realm.json) the first time Keycloak starts. It contains the `projects` client, the demo accounts, and three realm roles. Manage users in the Keycloak admin console at `http://localhost:8080` (user `admin`, password `admin`):

- A user needs an email address to sign in to the app.
- The realm role `admin`, `staff`, or `viewer` sets what the user may do. A user with none of them can only read.
- A user appears on the team page after signing in for the first time. Changes to the name, email, or roles apply at the next sign-in.

To use another Keycloak server, create a confidential OpenID Connect client that allows the redirect URI `<app URL>/login/callback` and the post-logout redirect URI `<app URL>/login`. Add a "User Realm Role" mapper that puts the roles in a multivalued `roles` claim of the ID token. Then set these variables:

| Variable | Default | Purpose |
| --- | --- | --- |
| `KEYCLOAK_URL` | `http://localhost:8080` | Keycloak's URL as browsers reach it. |
| `KEYCLOAK_INTERNAL_URL` | the value of `KEYCLOAK_URL` | Keycloak's URL as the app reaches it, when that differs. |
| `KEYCLOAK_REALM` | `projects` | The realm. |
| `KEYCLOAK_CLIENT_ID` | `projects` | The client ID. |
| `KEYCLOAK_CLIENT_SECRET` | `projects-dev-secret` | The client secret. |
| `APP_URL` | `http://` and the request's host | The app's public URL. Set it when the app is served over HTTPS or behind a proxy. |

The Compose setup runs Keycloak in development mode with a published client secret and admin password. Do not expose it to others as it is.

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
| [`src/app/login.rs`](src/app/login.rs) | Sign-in page, the redirect to Keycloak, and the return from it. |
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

## Contributing

Bug reports, ideas, and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) to get started and the [code of conduct](CODE_OF_CONDUCT.md) before taking part. Report security problems privately as described in [SECURITY.md](SECURITY.md).

## License

Projects is released under the [MIT license](LICENSE). It is derived from the admin demo of [Topcoat](https://github.com/tokio-rs/topcoat), which is also MIT-licensed.
