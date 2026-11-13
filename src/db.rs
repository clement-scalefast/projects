use toasty::{Db, migration::MigrationSet};
use toasty_cli::{Config, MigrationConfig, ToastyCli};
use topcoat::context::{Cx, context};

/// Names the environment variable that holds the PostgreSQL connection URL.
const DATABASE_URL_ENV: &str = "DATABASE_URL";

/// Matches the database that `docker compose up db` starts.
const DEFAULT_DATABASE_URL: &str = "postgresql://projects:projects@127.0.0.1:5432/projects";

const MIGRATIONS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/toasty");

const MIGRATIONS: MigrationSet = toasty::embed_migrations!();

pub async fn connect() -> toasty::Result<Db> {
    let url = std::env::var(DATABASE_URL_ENV).unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    Db::builder()
        .models(toasty::models!(crate::*))
        .connect(&url)
        .await
}

pub async fn migrate(db: &Db) -> toasty::Result<()> {
    MIGRATIONS.apply(db).await?;
    Ok(())
}

pub fn cli(db: Db) -> ToastyCli {
    let config = Config::new().migration(MigrationConfig::new().path(MIGRATIONS_DIR));
    ToastyCli::with_config(db, config)
}

pub fn db(cx: &Cx) -> Db {
    context::<Db>(cx).clone()
}

/// Builds a SQL LIKE pattern for a literal substring search.
/// Use with `ilike_with_escape` and a backslash escape character, so the search
/// ignores case.
pub fn contains_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    pattern.push('%');
    for char in text.chars() {
        if matches!(char, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(char);
    }
    pattern.push('%');
    pattern
}
