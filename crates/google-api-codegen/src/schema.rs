//! Generation of typed models from Discovery JSON schemas.

use std::{borrow::Cow, collections::BTreeMap, ops::Not};

use google_api_discovery::{models::JsonSchema, RestDescription};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    attributes::{documentation, field_attributes},
    error::{GenerationError, IdentifierKind},
    identifier,
    output::GeneratedModule,
};

/// Selects how generated request fields store their values.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RequestStorage {
    /// Store values by value.
    Owned,
    /// Borrow values from a caller-provided lifetime.
    Borrowed,
    /// Allow each value to be borrowed or owned.
    Cow,
}

impl RequestStorage {
    pub(crate) fn apply(self, ty: TokenStream) -> TokenStream {
        match self {
            Self::Owned => ty,
            Self::Borrowed => quote! { &'a #ty },
            Self::Cow => quote! { ::std::borrow::Cow<'a, #ty> },
        }
    }

    pub(crate) fn needs_lifetime(self, schema: &JsonSchema) -> bool {
        matches!(self, Self::Owned).not()
            && matches!(
                schema.schema_type.as_deref(),
                Some("boolean" | "integer" | "number")
            )
            .not()
    }

    pub(crate) fn sequence(self, ty: &TokenStream) -> TokenStream {
        match self {
            Self::Owned => quote! { ::std::vec::Vec<#ty> },
            Self::Borrowed => quote! { &'a [#ty] },
            Self::Cow => quote! { ::std::borrow::Cow<'a, [#ty]> },
        }
    }
}

/// Generates the contents of the generated `schemas` module.
pub(crate) struct SchemaGenerator<'a> {
    description: &'a RestDescription,
    names: BTreeMap<&'a str, Ident>,
    generated_names: BTreeMap<String, Cow<'a, str>>,
    generated: Vec<TokenStream>,
}

impl<'a> SchemaGenerator<'a> {
    pub(crate) fn new(description: &'a RestDescription) -> Result<Self, GenerationError> {
        let mut names = BTreeMap::new();
        let mut generated_names: BTreeMap<String, Cow<'a, str>> = BTreeMap::new();

        for name in description.schemas.keys() {
            let name = name.as_str();
            let ident = identifier::IdentifierStyle::Type.ident(name);
            let generated = ident.to_string();
            if let Some(first) = generated_names.get(&generated) {
                return Err(GenerationError::DuplicateIdentifier {
                    kind: IdentifierKind::Schema,
                    first: first.as_ref().to_owned(),
                    second: name.to_owned(),
                    generated,
                });
            }
            generated_names.insert(generated, Cow::Borrowed(name));
            names.insert(name, ident);
        }

        Ok(Self {
            description,
            names,
            generated_names,
            generated: Vec::new(),
        })
    }

    pub(crate) fn prepare(&mut self) -> Result<Vec<TokenStream>, GenerationError> {
        self.description
            .schemas
            .iter()
            .map(|(name, schema)| {
                let ident = self
                    .names
                    .get(name.as_str())
                    .expect("schema names are initialized before generation")
                    .clone();
                self.named_item(&ident, name, schema)
            })
            .collect()
    }

    pub(crate) fn finish(mut self, mut named_items: Vec<TokenStream>) -> GeneratedModule {
        named_items.append(&mut self.generated);
        let mut module = GeneratedModule::new("schemas");
        module.set_documentation("Serde models generated from the Discovery document.");
        if named_items.is_empty().not() {
            module.add_item(quote! { use ::serde::{Deserialize, Serialize}; });
        }
        module.add_items(named_items);
        module
    }

    pub(crate) fn reference_type(&self, reference: &str) -> Result<TokenStream, GenerationError> {
        let name = reference
            .strip_prefix("#/schemas/")
            .unwrap_or(reference)
            .rsplit('/')
            .next()
            .unwrap_or(reference);
        let ident = self
            .names
            .get(name)
            .ok_or_else(|| GenerationError::UnknownSchema {
                reference: reference.to_owned(),
            })?;
        Ok(quote! { crate::schemas::#ident })
    }

    pub(crate) fn schema_type(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        self.schema_type_with_storage(schema, hint, RequestStorage::Owned)
    }

    pub(crate) fn schema_type_with_storage(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
        storage: RequestStorage,
    ) -> Result<TokenStream, GenerationError> {
        if let Some(reference) = schema.schema_ref.as_deref() {
            return Ok(storage.apply(self.reference_type(reference)?));
        }

        if is_enum(schema) {
            return Ok(storage.apply(self.inline_enum(hint, schema)?));
        }

        match schema.schema_type.as_deref() {
            Some("array") => {
                let item_type = if let Some(item) = schema.items.as_deref() {
                    self.schema_type_with_storage(
                        item,
                        &format!("{hint}Item"),
                        RequestStorage::Owned,
                    )?
                } else {
                    quote! { ::serde_json::Value }
                };
                Ok(match storage {
                    RequestStorage::Owned => quote! { ::std::vec::Vec<#item_type> },
                    RequestStorage::Borrowed => quote! { &'a [#item_type] },
                    RequestStorage::Cow => quote! { ::std::borrow::Cow<'a, [#item_type]> },
                })
            }
            Some("boolean") => Ok(quote! { bool }),
            Some("integer") => Ok(integer_type(schema.format.as_deref())),
            Some("number") => Ok(quote! { f64 }),
            Some("string") => Ok(match storage {
                RequestStorage::Owned => quote! { ::std::string::String },
                RequestStorage::Borrowed => quote! { &'a str },
                RequestStorage::Cow => quote! { ::std::borrow::Cow<'a, str> },
            }),
            Some("object") | None if is_object(schema) => {
                Ok(storage.apply(self.object_type(schema, hint)?))
            }
            _ => Ok(storage.apply(quote! { ::serde_json::Value })),
        }
    }

    fn named_item(
        &mut self,
        ident: &Ident,
        name: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        if is_enum(schema) {
            return Self::enum_item(ident, name, schema);
        }

        if is_object(schema) {
            return self.struct_item(ident, name, schema);
        }

        let ty = self.schema_type(schema, name)?;
        let doc = documentation(schema.description.as_deref(), || {
            format!("Generated type for the `{name}` schema.")
        });
        Ok(quote! {
            #doc
            pub type #ident = #ty;
        })
    }

    fn struct_item(
        &mut self,
        ident: &Ident,
        name: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        let mut used_names = BTreeMap::new();
        let mut fields = Vec::new();

        if let Some(properties) = schema.properties.as_ref() {
            for (property_name, property) in properties {
                let field =
                    unique_field_ident(property_name, &mut used_names, IdentifierKind::Property)?;
                let base_type = self.schema_type(
                    property,
                    &format!("{name}{}", identifier::normalized(property_name)),
                )?;
                let property_type = repeated_type(property, base_type);
                let required = property.required.unwrap_or(false)
                    || schema.annotations.as_ref().is_some_and(|annotations| {
                        annotations
                            .required
                            .iter()
                            .any(|name| name == property_name)
                    });
                let field_type = if required {
                    property_type
                } else {
                    quote! { ::core::option::Option<#property_type> }
                };
                let attributes = field_attributes(
                    property_name,
                    &field,
                    required.not(),
                    false,
                    property.description.as_deref(),
                    true,
                );
                fields.push(quote! {
                    #(#attributes)*
                    pub #field: #field_type,
                });
            }
        }

        if let Some(additional) = schema.additional_properties.as_deref() {
            let value_type = self.schema_type(additional, &format!("{name}Additional"))?;
            let field = unique_field_ident(
                "additional_properties",
                &mut used_names,
                IdentifierKind::Property,
            )?;
            let attributes = field_attributes(
                "additionalProperties",
                &field,
                true,
                true,
                Some("Additional properties not represented by named fields."),
                true,
            );
            fields.push(quote! {
                #(#attributes)*
                pub #field:
                    ::core::option::Option<::std::collections::BTreeMap<::std::string::String, #value_type>>,
            });
        }

        let doc = documentation(schema.description.as_deref(), || {
            format!("Generated model for the `{name}` schema.")
        });
        Ok(quote! {
            #doc
            #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
            pub struct #ident {
                #(#fields)*
            }
        })
    }

    fn enum_item(
        ident: &Ident,
        name: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        let values = schema
            .enum_values
            .as_deref()
            .expect("enum schemas have values");
        let mut used_names: BTreeMap<String, String> = BTreeMap::new();
        let mut variants = Vec::new();

        for (index, value) in values.iter().enumerate() {
            let variant = identifier::IdentifierStyle::Variant.ident(value);
            let generated = variant.to_string();
            if let Some(first) = used_names.get(&generated) {
                return Err(GenerationError::DuplicateIdentifier {
                    kind: IdentifierKind::EnumVariant,
                    first: first.clone(),
                    second: String::from(value.as_str()),
                    generated,
                });
            }
            used_names.insert(generated, String::from(value.as_str()));
            let doc = schema
                .enum_descriptions
                .as_deref()
                .and_then(|descriptions| descriptions.get(index))
                .map(|description| quote! { #[doc = #description] });
            variants.push(quote! {
                #doc
                #[serde(rename = #value)]
                #variant,
            });
        }

        let unknown = if used_names.contains_key("Unknown") {
            "Other"
        } else {
            "Unknown"
        };
        let unknown = Ident::new(unknown, proc_macro2::Span::call_site());
        variants.push(quote! {
            #[doc = "An unrecognized value returned by the API."]
            #[serde(other)]
            #unknown,
        });

        let doc = documentation(schema.description.as_deref(), || {
            format!("Generated values for the `{name}` schema.")
        });
        Ok(quote! {
            #doc
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
            pub enum #ident {
                #(#variants)*
            }
        })
    }

    fn object_type(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if schema
            .properties
            .as_ref()
            .is_some_and(|properties| properties.is_empty().not())
        {
            let is_new = self.has_generated_hint(hint).not();
            let ident = self.fresh_type_ident(hint);
            if is_new {
                let item = self.struct_item(&ident, hint, schema)?;
                self.generated.push(item);
            }
            return Ok(quote! { crate::schemas::#ident });
        }

        if let Some(additional) = schema.additional_properties.as_deref() {
            let value_type = self.schema_type(additional, &format!("{hint}Value"))?;
            return Ok(quote! {
                ::std::collections::BTreeMap<::std::string::String, #value_type>
            });
        }

        Ok(quote! { ::serde_json::Value })
    }

    fn inline_enum(
        &mut self,
        hint: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        let is_new = self.has_generated_hint(hint).not();
        let ident = self.fresh_type_ident(hint);
        if is_new {
            let item = Self::enum_item(&ident, hint, schema)?;
            self.generated.push(item);
        }
        Ok(quote! { crate::schemas::#ident })
    }

    fn has_generated_hint(&self, hint: &str) -> bool {
        self.generated_names
            .values()
            .any(|source| source.as_ref() == hint)
    }

    fn fresh_type_ident(&mut self, hint: &str) -> Ident {
        if let Some(generated) = self
            .generated_names
            .iter()
            .find_map(|(generated, source)| (source.as_ref() == hint).then_some(generated))
        {
            return Ident::new(generated, proc_macro2::Span::call_site());
        }
        let base = identifier::normalized(hint);
        let mut candidate = base.as_ref().to_owned();
        let mut suffix = 2;
        while self.generated_names.contains_key(&candidate) {
            candidate = format!("{base}{suffix}");
            suffix += 1;
        }
        let ident = Ident::new(&candidate, proc_macro2::Span::call_site());
        self.generated_names
            .insert(candidate, Cow::Owned(hint.to_owned()));
        ident
    }
}

fn is_object(schema: &JsonSchema) -> bool {
    schema.schema_type.as_deref() == Some("object")
        || schema.properties.is_some()
        || schema.additional_properties.is_some()
}

fn is_enum(schema: &JsonSchema) -> bool {
    matches!(schema.schema_type.as_deref(), Some("object" | "array")).not()
        && schema
            .enum_values
            .as_ref()
            .is_some_and(|values| values.is_empty().not())
}

fn integer_type(format: Option<&str>) -> TokenStream {
    match format {
        Some("int32") => quote! { i32 },
        Some("uint32") => quote! { u32 },
        Some("uint64") => quote! { u64 },
        _ => quote! { i64 },
    }
}

fn repeated_type(schema: &JsonSchema, base_type: TokenStream) -> TokenStream {
    if schema.repeated.unwrap_or(false) && schema.schema_type.as_deref() != Some("array") {
        quote! { ::std::vec::Vec<#base_type> }
    } else {
        base_type
    }
}

fn unique_field_ident(
    original: &str,
    used: &mut BTreeMap<String, String>,
    kind: IdentifierKind,
) -> Result<Ident, GenerationError> {
    let ident = identifier::IdentifierStyle::Field.ident(original);
    let generated = ident.to_string();
    if let Some(first) = used.get(&generated) {
        return Err(GenerationError::DuplicateIdentifier {
            kind,
            first: first.clone(),
            second: original.to_owned(),
            generated,
        });
    }
    used.insert(generated, original.to_owned());
    Ok(ident)
}

pub(crate) fn schema_type_name(schema: &JsonSchema) -> String {
    schema.schema_ref.as_deref().map_or_else(
        || {
            schema
                .schema_type
                .clone()
                .unwrap_or_else(|| "any".to_owned())
        },
        |reference| format!("ref:{reference}"),
    )
}
