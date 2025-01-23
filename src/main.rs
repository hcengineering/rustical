use crate::config::Config;
use actix_web::http::KeepAlive;
use actix_web::HttpServer;
use anyhow::Result;
use app::make_app;
use clap::{Parser, Subcommand};
use commands::{cmd_gen_config, cmd_pwhash};
use config::{DataStoreConfig, SqliteDataStoreConfig};
use rustical_dav::push::push_notifier;
use rustical_store::auth::StaticUserStore;
use rustical_store::{AddressbookStore, CalendarStore, CollectionOperation, SubscriptionStore};
// use rustical_store_sqlite::addressbook_store::SqliteAddressbookStore;
// use rustical_store_sqlite::calendar_store::SqliteCalendarStore;
// use rustical_store_sqlite::{create_db_pool, SqliteStore};
use setup_tracing::setup_tracing;
use std::fs;
use std::sync::Arc;
use tokio::sync::mpsc::Receiver;

mod app;
mod commands;
mod config;
mod setup_tracing;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, env, default_value = "/etc/rustical/config.toml")]
    config_file: String,
    #[arg(long, env, help = "Do no run database migrations (only for sql store)")]
    no_migrations: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    GenConfig(commands::GenConfigArgs),
    Pwhash(commands::PwhashArgs),
}

async fn get_data_stores(
    migrate: bool,
    config: &DataStoreConfig,
) -> Result<(
    Arc<impl AddressbookStore>,
    Arc<impl CalendarStore>,
    Arc<impl SubscriptionStore>,
    Receiver<CollectionOperation>,
)> {
    let (_, recv) = tokio::sync::mpsc::channel(1000);
    let store = Arc::new(rustical_store_huly::HulyStore::new());
    Ok((store.clone(), store.clone(), store.clone(), recv))
/*
    Ok(match &config {
        DataStoreConfig::Sqlite(SqliteDataStoreConfig { db_url }) => {
            let db = create_db_pool(db_url, migrate).await?;
            // Channel to watch for changes (for DAV Push)
            let (send, recv) = tokio::sync::mpsc::channel(1000);

            let addressbook_store = Arc::new(SqliteAddressbookStore::new(db.clone(), send.clone()));
            let cal_store = Arc::new(SqliteCalendarStore::new(db.clone(), send));
            let subscription_store = Arc::new(SqliteStore::new(db.clone()));
            (addressbook_store, cal_store, subscription_store, recv)
        }
    })
*/
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Some(Command::GenConfig(gen_config_args)) => cmd_gen_config(gen_config_args)?,
        Some(Command::Pwhash(pwhash_args)) => cmd_pwhash(pwhash_args)?,
        None => {
            let config: Config = toml::from_str(
                &fs::read_to_string(&args.config_file).unwrap_or_else(|err| {
                    panic!("Could not open file at {}: {}", &args.config_file, err)
                }),
            )?;

            setup_tracing(&config.tracing);

            let (addr_store, cal_store, subscription_store, update_recv) =
                get_data_stores(!args.no_migrations, &config.data_store).await?;

            if config.dav_push.enabled {
                tokio::spawn(push_notifier(
                    config.dav_push.allowed_push_servers,
                    update_recv,
                    subscription_store.clone(),
                ));
            }

            let user_store = Arc::new(match config.auth {
                config::AuthConfig::Static(config) => StaticUserStore::new(config),
            });

            HttpServer::new(move || {
                make_app(
                    addr_store.clone(),
                    cal_store.clone(),
                    subscription_store.clone(),
                    user_store.clone(),
                    config.frontend.clone(),
                )
            })
            .bind((config.http.host, config.http.port))?
            // Workaround for a weird bug where
            // new requests might timeout since they cannot properly reuse the connection
            // https://github.com/lennart-k/rustical/issues/10
            .keep_alive(KeepAlive::Disabled)
            .run()
            .await?;
        }
    }
    Ok(())
}
