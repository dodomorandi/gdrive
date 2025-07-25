pub mod about;
pub mod account;
pub mod app_config;
pub mod common;
pub mod drives;
pub mod files;
pub mod hub;
pub mod permissions;
pub mod version;

use clap::{Parser, Subcommand};
use common::delegate::ChunkSize;
use common::permission;
use error_trace::ErrorTrace;
use files::list::ListQuery;
use files::list::ListSortOrder;
use mime::Mime;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about, long_about = None, disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print information about gdrive
    About,

    /// Commands for managing accounts
    Account {
        #[command(subcommand)]
        command: AccountCommand,
    },

    /// Commands for managing drives
    Drives {
        #[command(subcommand)]
        command: DriveCommand,
    },

    /// Commands for managing files
    Files {
        #[command(subcommand)]
        command: FileCommand,
    },

    /// Commands for managing file permissions
    Permissions {
        #[command(subcommand)]
        command: PermissionCommand,
    },

    /// Print version information
    Version,
}

#[derive(Subcommand)]
enum AccountCommand {
    /// Add an account
    Add,

    /// List all accounts
    List,

    /// Print current account
    Current,

    /// Switch to a different account
    Switch {
        /// Account name
        account_name: String,
    },

    /// Remove an account
    Remove {
        /// Account name
        account_name: String,
    },

    /// Export account, this will create a zip file of the account which can be imported
    Export {
        /// Account name
        account_name: String,
    },

    /// Import account that was created with the export command
    Import {
        /// Path to archive
        file_path: PathBuf,
    },
}

#[derive(Subcommand)]
enum DriveCommand {
    /// List drives
    List {
        /// Don't print header
        #[arg(long)]
        skip_header: bool,

        /// Field separator
        #[arg(long, default_value_t = String::from("\t"))]
        field_separator: String,
    },
}

#[derive(Subcommand)]
enum FileCommand {
    /// Print file info
    Info {
        /// File id
        file_id: String,

        /// Display size in bytes
        #[arg(long, default_value_t = false)]
        size_in_bytes: bool,
    },

    /// List files
    List {
        /// Max files to list
        #[arg(long, default_value_t = 30)]
        max: usize,

        /// Query. See <https://developers.google.com/drive/search-parameters>
        #[arg(long, default_value_t = ListQuery::default())]
        query: ListQuery,

        /// Order by. See <https://developers.google.com/drive/api/v3/reference/files/list>
        #[arg(long, default_value_t = ListSortOrder::default())]
        order_by: ListSortOrder,

        /// List files in a specific folder
        #[arg(long, value_name = "DIRECTORY_ID")]
        parent: Option<String>,

        /// List files on a shared drive
        #[arg(long, value_name = "DRIVE_ID")]
        drive: Option<String>,

        /// Don't print header
        #[arg(long)]
        skip_header: bool,

        /// Show full file name without truncating
        #[arg(long)]
        full_name: bool,

        /// Field separator
        #[arg(long, default_value_t = String::from("\t"))]
        field_separator: String,
    },

    /// Download file
    Download {
        /// File id
        file_id: String,

        /// Overwrite existing files and folders
        #[arg(long)]
        overwrite: bool,

        /// Follow shortcut and download target file (does not work with recursive download)
        #[arg(long)]
        follow_shortcuts: bool,

        /// Download directories
        #[arg(long)]
        recursive: bool,

        /// Path where the file/directory should be downloaded to
        #[arg(long, value_name = "PATH")]
        destination: Option<PathBuf>,

        /// Write file to stdout
        #[arg(long)]
        stdout: bool,
    },

    /// Upload file
    Upload {
        /// Path of file to upload
        file_path: Option<PathBuf>,

        /// Force mime type [default: auto-detect]
        #[arg(long, value_name = "MIME_TYPE")]
        mime: Option<Mime>,

        /// Upload to an existing directory
        #[arg(long, value_name = "DIRECTORY_ID")]
        parent: Option<Vec<String>>,

        /// Upload directories. Note that this will always create a new directory on drive and will not update existing directories with the same name
        #[arg(long)]
        recursive: bool,

        /// Set chunk size in MB, must be a power of two.
        #[arg(long, value_name = "1|2|4|8|16|32|64|128|256|512|1024|4096|8192", default_value_t = ChunkSize::default())]
        chunk_size: ChunkSize,

        /// Print errors occuring during chunk upload
        #[arg(long, value_name = "", default_value_t = false)]
        print_chunk_errors: bool,

        /// Print details about each chunk
        #[arg(long, value_name = "", default_value_t = false)]
        print_chunk_info: bool,

        /// Print only id of file/folder
        #[arg(long, default_value_t = false)]
        print_only_id: bool,
    },

