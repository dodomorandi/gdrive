//! Errors reported while translating a Discovery document into Rust code.

use std::fmt;

/// Identifies the kind of item whose generated name collided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentifierKind {
    /// A generated module.
    Module,

    /// A named schema.
    Schema,

    /// A generated enum variant.
    EnumVariant,

    /// A generated method.
    Method,

    /// A generated property.
    Property,

    /// A generated method parameter.
    Parameter,

    /// A generated request body field.
    RequestBody,
}

impl IdentifierKind {
    /// Returns the human-readable name of this identifier kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Schema => "schema",
            Self::EnumVariant => "enum variant",
            Self::Method => "method",
            Self::Property => "property",
            Self::Parameter => "parameter",
            Self::RequestBody => "request body",
        }
    }
}

impl fmt::Display for IdentifierKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Identifies a problem in a Discovery document that prevents code generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationError {
    /// A schema reference names a schema absent from the document.
    UnknownSchema {
        /// The unresolved `$ref` value.
        reference: String,
    },

    /// Two API names would become the same Rust identifier.
    DuplicateIdentifier {
        /// The kind of item being generated.
        kind: IdentifierKind,

        /// The first Discovery name using the generated identifier.
        first: String,

        /// The conflicting Discovery name.
        second: String,

        /// The Rust identifier both names would use.
        generated: String,
    },
}

impl fmt::Display for GenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSchema { reference } => {
                write!(f, "schema reference `{reference}` is not defined")
            }
            Self::DuplicateIdentifier {
                kind,
                first,
                second,
                generated,
            } => write!(
                f,
                "{kind} names `{first}` and `{second}` both map to Rust identifier `{generated}`"
            ),
        }
    }
}

impl std::error::Error for GenerationError {}
