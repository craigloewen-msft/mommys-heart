//! One-time importer for the sender.net subscriber CSV export.
//!
//! A standalone binary rather than a subcommand of the app: this is migration
//! tooling that runs a handful of times, so nothing about it belongs in the
//! shipped application. It depends on the app as a library and writes through
//! the same repository functions the UI uses, so every validation rule and
//! database CHECK still applies.
//!
//!     cargo run --release -- <file.csv> --actor you@example.org [--dry-run]
//!
//! The database comes from DATABASE_URL. Set it explicitly for anything other
//! than the local dev database — see the README.

mod importer;

#[tokio::main]
async fn main() {
    // Only pick up a .env when the caller has not already chosen a database.
    // Otherwise a stray .env.local in the repo root could silently redirect a
    // production import at the local container.
    let explicit_database = std::env::var("DATABASE_URL").is_ok();
    if !explicit_database {
        let _ = dotenvy::from_filename("../../.env.local");
        let _ = dotenvy::dotenv();
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            // An inherited RUST_LOG (a dev shell, a CI runner) must not be able
            // to hide this tool's own report, so always force our own level in.
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
                .add_directive("sender_net_import=info".parse().expect("static directive"))
                .add_directive(
                    "mommys_heart_app=warn"
                        .parse()
                        .expect("static directive"),
                ),
        )
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        eprintln!(
            "usage: sender-net-import <file.csv> --actor <email> [--dry-run]\n\
             \n\
               --actor <email>  an existing account; the import is recorded as this person\n\
               --dry-run        report what would happen and write nothing\n\
             \n\
             The target database is DATABASE_URL. With none set, the local\n\
             ../../.env.local is used."
        );
        std::process::exit(2);
    }

    let dry_run = args.iter().any(|a| a == "--dry-run");
    let actor_at = args.iter().position(|a| a == "--actor");
    let Some(actor) = actor_at.and_then(|i| args.get(i + 1)).cloned() else {
        fail("--actor <email> is required so the import is attributed to a real account");
    };
    // The path is the first bare argument that is not the value of --actor.
    let Some(path) = args
        .iter()
        .enumerate()
        .find(|(i, a)| !a.starts_with("--") && Some(*i) != actor_at.map(|at| at + 1))
        .map(|(_, a)| a.clone())
    else {
        fail("no CSV file given");
    };

    if !std::path::Path::new(&path).is_file() {
        fail(&format!("{path} is not a readable file"));
    }

    let database_url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => fail("DATABASE_URL is not set, and no .env.local was found"),
    };
    tracing::info!("database  {}", redact(&database_url));
    tracing::info!("actor     {actor}");
    tracing::info!("file      {path}");
    if dry_run {
        tracing::info!("mode      dry run (nothing will be written)");
    } else {
        tracing::warn!("mode      LIVE - this will write to the database above");
    }

    if let Err(error) = mommys_heart_app::server::db::init().await {
        fail(&format!("cannot connect to the database: {error}"));
    }
    if let Err(error) = importer::run(&path, dry_run, &actor).await {
        fail(&error);
    }
}

fn fail(message: &str) -> ! {
    // Straight to stderr, not through tracing: an operator must see why this
    // exited even when RUST_LOG is set to something restrictive.
    eprintln!("sender-net-import: {message}");
    std::process::exit(1);
}

/// A connection string is safe to log only without its password.
fn redact(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    match rest.split_once('@') {
        Some((credentials, host)) => {
            let user = credentials.split(':').next().unwrap_or("");
            format!("{scheme}://{user}:***@{host}")
        }
        None => url.to_string(),
    }
}
