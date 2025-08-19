use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Role {
    Owner,
    Organizer,
    FileOrganizer,
    Writer,
    Commenter,
    #[default]
    Reader,
}

const ROLES: [Role; 6] = [
    Role::Owner,
    Role::Organizer,
    Role::FileOrganizer,
    Role::Writer,
    Role::Commenter,
    Role::Reader,
];

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Role::Owner => write!(f, "owner"),
            Role::Organizer => write!(f, "organizer"),
            Role::FileOrganizer => write!(f, "fileOrganizer"),
            Role::Writer => write!(f, "writer"),
            Role::Commenter => write!(f, "commenter"),
            Role::Reader => write!(f, "reader"),
        }
    }
}

impl FromStr for Role {
    type Err = InvalidRole;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Role::Owner),
            "organizer" => Ok(Role::Organizer),
            "fileOrganizer" => Ok(Role::FileOrganizer),
            "writer" => Ok(Role::Writer),
            "commenter" => Ok(Role::Commenter),
            "reader" => Ok(Role::Reader),
            _ => Err(InvalidRole),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InvalidRole;

impl Display for InvalidRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("role is invalid, valid roles are: ")?;
        let mut roles = ROLES.iter();
        let role = roles.next().unwrap();
        write!(f, "{role}")?;
        for role in roles {
            write!(f, ", {role}")?;
        }
        Ok(())
    }
}

impl Error for InvalidRole {}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum Type {
    User,
    Group,
    Domain,
    #[default]
    Anyone,
}

impl Type {
    #[must_use]
    pub fn requires_email(&self) -> bool {
        match self {
            Type::Group | Type::User => true,
            Type::Domain | Type::Anyone => false,
        }
    }

    #[must_use]
    pub fn requires_domain(&self) -> bool {
        match self {
            Type::Domain => true,
            Type::Anyone | Type::Group | Type::User => false,
        }
    }

    #[must_use]
    pub fn supports_file_discovery(&self) -> bool {
        match self {
            Type::Group | Type::User => false,
            Type::Domain | Type::Anyone => true,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::User => write!(f, "user"),
            Type::Group => write!(f, "group"),
            Type::Domain => write!(f, "domain"),
            Type::Anyone => write!(f, "anyone"),
        }
    }
}

impl FromStr for Type {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user" => Ok(Type::User),
            "group" => Ok(Type::Group),
            "domain" => Ok(Type::Domain),
            "anyone" => Ok(Type::Anyone),
            _ => Err(format!(
                "'{s}' is not a valid type, valid types are: user, group, domain, anyone"
            )),
        }
    }
}
