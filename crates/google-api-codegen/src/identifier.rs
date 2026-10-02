//! Conversion of Discovery names into valid Rust identifiers.

use std::{
    borrow::Cow,
    fmt::Write as _,
    ops::{ControlFlow, Not},
};

use proc_macro2::{Ident, Span};

const RESERVED: &[&str] = &[
    "abstract", "as", "async", "await", "become", "box", "break", "const", "continue", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "if", "impl", "in", "let", "loop",
    "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return", "self",
    "Self", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

#[derive(Clone, Copy)]
pub(crate) enum IdentifierStyle {
    Type,
    Field,
    Variant,
}

impl IdentifierStyle {
    const fn empty_name(self) -> &'static str {
        match self {
            Self::Type => "GeneratedType",
            Self::Field => "field",
            Self::Variant => "Value",
        }
    }

    const fn digit_prefix(self) -> &'static str {
        match self {
            Self::Type => "Type",
            Self::Field => "field_",
            Self::Variant => "Value",
        }
    }

    const fn reserved_suffix(self) -> &'static str {
        match self {
            Self::Type => "Type",
            Self::Field => "_",
            Self::Variant => "Value",
        }
    }

    fn normalize(self, name: &str) -> Cow<'_, str> {
        let value = match self {
            Self::Field => snake_case(name),
            Self::Type | Self::Variant => pascal_case(name),
        };

        if value.is_empty() {
            return Cow::Borrowed(self.empty_name());
        }

        if value.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Cow::Owned(prepend(self.digit_prefix(), value.as_ref()));
        }

        if RESERVED.iter().any(|reserved| *reserved == value.as_ref()) {
            return Cow::Owned(append(value.as_ref(), self.reserved_suffix()));
        }

        value
    }

    pub(crate) fn ident(self, name: &str) -> Ident {
        Ident::new(self.normalize(name).as_ref(), Span::call_site())
    }
}

/// Returns a normalized type name suitable for diagnostics or generated names.
pub(crate) fn normalized(name: &str) -> Cow<'_, str> {
    IdentifierStyle::Type.normalize(name)
}

fn pascal_case(name: &str) -> Cow<'_, str> {
    if is_pascal_case(name) {
        return Cow::Borrowed(name);
    }

    let mut output = String::with_capacity(name.len());
    let mut start_of_word = true;

    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if start_of_word {
                output.extend(character.to_uppercase());
                start_of_word = false;
            } else {
                output.push(character);
            }
        } else {
            start_of_word = character.is_ascii();
            if start_of_word.not() {
                write!(output, "u{:x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
        }
    }

    Cow::Owned(output)
}

fn snake_case(name: &str) -> Cow<'_, str> {
    enum Separator {
        PreviousAndPending,
        Previous,
        None,
    }

    if is_snake_case(name) {
        return Cow::Borrowed(name);
    }

    let mut output = String::with_capacity(name.len());
    let mut chars = name.chars();
    if let ControlFlow::Break(separator) = chars.by_ref().try_for_each(|character| {
        if character.is_ascii_uppercase() {
            output.push(character.to_ascii_lowercase());
            ControlFlow::Break(Separator::None)
        } else if character.is_ascii_alphanumeric() {
            output.push(character);
            ControlFlow::Break(Separator::None)
        } else if character.is_ascii().not() {
            write!(output, "u{:x}", u32::from(character)).expect("writing to a String cannot fail");
            ControlFlow::Break(Separator::Previous)
        } else {
            ControlFlow::Continue(())
        }
    }) {
        chars.fold(separator, |separator, character| {
            if character.is_ascii_uppercase() {
                if matches!(separator, Separator::Previous).not() {
                    output.push('_');
                }
                output.push(character.to_ascii_lowercase());
                Separator::None
            } else if character.is_ascii_alphanumeric() {
                if matches!(separator, Separator::PreviousAndPending) {
                    output.push('_');
                }
                output.push(character);
                Separator::None
            } else if character.is_ascii().not() {
                if matches!(separator, Separator::PreviousAndPending) {
                    output.push('_');
                }
                write!(output, "u{:x}", u32::from(character))
                    .expect("writing to a String cannot fail");
                Separator::Previous
            } else {
                Separator::PreviousAndPending
            }
        });
    }

    Cow::Owned(output)
}

fn is_pascal_case(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
        && characters.all(|c| c.is_ascii_alphanumeric())
}