    /// Update file. This will create a new version of the file. The older versions will typically be kept for 30 days.
    Update {
        /// File id of the file you want ot update
        file_id: String,

        /// Path of file to upload
        file_path: Option<PathBuf>,

        /// Force mime type [default: auto-detect]
        #[arg(long, value_name = "MIME_TYPE")]
        mime: Option<Mime>,

        /// Set chunk size in MB, must be a power of two.
        #[arg(long, value_name = "1|2|4|8|16|32|64|128|256|512|1024|4096|8192", default_value_t = ChunkSize::default())]
        chunk_size: ChunkSize,

        /// Print errors occuring during chunk upload
        #[arg(long, value_name = "", default_value_t = false)]
        print_chunk_errors: bool,

        /// Print details about each chunk
        #[arg(long, value_name = "", default_value_t = false)]
        print_chunk_info: bool,
    },

    /// Delete file
    Delete {
        /// File id
        file_id: String,

        /// Delete directory and all it's content
        #[arg(long)]
        recursive: bool,
    },

    /// Create directory
    Mkdir {
        /// Name
        name: String,

        /// Create in an existing directory
        #[arg(long, value_name = "DIRECTORY_ID")]
        parent: Option<Vec<String>>,

        /// Print only id of folder
        #[arg(long, default_value_t = false)]
        print_only_id: bool,
    },

    /// Rename file/directory
    Rename {
        /// Id of file or directory
        file_id: String,

        /// New name
        name: String,
    },

    /// Move file/directory
    Move {
        /// Id of file or directory to move
        file_id: String,

        /// Id of folder to move to
        folder_id: String,
    },

    /// Copy file
    Copy {
        /// Id of file or directory to move
        file_id: String,

        /// Id of folder to copy to
        folder_id: String,
    },

    /// Import file as a google document/spreadsheet/presentation.
    /// Example of file types that can be imported: doc, docx, odt, pdf, html, xls, xlsx, csv, ods, ppt, pptx, odp
    Import {
        /// Path to file
        file_path: PathBuf,

        /// Upload to an existing directory
        #[arg(long, value_name = "DIRECTORY_ID")]
        parent: Option<Vec<String>>,

        /// Print only id of file
        #[arg(long, default_value_t = false)]
        print_only_id: bool,
    },

    /// Export google document to file
    Export {
        /// File id
        file_id: String,

        /// File path to export to. The file extension will determine the export format
        file_path: PathBuf,

        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,
    },
}

#[derive(Subcommand)]
enum PermissionCommand {
    /// Grant permission to file
    Share {
        /// File id
        file_id: String,

        /// The role granted by this permission. Allowed values are: owner, organizer, fileOrganizer, writer, commenter, reader
        #[arg(long, default_value_t = permission::Role::default())]
        role: permission::Role,

        /// The type of the grantee. Valid values are: user, group, domain, anyone
        #[arg(long, default_value_t = permission::Type::default())]
        type_: permission::Type,

        /// Email address. Required for user and group type
        #[arg(long)]
        email: Option<String>,

        /// Domain. Required for domain type
        #[arg(long)]
        domain: Option<String>,

        /// Whether the permission allows the file to be discovered through search. This is only applicable for permissions of type domain or anyone
        #[arg(long)]
        discoverable: bool,
    },

    /// List permissions for a file
    List {
        /// File id
        file_id: String,

        /// Don't print header
        #[arg(long)]
        skip_header: bool,

        /// Field separator
        #[arg(long, default_value_t = String::from("\t"))]
        field_separator: String,
    },

