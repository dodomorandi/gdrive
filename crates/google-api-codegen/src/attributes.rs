//! Shared attributes and documentation text for generated items.

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use regex::Regex;
use std::{borrow::Cow, ops::Not, sync::LazyLock};

/// A sequence of Rust documentation attributes for one generated item.
#[derive(Debug, Clone)]
pub(crate) struct Documentation<'a>(DocumentationLines<'a>);

impl ToTokens for Documentation<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

/// Adds documentation attributes when Discovery supplied a non-empty description.
pub(crate) fn documentation<'a, S>(
    description: Option<&'a str>,
    fallback: impl FnOnce() -> S,
) -> Documentation<'a>
where
    S: Into<Cow<'a, str>>,
{
    let text = description
        .filter(|value| value.trim().is_empty().not())
        .map_or_else(|| fallback().into(), Cow::Borrowed);
    Documentation(DocumentationLines(text))
}

/// Wrapper that splits a Discovery description into a summary line and extended documentation.
#[derive(Debug, Clone)]
pub(crate) struct DocumentationLines<'a>(pub(crate) Cow<'a, str>);

impl ToTokens for DocumentationLines<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        static DOUBLE_NEWLINE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(:?\r?\n){2}").unwrap());
        static NEWLINE_END_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?:\r?\n)*$").unwrap());

        fn add_doc_line(tokens: &mut TokenStream, doc: &str) {
            tokens.extend(quote! { #[doc = #doc] });
        }

        let text = self.0.trim();
        if text.is_empty() {
            return;
        }

        let first_paragraph_end = DOUBLE_NEWLINE_RE
            .find(text)
            .map_or_else(|| text.len(), |m| m.start());
        let (first_paragraph, remainder) = text.split_at(first_paragraph_end);
        let remainder = &remainder[..NEWLINE_END_RE
            .find(remainder)
            .expect("end of string always exists")
            .start()];
        let (summary, summary_remainder) = split_summary(first_paragraph.trim());
        add_doc_line(tokens, summary);

        if summary_remainder.is_empty().not() {
            add_doc_line(tokens, "");
            add_doc_line(tokens, summary_remainder);
        }
        if remainder.is_empty().not() {
            add_doc_line(tokens, "");
            add_doc_line(tokens, remainder);
        }
    }
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
    let documentation = documentation(description, || "").to_token_stream();
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

fn split_summary(text: &str) -> (&str, &str) {
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
            return (text[..index + character.len_utf8()].trim(), trimmed);
        }
    }

    if let Some((summary, remainder)) = text.split_once('\n') {
        return (summary.trim(), remainder.trim());
    }
    (text, "")
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;
    use quote::{quote, ToTokens};

    use super::DocumentationLines;

    fn eq_documentation_lines<'a>(
        doc_lines: &DocumentationLines,
        actual_doc_lines: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        doc_lines.to_token_stream().to_string()
            == actual_doc_lines
                .into_iter()
                .map(|doc| {
                    quote! {
                    #[doc = #doc]
                    }
                })
                .collect::<TokenStream>()
                .to_string()
    }

    #[test]
    fn splits_summary_from_extended_documentation() {
        let lines = DocumentationLines("First sentence. Second sentence.".into());
        assert!(eq_documentation_lines(
            &lines,
            ["First sentence.", "", "Second sentence."]
        ));

        let lines = DocumentationLines("Summary.\n\nExtended description.".into());
        assert!(eq_documentation_lines(
            &lines,
            ["Summary.", "", "Extended description."]
        ));
    }

    #[test]
    fn keeps_single_sentence_descriptions_together() {
        assert!(eq_documentation_lines(
            &DocumentationLines("A single sentence.".into()),
            ["A single sentence."]
        ));
    }
}
