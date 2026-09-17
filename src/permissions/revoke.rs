use std::{
    error,
    fmt::{Display, Formatter},
};

use crate::{
    common::{
        delegate::{UploadDelegate, UploadDelegateConfig},
        hub_helper::{get_hub, GetHubError},
        permission,
    },
    files,
    hub::Hub,
    permissions,
};

#[derive(Clone, Debug)]
pub(crate) struct Config {
    pub(crate) file_id: String,
    pub(crate) action: RevokeAction,
}

pub(crate) async fn revoke(config: Config) -> Result<(), Error> {
    let hub = get_hub().await.map_err(Error::Hub)?;
    let delegate_config = UploadDelegateConfig::default();

    let file = files::info::get_file(&hub, &config.file_id)
        .await
        .map_err(|err| Error::GetFile(Box::new(err)))?;

    let permissions = permissions::list::list_permissions(&hub, &delegate_config, &config.file_id)
        .await
        .map_err(Error::ListPermissions)?;

    for permission in config.action.get_matching_permissions(permissions) {
        let permission = permission?;
        if print_revoke_details(&file, &permission).is_err() {
            println!(
                "Revoking permission with id: '{}'",
                permission.id.as_deref().unwrap_or_default()
            );
        }

        delete_permission(
            &hub,
            &delegate_config,
            &config.file_id,
            permission.id.as_deref().unwrap_or_default(),
        )
        .await
        .map_err(|err| Error::DeletePermission(Box::new(permission), err))?;
    }

    Ok(())
}

async fn delete_permission(
    hub: &Hub,
    delegate_config: &UploadDelegateConfig,
    file_id: &str,
    permission_id: &str,
) -> Result<(), Box<google_drive3::Error>> {
    let mut delegate = UploadDelegate::new(delegate_config);

    hub.permissions()
        .delete(file_id, permission_id)
        .param(
            "fields",
            "id,role,type,domain,emailAddress,allowFileDiscovery",
        )
        .add_scope(google_drive3::api::Scope::Full)
        .delegate(&mut delegate)
        .supports_all_drives(true)
        .doit()
        .await?;

    Ok(())
}

#[derive(Debug)]
pub(crate) enum Error {
    Hub(GetHubError),
    GetFile(Box<google_drive3::Error>),
    ListPermissions(Box<google_drive3::Error>),
    DeletePermission(
        Box<google_drive3::api::Permission>,
        Box<google_drive3::Error>,
    ),
    PermissionNotFound(String),
    UnknownPermissionType(String),
    UnknownPermissionRole(String),
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Error::Hub(source) => Some(source),
            Error::GetFile(source)
            | Error::ListPermissions(source)
            | Error::DeletePermission(_, source) => Some(source),
            Error::PermissionNotFound(_)
            | Error::UnknownPermissionType(_)
            | Error::UnknownPermissionRole(_) => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Error::Hub(_) => f.write_str("unable to get drive hub"),
            Error::GetFile(_) => f.write_str("unable to get file"),
            Error::ListPermissions(_) => f.write_str("unable to list permissions"),
            Error::DeletePermission(permission, _) => {
                write!(
                    f,
                    "unable to delete permission '{}'",
                    permission.id.as_deref().unwrap_or_default(),
                )
            }
            Error::PermissionNotFound(id) => {
                write!(f, "permission '{id}' not found")
            }
            Error::UnknownPermissionType(type_) => {
                write!(f, "unknown permission type: '{type_}'")
            }
            Error::UnknownPermissionRole(role) => write!(f, "unknown permission role: '{role}'"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) enum RevokeAction {
    #[default]
    Anyone,
    AllExceptOwner,
    Id(String),
}

impl RevokeAction {
    fn get_matching_permissions(
        &self,
        permissions: Vec<google_drive3::api::Permission>,
    ) -> matching_permissions::Iter<'_> {
        matching_permissions::Iter::new(self, permissions)
    }
}

mod matching_permissions {
    use std::{iter, vec};

    use crate::{
        common::permission,
        permissions::revoke::{Error, RevokeAction},
    };

    pub(super) struct Iter<'a>(Inner<'a>);

    enum Inner<'a> {
        Anyone(
            iter::Filter<
                vec::IntoIter<google_drive3::api::Permission>,
                for<'b> fn(&'b google_drive3::api::Permission) -> bool,
            >,
        ),
        NotOwner(
            iter::Filter<
                vec::IntoIter<google_drive3::api::Permission>,
                for<'b> fn(&'b google_drive3::api::Permission) -> bool,
            >,
        ),
        ById {
            permissions: Option<vec::IntoIter<google_drive3::api::Permission>>,
            id: &'a str,
        },
    }

    impl<'a> Iter<'a> {
        pub(super) fn new(
            action: &'a RevokeAction,
            permissions: Vec<google_drive3::api::Permission>,
        ) -> Self {
            Self(match action {
                RevokeAction::Anyone => {
                    Inner::Anyone(permissions.into_iter().filter(|permission| {
                        permission.type_.as_deref() == Some(permission::Type::Anyone.as_str())
                    }))
                }
                RevokeAction::AllExceptOwner => {
                    Inner::NotOwner(permissions.into_iter().filter(|permission| {
                        permission.role.as_deref() != Some(permission::Role::Owner.as_str())
                    }))
                }
                RevokeAction::Id(id) => Inner::ById {
                    permissions: Some(permissions.into_iter()),
                    id: id.as_str(),
                },
            })
        }
    }

    impl Iterator for Iter<'_> {
        type Item = Result<google_drive3::api::Permission, Error>;

        fn next(&mut self) -> Option<Self::Item> {
            match &mut self.0 {
                Inner::Anyone(iter) | Inner::NotOwner(iter) => iter.next().map(Ok),
                &mut Inner::ById {
                    ref mut permissions,
                    id,
                } => {
                    let permission = permissions
                        .take()?
                        .into_iter()
                        .find(|permission| permission.id.as_deref() == Some(&*id));
                    Some(permission.ok_or(Error::PermissionNotFound(id.to_owned())))
                }
            }
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            match &self.0 {
                Inner::Anyone(iter) | Inner::NotOwner(iter) => iter.size_hint(),
                Inner::ById { permissions, .. } => {
                    let len = usize::from(permissions.is_some());
                    (len, Some(len))
                }
            }
        }
    }
}

