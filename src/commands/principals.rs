use clap::{Parser, Subcommand};
use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use password_hash::PasswordHasher;
use password_hash::SaltString;
use rand::rngs::OsRng;
use rustical_store::auth::{AuthenticationProvider, TomlPrincipalStore, User, user::PrincipalType};

use crate::config::{self, Config};

#[derive(Parser, Debug)]
pub struct PrincipalsArgs {
    #[arg(short, long, env, default_value = "/etc/rustical/config.toml")]
    config_file: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
struct CreateArgs {
    id: String,
    #[arg(value_enum, short, long)]
    principal_type: Option<PrincipalType>,
    #[arg(short, long)]
    name: Option<String>,
    #[arg(long, help = "Ask for password input")]
    password: bool,
}

#[derive(Parser, Debug)]
struct RemoveArgs {
    id: String,
}

#[derive(Debug, Subcommand)]
enum Command {
    List,
    Create(CreateArgs),
    Remove(RemoveArgs),
}

pub async fn cmd_principals(args: PrincipalsArgs) -> anyhow::Result<()> {
    let config: Config = Figment::new()
        // TODO: What to do when config file does not exist?
        .merge(Toml::file(&args.config_file))
        .merge(Env::prefixed("RUSTICAL_").split("__"))
        .extract()?;

    let user_store = match config.auth {
        config::AuthConfig::Toml(config) => TomlPrincipalStore::new(config)?,
    };

    match args.command {
        Command::List => {
            for principal in user_store.get_principals().await? {
                println!(
                    "{} (displayname={}) [{}]",
                    principal.id,
                    principal.displayname.unwrap_or_default(),
                    principal.principal_type
                );
            }
        }
        Command::Create(CreateArgs {
            id,
            principal_type,
            name,
            password,
        }) => {
            let salt = SaltString::generate(OsRng);
            let password = if password {
                println!("Enter your password:");
                let password = rpassword::read_password()?;
                Some(
                    argon2::Argon2::default()
                        .hash_password(password.as_bytes(), &salt)
                        .unwrap()
                        .to_string()
                        .into(),
                )
            } else {
                None
            };
            user_store
                .insert_principal(User {
                    id,
                    displayname: name,
                    principal_type: principal_type.unwrap_or_default(),
                    app_tokens: vec![],
                    password,
                    memberships: vec![],
                })
                .await?;
            println!("Principal created");
        }
        Command::Remove(RemoveArgs { id }) => {
            user_store.remove_principal(&id).await?;
            println!("Principal {id} removed");
        }
    }
    Ok(())
}
