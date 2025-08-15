pub mod errors;

use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::io;
use std::ops::Not;
use std::path::Path;
use std::path::PathBuf;

const SYSTEM_CONFIG_DIR_NAME: &str = ".config";
const BASE_PATH_DIR_NAME: &str = "gdrive3";
const ACCOUNT_CONFIG_NAME: &str = "account.json";
const SECRET_CONFIG_NAME: &str = "secret.json";
const TOKENS_CONFIG_NAME: &str = "tokens.json";

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub base_path: PathBuf,
    pub account: Account,
}

pub fn add_account(
    account_name: &str,
    secret: &Secret,
    tokens_path: &Path,
) -> Result<AppConfig, errors::AddAccount> {
    let config = AppConfig::init_account(account_name).map_err(errors::AddAccount::InitAccount)?;
    config
        .save_secret(secret)
        .map_err(errors::AddAccount::SaveSecret)?;
    fs::copy(tokens_path, config.tokens_path()).map_err(errors::AddAccount::CopyTokens)?;
    Ok(config)
}

pub fn switch_account(config: &AppConfig) -> Result<(), errors::SaveAccountConfig> {
    config.save_account_config()
}

pub fn list_accounts() -> Result<Vec<String>, errors::ListAccounts> {
    let base_path =
        AppConfig::default_base_path().map_err(errors::ListAccounts::DefaultBasePath)?;
    if let Err(source) = fs::create_dir_all(&base_path) {
        return Err(errors::ListAccounts::CreateBaseDir {
            path: base_path,
            source,
        });
    }
    let entries = match fs::read_dir(&base_path) {
        Ok(entries) => entries,
        Err(source) => {
            return Err(errors::ListAccounts::ListFiles {
                path: base_path,
                source,
            })
        }
    };

    let mut accounts: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join(TOKENS_CONFIG_NAME).exists())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    accounts.sort();

    Ok(accounts)
}

impl AppConfig {
    #[must_use]
    fn new(base_path: PathBuf, account: Account) -> Self {
        AppConfig { base_path, account }
    }

    #[must_use]
    pub fn has_current_account() -> bool {
        AppConfig::default_base_path().is_ok_and(|base_path| {
            let account_config_path = base_path.join(ACCOUNT_CONFIG_NAME);
            account_config_path.exists()
        })
    }

    pub fn load_current_account() -> Result<AppConfig, errors::LoadCurrentAccount> {
        let base_path =
            AppConfig::default_base_path().map_err(errors::LoadCurrentAccount::DefaultBasePath)?;
        let account_config = AppConfig::load_account_config()
            .map_err(errors::LoadCurrentAccount::LoadAccountConfig)?;
        let account = Account::new(&account_config.current);
        Ok(AppConfig::new(base_path, account))
    }

    pub fn load_account(account_name: &str) -> Result<AppConfig, errors::LoadAccount> {
        let base_path = AppConfig::default_base_path().map_err(errors::LoadAccount)?;
        let account = Account::new(account_name);
        Ok(AppConfig::new(base_path, account))
    }

    pub fn init_account(account_name: &str) -> Result<AppConfig, errors::InitAccount> {
        let base_path = AppConfig::default_base_path()?;
        let account = Account::new(account_name);

        let config = AppConfig::new(base_path, account);
        config.create_account_dir()?;

        Ok(config)
    }

    pub fn remove_account(&self) -> Result<(), errors::RemoveAccount> {
        let path = self.account_base_path();
        if let Err(source) = fs::remove_dir_all(&path) {
            return Err(errors::RemoveAccount::RemoveDirectory { path, source });
        }

        let account_config =
            AppConfig::load_account_config().map_err(errors::RemoveAccount::LoadConfig)?;
        if self.account.name == account_config.current {
            let config_path = self.account_config_path();
            if let Err(source) = fs::remove_file(&config_path) {
                return Err(errors::RemoveAccount::RemoveConfig {
                    path: config_path,
                    source,
                });
            }
        }

        Ok(())
    }

    pub fn save_secret(&self, secret: &Secret) -> Result<(), errors::SaveSecret> {
        let content =
            serde_json::to_string_pretty(&secret).map_err(errors::SaveSecret::Serialize)?;
        let path = self.secret_path();
        if let Err(source) = fs::write(&path, content) {
            return Err(errors::SaveSecret::Write { path, source });
        }

        if let Err(err) = set_file_permissions(&path) {
            eprintln!("Warning: Failed to set file permissions on secrets file: {err}");
        }

        Ok(())
    }

    pub fn load_secret(&self) -> Result<Secret, errors::LoadSecret> {
        let path = self.secret_path();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(source) => return Err(errors::LoadSecret::Read { path, source }),
        };
        match serde_json::from_str(&content) {
            Ok(secret) => Ok(secret),
            Err(source) => Err(errors::LoadSecret::Deserialize { content, source }),
        }
    }

    pub fn load_account_config() -> Result<AccountConfig, errors::LoadAccountConfig> {
        let base_path =
            AppConfig::default_base_path().map_err(errors::LoadAccountConfig::DefaultBasePath)?;
        let account_config_path = base_path.join(ACCOUNT_CONFIG_NAME);
        if account_config_path.exists().not() {
            return Err(errors::LoadAccountConfig::AccountConfigMissing);
        }
        let content = match fs::read_to_string(&account_config_path) {
            Ok(content) => content,
            Err(source) => {
                return Err(errors::LoadAccountConfig::ReadAccountConfig {
                    source,
                    path: account_config_path,
                })
            }
        };
        match serde_json::from_str(&content) {
            Ok(config) => Ok(config),
            Err(source) => Err(errors::LoadAccountConfig::Deserialize { content, source }),
        }
    }

    pub fn save_account_config(&self) -> Result<(), errors::SaveAccountConfig> {
        let account_config = AccountConfig {
            current: self.account.name.clone(),
        };

        let content = serde_json::to_string_pretty(&account_config)
            .map_err(errors::SaveAccountConfig::Serialize)?;
        let account_config_path = self.account_config_path();
        match fs::write(&account_config_path, content) {
            Ok(()) => Ok(()),
            Err(source) => Err(errors::SaveAccountConfig::Write {
                path: account_config_path,
                source,
            }),
        }
    }

    #[must_use]
    pub fn account_config_path(&self) -> PathBuf {
        self.base_path.join(ACCOUNT_CONFIG_NAME)
    }

    #[must_use]
    pub fn account_base_path(&self) -> PathBuf {
        self.base_path.join(&self.account.name)
    }

    #[must_use]
    pub fn secret_path(&self) -> PathBuf {
        self.account_base_path().join(SECRET_CONFIG_NAME)
    }

    #[must_use]
    pub fn tokens_path(&self) -> PathBuf {
        self.account_base_path().join(TOKENS_CONFIG_NAME)
    }

    pub fn default_base_path() -> Result<PathBuf, errors::DefaultBasePath> {
        let home_path = home::home_dir().ok_or(errors::DefaultBasePath)?;
        let base_path = home_path
            .join(SYSTEM_CONFIG_DIR_NAME)
            .join(BASE_PATH_DIR_NAME);
        Ok(base_path)
    }

    fn create_account_dir(&self) -> Result<(), errors::CreateAccountDir> {
        let path = self.account_base_path();
        fs::create_dir_all(&path).map_err(errors::CreateAccountDir)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountConfig {
    pub current: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Account {
    pub name: String,
}

impl Account {
    #[must_use]
    pub fn new(name: &str) -> Account {
        Account {
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub client_id: String,
    pub client_secret: String,
}

pub fn set_file_permissions(path: &Path) -> Result<(), io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}