fn print_revoke_details(
    file: &google_drive3::api::File,
    permission: &google_drive3::api::Permission,
) -> Result<(), Error> {
    let type_ = permission
        .type_
        .as_deref()
        .unwrap_or_default()
        .parse::<permission::Type>()
        .map_err(|_| Error::UnknownPermissionType(permission.type_.clone().unwrap_or_default()))?;

    let role = permission
        .role
        .as_deref()
        .unwrap_or_default()
        .parse::<permission::Role>()
        .map_err(|_| Error::UnknownPermissionRole(permission.role.clone().unwrap_or_default()))?;

    if type_.requires_domain() {
        println!(
            "Revoking '{}' permission to {} '{}' for '{}'",
            role,
            type_,
            permission.domain.as_deref().unwrap_or_default(),
            file.name.as_deref().unwrap_or_default()
        );
    } else if type_.requires_email() {
        println!(
            "Revoking '{}' permission to '{}' with email '{}' for '{}'",
            role,
            type_,
            permission.email_address.as_deref().unwrap_or_default(),
            file.name.as_deref().unwrap_or_default()
        );
    } else {
        println!(
            "Revoking '{}' permission to '{}' for '{}'",
            role,
            type_,
            file.name.as_deref().unwrap_or_default()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use google_drive3::api::Permission;

    use crate::common::permission;

    use super::RevokeAction;

    #[test]
    fn get_matching_permissions_anyone() {
        let permissions = vec![
            Permission {
                display_name: Some("test1".to_string()),
                type_: Some(permission::Type::Anyone.as_str().to_owned()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test2".to_string()),
                type_: Some(permission::Type::Group.as_str().to_owned()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test3".to_string()),
                type_: Some(permission::Type::Anyone.as_str().to_owned()),
                ..Permission::default()
            },
        ];

        let permissions = RevokeAction::Anyone
            .get_matching_permissions(permissions)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(permissions
            .iter()
            .map(|permission| permission.display_name.as_ref().unwrap())
            .eq(["test1", "test3"]));
    }

    #[test]
    fn get_matching_permissions_all_except_owner() {
        let permissions = vec![
            Permission {
                display_name: Some("test1".to_string()),
                role: Some(permission::Role::Owner.as_str().to_owned()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test2".to_string()),
                role: Some(permission::Role::Organizer.as_str().to_owned()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test3".to_string()),
                role: Some(permission::Role::Owner.as_str().to_owned()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test4".to_string()),
                role: Some(permission::Role::FileOrganizer.as_str().to_owned()),
                ..Permission::default()
            },
        ];

        let permissions = RevokeAction::AllExceptOwner
            .get_matching_permissions(permissions)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            permissions
                .iter()
                .map(|permission| permission.display_name.as_ref().unwrap())
                .eq(["test2", "test4"]),
            "invalid permissions: {permissions:?}"
        );
    }

    #[test]
    fn get_matching_permissions_id() {
        let permissions = vec![
            Permission {
                display_name: Some("test1".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test2".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test3".to_string()),
                id: Some("id1".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test4".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test5".to_string()),
                id: Some("id1".to_string()),
                ..Permission::default()
            },
        ];

        let permissions = RevokeAction::Id("id1".to_string())
            .get_matching_permissions(permissions)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(
            permissions
                .iter()
                .map(|permission| permission.display_name.as_ref().unwrap())
                .eq(["test3"]),
            "invalid permissions: {permissions:?}"
        );
    }

    #[test]
    fn get_matching_permissions_id_size_hint() {
        let permissions = vec![
            Permission {
                display_name: Some("test1".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test2".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test3".to_string()),
                id: Some("id1".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test4".to_string()),
                id: Some("id2".to_string()),
                ..Permission::default()
            },
            Permission {
                display_name: Some("test5".to_string()),
                id: Some("id1".to_string()),
                ..Permission::default()
            },
        ];

        let action = RevokeAction::Id("id1".to_string());
        let mut iter = action.get_matching_permissions(permissions);

        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert!(iter.next().unwrap().is_ok());
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert!(iter.next().is_none());

        let mut iter = action.get_matching_permissions(vec![]);

        assert_eq!(iter.size_hint(), (1, Some(1)));
        assert!(iter.next().unwrap().is_err());
        assert_eq!(iter.size_hint(), (0, Some(0)));
        assert!(iter.next().is_none());
    }
}
