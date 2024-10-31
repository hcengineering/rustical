use crate::config::Config;
use actix_web::HttpServer;
use anyhow::Result;
use app::make_app;
use clap::Parser;
use config::{DataStoreConfig, SqliteDataStoreConfig};
use rustical_store::auth::StaticUserStore;
use rustical_store::{AddressbookStore, CalendarStore};
use rustical_store_sqlite::{create_db_pool, SqliteStore};
use setup_tracing::setup_tracing;
use std::fs;
use std::sync::Arc;

mod app;
mod config;
mod setup_tracing;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env)]
    config_file: String,
    #[arg(long, env, help = "Run database migrations (only for sql store)")]
    migrate: bool,
}

async fn get_data_stores(
    migrate: bool,
    config: &DataStoreConfig,
) -> Result<(Arc<dyn AddressbookStore>, Arc<dyn CalendarStore>)> {
    Ok(match &config {
        DataStoreConfig::Sqlite(SqliteDataStoreConfig { db_url }) => {
            let db = create_db_pool(db_url, migrate).await?;
            let sqlite_store = Arc::new(SqliteStore::new(db));
            (sqlite_store.clone(), sqlite_store.clone())
        }
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config: Config = toml::from_str(&fs::read_to_string(&args.config_file)?)?;

    setup_tracing(&config.tracing);

    let (addr_store, cal_store) = get_data_stores(args.migrate, &config.data_store).await?;

    let user_store = Arc::new(match config.auth {
        config::AuthConfig::Static(config) => StaticUserStore::new(config),
    });

    HttpServer::new(move || make_app(addr_store.clone(), cal_store.clone(), user_store.clone()))
        .bind((config.http.host, config.http.port))?
        .run()
        .await?;

    Ok(())
}
