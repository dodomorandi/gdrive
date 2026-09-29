//! Shared attributes and documentation text for generated items.

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use std::ops::Not;

/// A sequence of Rust documentation attributes for one generated item.
#[derive(Debug, Clone)]
pub(crate) struct Documentation {
    attributes: Vec<TokenStream>,
}

impl ToTokens for Documentation {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for attribute in &self.attributes {
            attribute.to_tokens(tokens);
        }
    }
}

/// Adds documentation attributes when Discovery supplied a non-empty description.
pub(crate) fn documentation(
    description: Option<&str>,
    fallback: impl FnOnce() -> String,
) -> Documentation {
    let text = description
        .filter(|value| value.trim().is_empty().not())
        .map_or_else(fallback, ToOwned::to_owned);
    Documentation {
        attributes: documentation_lines(&text)
            .into_iter()
            .map(|line| quote! { #[doc = #line] })
            .collect(),
    }
}

/// Splits a Discovery description into a summary line and extended documentation.
pub(crate) fn documentation_lines(text: &str) -> Vec<String> {
    let text = text.replace("\r\n", "\n");
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let first_paragraph_end = text.find("\n\n").unwrap_or(text.len());
    let first_paragraph = text[..first_paragraph_end].trim();
    let remainder = text[first_paragraph_end..].trim_matches('\n');
    let (summary, summary_remainder) = split_summary(first_paragraph);
    let mut lines = vec![summary];

    if summary_remainder.is_empty().not() {
        lines.push(String::new());
        lines.extend(summary_remainder.lines().map(str::to_owned));
    }
    if remainder.is_empty().not() {
        lines.push(String::new());
        lines.extend(remainder.lines().map(str::to_owned));
    }
    lines
}

/// Builds serde and documentation attributes for a generated field.
pub(crate) fn field_attributes(
    json_name: &str,
    field: &Ident,
    optional: bool,
    flatten: bool,
    description: Option<&str>,
    serde_attributes: bool,
) -> Vec<TokenStream> {
    let mut attributes = Vec::new();
    let documentation = documentation(description, String::new).to_token_stream();
    if documentation.is_empty().not() {
        attributes.push(documentation);
    }
    if serde_attributes {
        if field != json_name {
            attributes.push(quote! { #[serde(rename = #json_name)] });
        }
        if flatten {
            attributes.push(
                quote! { #[serde(flatten, default, skip_serializing_if = "Option::is_none")] },
            );
        } else if optional {
            attributes.push(quote! { #[serde(default, skip_serializing_if = "Option::is_none")] });
        }
    }
    attributes
}

fn split_summary(text: &str) -> (String, String) {
    for (index, character) in text.char_indices() {
        if character != '.' {
            continue;
        }
        let after_period = &text[index + character.len_utf8()..];
        let trimmed = after_period.trim_start();
        if after_period.starts_with(char::is_whitespace)
            && trimmed
                .chars()
                .next()
                .is_some_and(|next| next.is_uppercase() || matches!(next, '[' | '(' | '`'))
        {
            return (
                text[..index + character.len_utf8()].trim().to_owned(),
                trimmed.to_owned(),
            );
        }
    }

    if let Some((summary, remainder)) = text.split_once('\n') {
        return (summary.trim().to_owned(), remainder.trim().to_owned());
    }
    (text.to_owned(), String::new())
}

#[cfg(test)]
mod tests {
    use super::documentation_lines;

    #[test]
    fn splits_summary_from_extended_documentation() {
        let lines = documentation_lines("First sentence. Second sentence.");
        assert_eq!(
            lines.iter().map(String::as_str).collect::<Vec<_>>(),
            ["First sentence.", "", "Second sentence."]
        );

        let lines = documentation_lines("Summary.\n\nExtended description.");
        assert_eq!(
            lines.iter().map(String::as_str).collect::<Vec<_>>(),
            ["Summary.", "", "Extended description."]
        );
    }

    #[test]
    fn keeps_single_sentence_descriptions_together() {
        assert_eq!(
            documentation_lines("A single sentence."),
            ["A single sentence."]
        );
    }
}
