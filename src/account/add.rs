use crate::app_config;
use crate::hub;
use std::borrow::Cow;
use std::error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io;
use std::io::Write;

pub async fn add() -> Result<(), Error> {
    println!("To add an account you need a Google Client ID and Client Secret.");
    println!(
        "Instructions for how to create credentials can be found here:\
        https://github.com/glotlabs/gdrive/blob/main/docs/create_google_api_credentials.md"
    );
    println!(
        "Note that if you are using gdrive on a remote server you should read this first:\
        https://github.com/glotlabs/gdrive#using-gdrive-on-a-remote-server"
    );
    println!();

    let secret = secret_prompt().map_err(Error::Prompt)?;

    let tmp_dir = tempfile::tempdir().map_err(Error::Tempdir)?;
    let tokens_path = tmp_dir.path().join("tokens.json");

    let auth = hub::Auth::new(&secret, &tokens_path)
        .await
        .map_err(Error::Auth)?;

    // Get access tokens
    auth.token(&[
        "https://www.googleapis.com/auth/drive",
        "https://www.googleapis.com/auth/drive.metadata.readonly",
    ])
    .await
    .map_err(Error::AccessToken)?;

    let hub = hub::Hub::new(auth).map_err(Error::HubCreation)?;
    let (_, about) = hub
        .about()
        .get()
        .param("fields", "user")
        .doit()
        .await
        .map_err(Error::About)?;

    let email = about
        .user
        .and_then(|u| u.email_address)
        .map_or(Cow::Borrowed("unknown"), Into::into);

    let app_cfg =
        app_config::add_account(&email, &secret, &tokens_path).map_err(Error::AddAccount)?;

    println!();
    println!(
        "Saved account credentials in {}",
        app_cfg.base_path.display()
    );
    println!(
        "Keep them safe! If someone gets access to them, they will also be able to access your\
        Google Drive."
    );

    app_config::switch_account(&app_cfg).map_err(Error::SwitchAccount)?;
    println!();
    println!("Logged in as {}", app_cfg.account.name);

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    HubCreation(io::Error),
    Prompt(io::Error),
    Tempdir(io::Error),
    Auth(io::Error),
    AddAccount(app_config::Error),
    SwitchAccount(app_config::Error),
    AccessToken(google_drive3::oauth2::Error),
    About(google_drive3::Error),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::HubCreation(error)
            | Error::Prompt(error)
            | Error::Tempdir(error)
            | Error::Auth(error) => Some(error),
            Error::AddAccount(error) | Error::SwitchAccount(error) => Some(error),
            Error::AccessToken(error) => Some(error),
            Error::About(error) => Some(error),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Error::HubCreation(_) => "unable to create a Google Drive hub",
            Error::Prompt(_) => "failed to get input from user",
            Error::Tempdir(_) => "failed to create temporary directory",
            Error::Auth(_) => "failed to authenticate",
            Error::AddAccount(_) => "unable to add account in the config",
            Error::SwitchAccount(_) => "unable to switch account in the config",
            Error::AccessToken(_) => "failed to get access token",
            Error::About(_) => "failed to get user info",
        };

        f.write_str(s)
    }
}

fn secret_prompt() -> Result<app_config::Secret, io::Error> {
    let client_id = prompt_input("Client ID")?;
    let client_secret = prompt_input("Client secret")?;

    Ok(app_config::Secret {
        client_id,
        client_secret,
    })
}

fn prompt_input(msg: &str) -> Result<String, io::Error> {
    print!("{msg}: ");
    let _ = io::stdout().flush();

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_string())
}