fn is_snake_case(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if (first.is_ascii_lowercase() || first.is_ascii_digit()).not() {
        return false;
    }

    let mut previous_was_separator = false;
    for character in characters {
        if character == '_' {
            if previous_was_separator {
                return false;
            }
            previous_was_separator = true;
        } else if character.is_ascii_lowercase() || character.is_ascii_digit() {
            previous_was_separator = false;
        } else {
            return false;
        }
    }
    previous_was_separator.not()
}

/// Prepends a prefix using one exactly sized allocation.
///
/// Identifier normalization runs for every generated name, so this avoids the formatting
/// machinery of `format!` on the common prefixing path.
fn prepend(prefix: &str, value: &str) -> String {
    let mut output = String::with_capacity(prefix.len() + value.len());
    output.push_str(prefix);
    output.push_str(value);
    output
}

/// Appends a suffix using one exactly sized allocation.
///
/// Identifier normalization runs for every generated name, so this avoids the formatting
/// machinery of `format!` on the common suffixing path.
fn append(value: &str, suffix: &str) -> String {
    let mut output = String::with_capacity(value.len() + suffix.len());
    output.push_str(value);
    output.push_str(suffix);
    output
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{normalized, pascal_case, snake_case, IdentifierStyle};

    #[test]
    fn identifiers_are_normalized() {
        assert_eq!(
            IdentifierStyle::Type.ident("file-resource").to_string(),
            "FileResource"
        );
        assert_eq!(IdentifierStyle::Type.ident("2fa").to_string(), "Type2fa");
        assert_eq!(
            IdentifierStyle::Field.ident("page-size").to_string(),
            "page_size"
        );
        assert_eq!(IdentifierStyle::Field.ident("page-").to_string(), "page");
        assert_eq!(
            IdentifierStyle::Field.ident("page--size").to_string(),
            "page_size"
        );
        assert_eq!(IdentifierStyle::Field.ident("type").to_string(), "type_");
        assert_eq!(IdentifierStyle::Variant.ident("done").to_string(), "Done");
        assert_eq!(
            IdentifierStyle::Variant.ident("Self").to_string(),
            "SelfValue"
        );
    }

    #[test]
    fn already_normalized_names_are_borrowed() {
        assert!(matches!(pascal_case("Item"), Cow::Borrowed(_)));
        assert!(matches!(snake_case("page_size"), Cow::Borrowed(_)));
        assert!(matches!(normalized("Item"), Cow::Borrowed(_)));
        assert!(matches!(
            IdentifierStyle::Type.normalize(""),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    fn test_standard_cases() {
        assert_eq!(snake_case("camelCase"), "camel_case");
        assert_eq!(snake_case("PascalCase"), "pascal_case");
        assert_eq!(snake_case("kebab-case"), "kebab_case");
        assert_eq!(snake_case("space case"), "space_case");
    }

    #[test]
    fn test_acronyms_and_consecutive_capitals() {
        // Based on your specific logic, consecutive capitals are separated.
        assert_eq!(snake_case("AB"), "a_b");
        assert_eq!(snake_case("XMLParser"), "x_m_l_parser");
    }

    #[test]
    fn test_leading_and_trailing_separators() {
        assert_eq!(snake_case("  hello world  "), "hello_world");
        assert_eq!(snake_case("___hello_world---"), "hello_world");
    }

    #[test]
    fn test_consecutive_separators() {
        assert_eq!(snake_case("hello   world"), "hello_world");
        assert_eq!(snake_case("hello-_-world"), "hello_world");
    }

    #[test]
    fn test_non_ascii_hex_escaping() {
        // '世' is U+4E16
        assert_eq!(snake_case("hello世"), "hellou4e16");
        assert_eq!(snake_case("hello 世"), "hello_u4e16");
        assert_eq!(snake_case("世world"), "u4e16world"); // Followed by lowercase
        assert_eq!(snake_case("世World"), "u4e16world"); // Followed by uppercase
    }

    #[test]
    fn test_empty_and_separators_only() {
        assert_eq!(snake_case(""), "");
        assert_eq!(snake_case("   "), "");
        assert_eq!(snake_case("-_!"), "");
    }

    #[test]
    fn test_numbers() {
        assert_eq!(snake_case("version 2"), "version_2");
        assert_eq!(snake_case("v2"), "v2");
        assert_eq!(snake_case("123hello"), "123hello");
    }
}