    /// Revoke permissions for a file. If no other options are specified, the 'anyone' permission will be revoked
    Revoke {
        /// File id
        file_id: String,

        /// Revoke all permissions (except owner)
        #[arg(long)]
        all: bool,

        /// Revoke specific permission
        #[arg(long, value_name = "PERMISSION_ID")]
        id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = run().await {
        eprintln!("{}", err.trace());
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let cli = Cli::parse();

    match cli.command {
        Command::About => {
            about::about();
        }

        Command::Account { command } => {
            handle_account_command(command).await?;
        }

        Command::Drives { command } => match command {
            DriveCommand::List {
                skip_header,
                field_separator,
            } => {
                drives::list(drives::list::Config {
                    skip_header,
                    field_separator,
                })
                .await?;
            }
        },

        Command::Files { command } => {
            handle_files_command(command).await?;
        }

        Command::Permissions { command } => {
            handle_permissions_command(command).await?;
        }

        Command::Version => {
            version::version();
        }
    }

    Ok(())
}

async fn handle_permissions_command(
    command: PermissionCommand,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    match command {
        PermissionCommand::Share {
            file_id,
            role,
            type_,
            discoverable,
            email,
            domain,
        } => {
            permissions::share(permissions::share::Config {
                file_id,
                role,
                type_,
                discoverable,
                email,
                domain,
            })
            .await?;
        }

        PermissionCommand::List {
            file_id,
            skip_header,
            field_separator,
        } => {
            permissions::list(permissions::list::Config {
                file_id,
                skip_header,
                field_separator,
            })
            .await?;
        }

        PermissionCommand::Revoke { file_id, all, id } => {
            let action = if all {
                permissions::revoke::RevokeAction::AllExceptOwner
            } else if id.is_some() {
                permissions::revoke::RevokeAction::Id(id.unwrap_or_default())
            } else {
                permissions::revoke::RevokeAction::Anyone
            };

            permissions::revoke(permissions::revoke::Config { file_id, action }).await?;
        }
    }

    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "FileCommand has many variants, pretty big match statement"
)]
async fn handle_files_command(
    command: FileCommand,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    match command {
        FileCommand::Info {
            file_id,
            size_in_bytes,
        } => {
            files::info(files::info::Config {
                file_id,
                size_in_bytes,
            })
            .await?;
        }

        FileCommand::List {
            max,
            query,
            order_by,
            parent,
            drive,
            skip_header,
            full_name,
            field_separator,
        } => {
            let parent_query = parent.map(|folder_id| ListQuery::FilesInFolder { folder_id });
            let drive_query = drive.map(|drive_id| ListQuery::FilesOnDrive { drive_id });
            let q = parent_query.or(drive_query).unwrap_or(query);

            files::list(files::list::Config {
                query: q,
                order_by,
                max_files: max,
                skip_header,
                truncate_name: !full_name,
                field_separator,
            })
            .await?;
        }

        FileCommand::Download {
            file_id,
            overwrite,
            follow_shortcuts,
            recursive,
            destination,
            stdout,
        } => {
            let existing_file_action = if overwrite {
                files::download::ExistingFileAction::Overwrite
            } else {
                files::download::ExistingFileAction::Abort
            };

            let dst = if stdout {
                files::download::Destination::Stdout
            } else if let Some(path) = destination {
                files::download::Destination::Path(path)
            } else {
                files::download::Destination::CurrentDir
            };

            files::download(files::download::Config {
                file_id,
                existing_file_action,
                follow_shortcuts,
                download_directories: recursive,
                destination: dst,
            })
            .await?;
        }

        FileCommand::Upload {
            file_path,
            mime,
            parent,
            recursive,
            chunk_size,
            print_chunk_errors,
            print_chunk_info,
            print_only_id,
        } => {
            files::upload(files::upload::Config {
                file_path,
                mime_type: mime,
                parents: parent,
                chunk_size,
                print_chunk_errors,
                print_chunk_info,
                upload_directories: recursive,
                print_only_id,
            })
            .await?;
        }

        FileCommand::Update {
            file_id,
            file_path,
            mime,
            chunk_size,
            print_chunk_errors,
            print_chunk_info,
        } => {
            files::update(files::update::Config {
                file_id,
                file_path,
                mime_type: mime,
                chunk_size,
                print_chunk_errors,
                print_chunk_info,
            })
            .await?;
        }

        FileCommand::Delete { file_id, recursive } => {
            files::delete(files::delete::Config {
                file_id,
                delete_directories: recursive,
            })
            .await?;
        }

        FileCommand::Mkdir {
            name,
            parent,
            print_only_id,
        } => {
            files::mkdir(files::mkdir::Config {
                id: None,
                name,
                parents: parent,
                print_only_id,
            })
            .await?;
        }

        FileCommand::Rename { file_id, name } => {
            files::rename(files::rename::Config { file_id, name }).await?;
        }

        FileCommand::Move { file_id, folder_id } => {
            files::mv(files::mv::Config {
                file_id,
                to_folder_id: folder_id,
            })
            .await?;
        }

        FileCommand::Copy { file_id, folder_id } => {
            files::copy(files::copy::Config {
                file_id,
                to_folder_id: folder_id,
            })
            .await?;
        }

        FileCommand::Import {
            file_path,
            parent,
            print_only_id,
        } => {
            files::import(files::import::Config {
                file_path,
                parents: parent,
                print_only_id,
            })
            .await?;
        }

        FileCommand::Export {
            file_id,
            file_path,
            overwrite,
        } => {
            let existing_file_action = if overwrite {
                files::export::ExistingFileAction::Overwrite
            } else {
                files::export::ExistingFileAction::Abort
            };

            files::export(files::export::Config {
                file_id,
                file_path,
                existing_file_action,
            })
            .await?;
        }
    }

    Ok(())
}

async fn handle_account_command(
    command: AccountCommand,
) -> Result<(), Box<dyn std::error::Error + 'static>> {
    match command {
        AccountCommand::Add => {
            account::add().await?;
        }

        AccountCommand::List => {
            account::list()?;
        }

        AccountCommand::Current => {
            account::current()?;
        }

        AccountCommand::Switch { account_name } => {
            account::switch(&account::switch::Config { account_name })?;
        }

        AccountCommand::Remove { account_name } => {
            account::remove(&account::remove::Config { account_name })?;
        }

        AccountCommand::Export { account_name } => {
            account::export(&account::export::Config { account_name })?;
        }

        AccountCommand::Import { file_path } => {
            account::import(&account::import::Config {
                archive_path: file_path,
            })?;
        }
    }

    Ok(())
}
