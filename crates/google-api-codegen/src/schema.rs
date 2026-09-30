//! Generation of typed models from Discovery JSON schemas.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Not,
};

use google_api_discovery::{models::JsonSchema, RestDescription};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    attributes::{documentation, field_attributes},
    error::{GenerationError, IdentifierKind},
    identifier,
    output::GeneratedModule,
};

/// Selects how generated schema and request fields store their values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RequestStorage {
    /// Store values by value.
    Owned,
    /// Borrow values from a caller-provided lifetime.
    Borrowed,
    /// Allow each value to be borrowed or owned.
    Cow,
}

impl RequestStorage {
    pub(crate) const fn all() -> [Self; 3] {
        [Self::Owned, Self::Borrowed, Self::Cow]
    }

    const fn schema_module(self) -> &'static str {
        match self {
            Self::Owned => "owned",
            Self::Borrowed => "borrowed",
            Self::Cow => "cow",
        }
    }

    const fn is_owned(self) -> bool {
        matches!(self, Self::Owned)
    }

    pub(crate) fn apply(self, ty: TokenStream) -> TokenStream {
        match self {
            Self::Owned => ty,
            Self::Borrowed => quote! { &'a #ty },
            Self::Cow => quote! { ::std::borrow::Cow<'a, #ty> },
        }
    }

    /// Returns whether a value of `schema` carries the `'a` lifetime for this storage.
    ///
    /// Owned values are stored by value, and so are numbers, booleans, and the shared enums.
    /// Every other schema is borrowed, copied into a `Cow`, or held in a smart reference.
    pub(crate) fn needs_lifetime(self, schema: &JsonSchema) -> bool {
        !self.is_owned() && !is_copied(schema)
    }

    /// Returns the field type for a nested model of type `ty`.
    ///
    /// A borrowed model holds its nested models in a [`Cow`](std::borrow::Cow), because the view
    /// that [`BorrowModel::as_borrowed`](borrowed_or_owned::BorrowModel::as_borrowed) returns is a
    /// value rather than a reference, so a borrowed model has to be able to store one.
    pub(crate) fn model(self, ty: TokenStream) -> TokenStream {
        match self {
            Self::Owned => ty,
            Self::Borrowed | Self::Cow => quote! { ::std::borrow::Cow<'a, #ty> },
        }
    }

    /// Returns the sequence type for `storage`, holding elements of type `ty`.
    ///
    /// Borrowed and cow sequences are [`Cow`](std::borrow::Cow)s rather than plain references,
    /// because a borrowed model has to be able to own its collections to lend a view of itself.
    pub(crate) fn sequence(self, ty: &TokenStream) -> TokenStream {
        match self {
            Self::Owned => quote! { ::std::vec::Vec<#ty> },
            Self::Borrowed | Self::Cow => quote! { ::std::borrow::Cow<'a, [#ty]> },
        }
    }

    /// Returns the map type for `storage`, with `String` keys and values of type `ty`.
    pub(crate) fn mapping(self, ty: &TokenStream) -> TokenStream {
        let map = quote! { ::std::collections::BTreeMap<::std::string::String, #ty> };
        match self {
            Self::Owned => map,
            Self::Borrowed | Self::Cow => quote! { ::std::borrow::Cow<'a, #map> },
        }
    }
}

/// Generates the contents of the generated schema variant modules.
pub(crate) struct SchemaGenerator<'a> {
    description: &'a RestDescription,
    names: BTreeMap<&'a str, Ident>,
    generated_names: BTreeMap<RequestStorage, BTreeMap<String, String>>,
    lifetime_required: BTreeMap<RequestStorage, BTreeSet<String>>,
    generated: BTreeMap<RequestStorage, Vec<TokenStream>>,
    objects: BTreeMap<String, ObjectTypes>,
}

/// The generated type paths for one object schema, per storage variant.
#[derive(Debug, Clone)]
struct ObjectTypes {
    owned: TokenStream,
    borrowed: TokenStream,
    cow: TokenStream,
}

impl ObjectTypes {
    /// Returns the type path for `storage`.
    fn field(&self, storage: RequestStorage) -> TokenStream {
        match storage {
            RequestStorage::Owned => self.owned.clone(),
            RequestStorage::Borrowed => self.borrowed.clone(),
            RequestStorage::Cow => self.cow.clone(),
        }
    }
}

/// The fields of one generated struct, plus the expressions that convert each field between the
/// borrowed and the owned model.
struct StructFields {
    fields: Vec<TokenStream>,
    /// Expressions that build the owned fields from a borrowed struct, keyed by field.
    to_owned: Vec<(Ident, TokenStream)>,
    /// Expressions that build the borrowed fields from an owned struct, keyed by field.
    as_borrowed: Vec<(Ident, TokenStream)>,
    all_optional: bool,
}

impl Default for StructFields {
    fn default() -> Self {
        Self {
            fields: Vec::new(),
            to_owned: Vec::new(),
            as_borrowed: Vec::new(),
            all_optional: true,
        }
    }
}

impl StructFields {
    /// Appends the fields of `other`, keeping this list's `all_optional` flag.
    fn append(&mut self, other: Self) {
        self.fields.extend(other.fields);
        self.to_owned.extend(other.to_owned);
        self.as_borrowed.extend(other.as_borrowed);
    }
}

impl<'a> SchemaGenerator<'a> {
    pub(crate) fn new(description: &'a RestDescription) -> Result<Self, GenerationError> {
        let mut names: BTreeMap<&'a str, Ident> = BTreeMap::new();
        let mut generated_names: BTreeMap<RequestStorage, BTreeMap<String, String>> =
            BTreeMap::new();
        let mut lifetime_required: BTreeMap<RequestStorage, BTreeSet<String>> = BTreeMap::new();
        let mut used_schema_names: BTreeMap<String, String> = BTreeMap::new();
        for storage in RequestStorage::all() {
            generated_names.insert(storage, BTreeMap::new());
            lifetime_required.insert(storage, BTreeSet::new());
        }

        for name in description.schemas.keys() {
            let name = name.as_str();
            let ident = identifier::IdentifierStyle::Type.ident(name);
            let generated = ident.to_string();
            if let Some(first) = used_schema_names.get(&generated) {
                return Err(GenerationError::DuplicateIdentifier {
                    kind: IdentifierKind::Schema,
                    first: first.clone(),
                    second: name.to_owned(),
                    generated,
                });
            }
            used_schema_names.insert(generated, name.to_owned());
            names.insert(name, ident);
        }

        for storage in RequestStorage::all() {
            generated_names.insert(
                storage,
                used_schema_names
                    .iter()
                    .map(|(generated, name)| (generated.clone(), name.clone()))
                    .collect(),
            );
        }

        Ok(Self {
            description,
            names,
            generated_names,
            lifetime_required,
            generated: RequestStorage::all()
                .into_iter()
                .map(|storage| (storage, Vec::new()))
                .collect(),
            objects: BTreeMap::new(),
        })
    }

    pub(crate) fn prepare_all(
        &mut self,
    ) -> Result<BTreeMap<RequestStorage, Vec<TokenStream>>, GenerationError> {
        // Object items are generated once for all storage variants, so no pass may clear them.
        for storage in RequestStorage::all() {
            self.generated.insert(storage, Vec::new());
        }
        self.precompute_lifetimes();
        RequestStorage::all()
            .into_iter()
            .map(|storage| Ok((storage, self.prepare(storage)?)))
            .collect()
    }

    fn prepare(&mut self, storage: RequestStorage) -> Result<Vec<TokenStream>, GenerationError> {
        let mut items = Vec::with_capacity(self.description.schemas.len());
        for (name, schema) in &self.description.schemas {
            let ident = self
                .names
                .get(name.as_str())
                .expect("schema names are initialized before generation")
                .clone();
            if is_object(schema) {
                self.object_types(name, name, schema, Some(ident))?;
                continue;
            }
            items.push(self.named_item(&ident, name, schema, storage)?);
        }
        Ok(items)
    }

    /// Records which named borrowed types need a lifetime parameter.
    ///
    /// A schema may reference a type that sorts later by name, so the answer has to be known
    /// before the first item is emitted.
    fn precompute_lifetimes(&mut self) {
        for (name, schema) in &self.description.schemas {
            let Some(ident) = self.names.get(name.as_str()).cloned() else {
                continue;
            };
            if is_enum(schema) {
                continue;
            }
            let has_lifetime = if is_object(schema) {
                struct_uses_lifetime(RequestStorage::Borrowed, schema)
            } else {
                type_uses_lifetime(RequestStorage::Borrowed, schema)
            };
            self.record_lifetime(RequestStorage::Borrowed, &ident, has_lifetime);
        }
    }

    pub(crate) fn finish(
        mut self,
        named_items: &BTreeMap<RequestStorage, Vec<TokenStream>>,
    ) -> GeneratedModule {
        let mut root = GeneratedModule::new("schemas");
        root.set_documentation("Serde models generated from the Discovery document.");

        for storage in RequestStorage::all() {
            let mut items = named_items.get(&storage).cloned().unwrap_or_default();
            items.extend(self.generated.remove(&storage).unwrap_or_default());
            if items.is_empty() {
                continue;
            }

            let mut module = GeneratedModule::new(storage.schema_module());
            module.set_documentation(match storage {
                RequestStorage::Owned => "Owned schema models.",
                RequestStorage::Borrowed => "Borrowed schema models.",
                RequestStorage::Cow => "Smart references to borrowed or owned models.",
            });
            if storage.is_owned() {
                module.add_item(quote! { use ::serde::{Deserialize, Serialize}; });
            }
            module.add_items(items);
            root.add_module(module)
                .expect("schema variant module names are distinct");
        }
        root
    }

    /// Returns the unwrapped type a `$ref` points at, for the given storage.
    pub(crate) fn reference_type(
        &self,
        storage: RequestStorage,
        reference: &str,
    ) -> Result<TokenStream, GenerationError> {
        let name = Self::schema_name(reference);
        let ident = self
            .names
            .get(name)
            .ok_or_else(|| GenerationError::UnknownSchema {
                reference: reference.to_owned(),
            })?;
        Ok(match storage {
            RequestStorage::Owned => quote! { crate::schemas::owned::#ident },
            RequestStorage::Borrowed => self.module_path(storage, ident),
            RequestStorage::Cow if self.referenced_is_enum(reference) => {
                quote! { crate::schemas::owned::#ident }
            }
            RequestStorage::Cow => quote! { crate::schemas::cow::#ident<'a> },
        })
    }

    /// Returns the schema name that a `$ref` points at.
    fn schema_name(reference: &str) -> &str {
        reference
            .strip_prefix("#/schemas/")
            .unwrap_or(reference)
            .rsplit('/')
            .next()
            .unwrap_or(reference)
    }

    /// Returns whether a `$ref` points at an enum, which every variant shares as a plain value.
    fn referenced_is_enum(&self, reference: &str) -> bool {
        self.description
            .schemas
            .get(Self::schema_name(reference))
            .is_some_and(is_enum)
    }

    /// Returns the complete `cow` type for `schema`.
    ///
    /// Objects and references become a smart reference to the borrowed or owned model, strings
    /// and untyped values become a `Cow`, and the shared enum type covers plain values.
    fn cow_type(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if let Some(reference) = schema.schema_ref.as_deref() {
            return self.reference_type(RequestStorage::Cow, reference);
        }

        if is_enum(schema) {
            return self.enum_path(hint, schema);
        }

        match schema.schema_type.as_deref() {
            Some("array") => {
                let item_type = if let Some(item) = schema.items.as_deref() {
                    self.cow_element_type(item, &format!("{hint}Item"))?
                } else {
                    quote! { ::serde_json::Value }
                };
                Ok(quote! { ::std::borrow::Cow<'a, [#item_type]> })
            }
            Some("boolean") => Ok(quote! { bool }),
            Some("integer") => Ok(integer_type(schema.format.as_deref())),
            Some("number") => Ok(quote! { f64 }),
            Some("string") => Ok(quote! { ::std::borrow::Cow<'a, str> }),
            _ if is_object(schema) => self.cow_object_type(hint, schema),
            _ => Ok(quote! { ::std::borrow::Cow<'a, ::serde_json::Value> }),
        }
    }

    /// Returns the type of one element inside a `cow` array.
    ///
    /// String elements are owned, because borrowing every string of an owned array would need a
    /// self-referential buffer.
    fn cow_element_type(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if schema.schema_type.as_deref() == Some("string") {
            return Ok(quote! { ::std::string::String });
        }
        self.cow_type(schema, hint)
    }

    fn cow_object_type(
        &mut self,
        hint: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        if has_properties(schema) {
            return Ok(self.object_types(hint, hint, schema, None)?.cow);
        }

        if let Some(additional) = schema.additional_properties.as_deref() {
            let value_type = self.cow_type(additional, &format!("{hint}Value"))?;
            return Ok(quote! {
                ::std::borrow::Cow<'a, ::std::collections::BTreeMap<::std::string::String, #value_type>>
            });
        }

        Ok(quote! { ::std::borrow::Cow<'a, ::serde_json::Value> })
    }

    pub(crate) fn schema_type_with_storage(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
        storage: RequestStorage,
    ) -> Result<TokenStream, GenerationError> {
        if matches!(storage, RequestStorage::Cow) {
            return self.cow_type(schema, hint);
        }

        if let Some(reference) = schema.schema_ref.as_deref() {
            let base = self.reference_type(storage, reference)?;
            return Ok(if self.referenced_is_enum(reference) {
                base
            } else {
                storage.model(base)
            });
        }

        if is_enum(schema) {
            // Enums are copied values, so every variant shares one type.
            return self.enum_path(hint, schema);
        }

        match schema.schema_type.as_deref() {
            Some("array") => {
                let item_type = if let Some(item) = schema.items.as_deref() {
                    self.array_element_type(item, &format!("{hint}Item"), storage)?
                } else {
                    quote! { ::serde_json::Value }
                };
                Ok(storage.sequence(&item_type))
            }
            Some("boolean") => Ok(quote! { bool }),
            Some("integer") => Ok(integer_type(schema.format.as_deref())),
            Some("number") => Ok(quote! { f64 }),
            Some("string") => Ok(if storage.is_owned() {
                quote! { ::std::string::String }
            } else {
                quote! { &'a str }
            }),
            Some("object") | None if is_object(schema) => {
                if has_properties(schema) {
                    return Ok(
                        storage.model(self.object_types(hint, hint, schema, None)?.field(storage))
                    );
                }
                if let Some(additional) = schema.additional_properties.as_deref() {
                    // A borrowed map has to be ownable, so it is a `Cow` rather than a reference.
                    let value_type = self.schema_type_with_storage(
                        additional,
                        &format!("{hint}Value"),
                        storage,
                    )?;
                    return Ok(storage.mapping(&value_type));
                }
                Ok(storage.apply(quote! { ::serde_json::Value }))
            }
            _ => Ok(storage.apply(quote! { ::serde_json::Value })),
        }
    }

    fn array_element_type(
        &mut self,
        schema: &JsonSchema,
        hint: &str,
        storage: RequestStorage,
    ) -> Result<TokenStream, GenerationError> {
        if schema.schema_type.as_deref() == Some("string") {
            return Ok(if matches!(storage, RequestStorage::Borrowed) {
                quote! { &'a str }
            } else {
                quote! { ::std::string::String }
            });
        }

        if let Some(reference) = schema.schema_ref.as_deref() {
            return self.reference_type(storage, reference);
        }
        if is_enum(schema) {
            return self.enum_path(hint, schema);
        }
        if is_object(schema) {
            return self.object_type(storage, hint, schema);
        }
        if schema.schema_type.as_deref() == Some("array") {
            return self.schema_type_with_storage(schema, hint, storage);
        }
        Ok(match schema.schema_type.as_deref() {
            Some("boolean") => quote! { bool },
            Some("integer") => integer_type(schema.format.as_deref()),
            Some("number") => quote! { f64 },
            _ => quote! { ::serde_json::Value },
        })
    }

    fn named_item(
        &mut self,
        ident: &Ident,
        name: &str,
        schema: &JsonSchema,
        storage: RequestStorage,
    ) -> Result<TokenStream, GenerationError> {
        if is_enum(schema) {
            if storage.is_owned() {
                return enum_item(ident, name, schema);
            }
            let doc = documentation(schema.description.as_deref(), || {
                format!("Shared values for the `{name}` schema.")
            });
            return Ok(quote! {
                #[doc = #doc]
                pub use crate::schemas::owned::#ident;
            });
        }

        let ty = self.schema_type_with_storage(schema, name, storage)?;
        let has_lifetime = type_uses_lifetime(storage, schema);
        self.record_lifetime(storage, ident, has_lifetime);
        let generics = lifetime_generics(has_lifetime);
        let doc = documentation(schema.description.as_deref(), || {
            format!("Generated type for the `{name}` schema.")
        });
        Ok(quote! {
            #doc
            pub type #ident #generics = #ty;
        })
    }

    /// Generates the owned, borrowed, and cow items for one object schema.
    ///
    /// The cow variant is an alias for a smart reference to the borrowed or owned model, and a
    /// `From` impl converts the borrowed model into the owned one. Results are cached per `hint`,
    /// so a repeated inline schema resolves to the same types.
    fn object_types(
        &mut self,
        hint: &str,
        name: &str,
        schema: &JsonSchema,
        named: Option<Ident>,
    ) -> Result<ObjectTypes, GenerationError> {
        if let Some(existing) = self.objects.get(hint) {
            return Ok(existing.clone());
        }

        let (owned_ident, borrowed_ident, cow_ident) = match named {
            Some(ident) => (ident.clone(), ident.clone(), ident.clone()),
            None => (
                self.fresh_type_ident(RequestStorage::Owned, hint),
                self.fresh_type_ident(RequestStorage::Borrowed, hint),
                self.fresh_type_ident(RequestStorage::Cow, hint),
            ),
        };
        let owned_fields = self.struct_fields(RequestStorage::Owned, name, schema)?;
        let borrowed_fields = self.struct_fields(RequestStorage::Borrowed, name, schema)?;
        let has_lifetime = struct_uses_lifetime(RequestStorage::Borrowed, schema);
        self.record_lifetime(RequestStorage::Borrowed, &borrowed_ident, has_lifetime);

        let owned = quote! { crate::schemas::owned::#owned_ident };
        let borrowed = if has_lifetime {
            quote! { crate::schemas::borrowed::#borrowed_ident<'a> }
        } else {
            quote! { crate::schemas::borrowed::#borrowed_ident }
        };
        let types = ObjectTypes {
            owned: owned.clone(),
            borrowed: borrowed.clone(),
            cow: quote! { crate::schemas::cow::#cow_ident<'a> },
        };

        let borrowed_plain = quote! { crate::schemas::borrowed::#borrowed_ident };
        self.generated
            .entry(RequestStorage::Owned)
            .or_default()
            .push(struct_item(
                RequestStorage::Owned,
                &owned_ident,
                name,
                schema,
                &owned_fields,
            ));
        self.generated
            .entry(RequestStorage::Owned)
            .or_default()
            .push(borrow_model_item(
                &owned,
                &borrowed_plain,
                has_lifetime,
                &owned_fields.as_borrowed,
            ));
        self.generated
            .entry(RequestStorage::Borrowed)
            .or_default()
            .push(struct_item(
                RequestStorage::Borrowed,
                &borrowed_ident,
                name,
                schema,
                &borrowed_fields,
            ));
        self.generated
            .entry(RequestStorage::Borrowed)
            .or_default()
            .push(to_owned_model_item(
                &owned,
                &borrowed_plain,
                has_lifetime,
                &borrowed_fields.to_owned,
            ));
        let doc = format!("A `{name}` model that is either borrowed or owned.");
        self.generated
            .entry(RequestStorage::Cow)
            .or_default()
            .push(quote! {
                #[doc = #doc]
                pub type #cow_ident<'a> =
                    ::borrowed_or_owned::MaybeOwned<'a, #borrowed, #owned>;
            });

        self.objects.insert(hint.to_owned(), types.clone());
        Ok(types)
    }

    /// Builds the fields of one struct, plus the expressions that convert each field between the
    /// borrowed and the owned model.
    fn struct_fields(
        &mut self,
        storage: RequestStorage,
        name: &str,
        schema: &JsonSchema,
    ) -> Result<StructFields, GenerationError> {
        let mut used_names = BTreeMap::new();
        let mut fields = self.property_fields(storage, name, schema, &mut used_names)?;
        if let Some(additional) = schema.additional_properties.as_deref() {
            let map = self.map_field(storage, name, additional, &mut used_names)?;
            fields.append(map);
        }
        Ok(fields)
    }

    /// Builds one field per property, plus the conversion expression for each of them.
    fn property_fields(
        &mut self,
        storage: RequestStorage,
        name: &str,
        schema: &JsonSchema,
        used_names: &mut BTreeMap<String, String>,
    ) -> Result<StructFields, GenerationError> {
        let mut fields = StructFields::default();
        for (property_name, property) in schema.properties.iter().flatten() {
            let field = unique_field_ident(property_name, used_names, IdentifierKind::Property)?;
            let hint = format!("{name}{}", identifier::normalized(property_name));
            let property_type = self.property_type(property, &hint, storage)?;
            let required = property.required.unwrap_or(false)
                || schema.annotations.as_ref().is_some_and(|annotations| {
                    annotations
                        .required
                        .iter()
                        .any(|name| name == property_name)
                });
            let field_type = if required {
                fields.all_optional = false;
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
                storage.is_owned(),
            );
            if matches!(storage, RequestStorage::Borrowed) {
                let owned = self.owned_field(property, &field, required, &hint)?;
                fields.to_owned.push((field.clone(), owned));
            }
            if storage.is_owned() {
                let borrowed = self.borrowed_field(property, &field, required, &hint)?;
                fields.as_borrowed.push((field.clone(), borrowed));
            }
            fields.fields.push(quote! {
                #(#attributes)*
                pub #field: #field_type,
            });
        }
        Ok(fields)
    }

    /// Builds the `additionalProperties` field, plus the conversion expressions for the map.
    ///
    /// A borrowed map is a `Cow`, and so is a map value that is a model or a collection, because
    /// the view of an owned model has to be able to store the whole map.
    fn map_field(
        &mut self,
        storage: RequestStorage,
        name: &str,
        additional: &JsonSchema,
        used_names: &mut BTreeMap<String, String>,
    ) -> Result<StructFields, GenerationError> {
        let mut fields = StructFields::default();
        let hint = format!("{name}Additional");
        let value_type = self.schema_type_with_storage(additional, &hint, storage)?;
        let map_type = storage.mapping(&value_type);
        let field = unique_field_ident(
            "additional_properties",
            used_names,
            IdentifierKind::Property,
        )?;
        let attributes = field_attributes(
            "additionalProperties",
            &field,
            true,
            true,
            Some("Additional properties not represented by named fields."),
            storage.is_owned(),
        );
        if matches!(storage, RequestStorage::Borrowed) {
            let owned = self.array_element_type(additional, &hint, RequestStorage::Owned)?;
            let item = self.owned_value(additional, quote! { *item }, &hint)?;
            fields.to_owned.push((
                field.clone(),
                quote! {
                    self.#field.as_ref().map(|item| {
                        ::std::borrow::Cow::Owned(
                            item.as_ref()
                                .iter()
                                .map(|(key, item)| (::core::clone::Clone::clone(key), #item))
                                .collect::<::std::collections::BTreeMap<
                                    ::std::string::String,
                                    #owned,
                                >>(),
                        )
                    })
                },
            ));
        }
        if storage.is_owned() {
            // Resolving the value type registers any inline schema it needs; the generated
            // `collect` infers its target from the field type.
            self.schema_type_with_storage(additional, &hint, RequestStorage::Borrowed)?;
            let item = self.borrowed_value(additional, quote! { item }, &hint, false)?;
            fields.as_borrowed.push((
                field.clone(),
                quote! {
                    self.#field.as_ref().map(|item| {
                        ::std::borrow::Cow::Owned(
                            item.iter()
                                .map(|(key, item)| (::core::clone::Clone::clone(key), #item))
                                .collect(),
                        )
                    })
                },
            ));
        }
        fields.fields.push(quote! {
            #(#attributes)*
            pub #field: ::core::option::Option<#map_type>,
        });
        Ok(fields)
    }

    /// Returns the expression that converts the borrowed field `field` into its owned form.
    fn owned_field(
        &mut self,
        property: &JsonSchema,
        field: &Ident,
        required: bool,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        let value = quote! { self.#field };
        if required {
            return self.owned_value(property, value, hint);
        }
        if is_copied(property) {
            return Ok(value);
        }
        if is_json_value(property) {
            return Ok(quote! { #value.as_ref().map(::core::clone::Clone::clone) });
        }
        if is_string(property) {
            return Ok(quote! { #value.map(<str as ::std::borrow::ToOwned>::to_owned) });
        }
        if is_model(property) {
            return Ok(quote! { #value.as_ref().map(|item| item.to_owned_model()) });
        }
        let item = self.owned_value(property, quote! { item }, hint)?;
        Ok(quote! { #value.as_ref().map(|item| #item) })
    }

    /// Returns the expression that converts one borrowed value into its owned form.
    ///
    /// Every generated `collect` call names its target type, so nested collections stay
    /// inferable without relying on the surrounding field type.
    fn owned_value(
        &mut self,
        property: &JsonSchema,
        value: TokenStream,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if property.repeated.unwrap_or(false) && property.schema_type.as_deref() != Some("array") {
            let element = self.schema_type_with_storage(property, hint, RequestStorage::Owned)?;
            return self.slice_value(property, &value, hint, &element);
        }
        if is_copied(property) {
            return Ok(value);
        }
        if is_json_value(property) {
            return Ok(quote! { ::core::clone::Clone::clone(#value) });
        }
        if let Some(additional) = property.additional_properties.as_deref() {
            let item_owned =
                self.schema_type_with_storage(additional, hint, RequestStorage::Owned)?;
            let item = self.owned_value(additional, quote! { *item }, hint)?;
            return Ok(quote! {
                (#value)
                    .iter()
                    .map(|(key, item)| (::core::clone::Clone::clone(key), #item))
                    .collect::<::std::collections::BTreeMap<::std::string::String, #item_owned>>()
            });
        }
        if property.schema_type.as_deref() == Some("array") {
            return match property.items.as_deref() {
                Some(item) => {
                    // The element hint has to match the one used for the field type, or an inline
                    // object element resolves to a different generated type.
                    let element_hint = format!("{hint}Item");
                    let element =
                        self.schema_type_with_storage(item, &element_hint, RequestStorage::Owned)?;
                    self.slice_value(item, &value, &element_hint, &element)
                }
                None => Ok(quote! { ::core::clone::Clone::clone(#value) }),
            };
        }
        if property.schema_type.as_deref() == Some("string") {
            return Ok(quote! { <str as ::std::borrow::ToOwned>::to_owned(#value) });
        }
        Ok(quote! { (#value).to_owned_model() })
    }

    /// Returns the expression that copies a borrowed slice into an owned vector.
    fn slice_value(
        &mut self,
        element: &JsonSchema,
        value: &TokenStream,
        hint: &str,
        owned: &TokenStream,
    ) -> Result<TokenStream, GenerationError> {
        if is_copied(element) {
            return Ok(quote! { (#value).iter().copied().collect::<::std::vec::Vec<#owned>>() });
        }
        if is_string(element) {
            // The extra deref keeps the closure from being a plain function call.
            return Ok(quote! {
                (#value)
                    .iter()
                    .map(|item| <str as ::std::borrow::ToOwned>::to_owned(*item))
                    .collect::<::std::vec::Vec<#owned>>()
            });
        }
        if is_model(element) {
            return Ok(quote! {
                (#value)
                    .iter()
                    .map(::borrowed_or_owned::ToOwnedModel::to_owned_model)
                    .collect::<::std::vec::Vec<#owned>>()
            });
        }
        let item = self.owned_value(element, quote! { item }, hint)?;
        Ok(quote! { (#value).iter().map(|item| #item).collect::<::std::vec::Vec<#owned>>() })
    }

    /// Returns the expression that builds the borrowed field `field` from an owned struct.
    fn borrowed_field(
        &mut self,
        property: &JsonSchema,
        field: &Ident,
        required: bool,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if is_copied(property) {
            return Ok(quote! { self.#field });
        }
        if is_string(property) {
            // `as_deref` already yields the borrowed `str`, so no mapping is needed.
            return Ok(if required {
                quote! { self.#field.as_str() }
            } else {
                quote! { self.#field.as_deref() }
            });
        }
        if required {
            return self.borrowed_value(property, quote! { &self.#field }, hint, false);
        }
        let item = self.borrowed_value(property, quote! { item }, hint, false)?;
        Ok(quote! { self.#field.as_ref().map(|item| #item) })
    }

    /// Returns the expression that builds one borrowed field value from an owned value.
    ///
    /// Collections become owned [`Cow`](std::borrow::Cow)s, because a borrowed model has to be
    /// able to own its collections in order to lend a view of itself. Elements of a sequence are
    /// stored by the surrounding `Cow`, so they stay plain values.
    fn borrowed_value(
        &mut self,
        property: &JsonSchema,
        value: TokenStream,
        hint: &str,
        in_sequence: bool,
    ) -> Result<TokenStream, GenerationError> {
        if is_copied(property) {
            return Ok(value);
        }
        if is_string(property) {
            return Ok(quote! { #value.as_str() });
        }
        if is_json_value(property) {
            // A field borrows the untyped value in place, while a sequence element copies it,
            // because the element type is the value itself.
            return Ok(if in_sequence {
                quote! { #value.clone() }
            } else {
                value
            });
        }
        if is_model(property) {
            let view = quote! { (#value).as_borrowed() };
            return Ok(if in_sequence {
                view
            } else {
                // The view is built by value, so a borrowed field stores it as `Cow::Owned`.
                quote! { ::std::borrow::Cow::Owned(#view) }
            });
        }
        if let Some(additional) = property.additional_properties.as_deref() {
            let value_hint = format!("{hint}Value");
            // A map value has to be ownable on its own, because only the map itself is wrapped.
            // Resolving the value type registers any inline schema it needs; the generated
            // `collect` infers its target from the field type.
            self.schema_type_with_storage(additional, &value_hint, RequestStorage::Borrowed)?;
            let item = self.borrowed_value(additional, quote! { item }, &value_hint, false)?;
            let collect = quote! {
                #value
                    .iter()
                    .map(|(key, item)| (::core::clone::Clone::clone(key), #item))
                    .collect()
            };
            return Ok(if in_sequence {
                collect
            } else {
                quote! { ::std::borrow::Cow::Owned(#collect) }
            });
        }
        if property.repeated.unwrap_or(false) && property.schema_type.as_deref() != Some("array") {
            // Resolving the element type registers any inline schema it needs. The generated
            // `collect` infers its target from the field type.
            self.array_element_type(property, hint, RequestStorage::Borrowed)?;
            let mapper = self.sequence_mapper(property, hint)?;
            return Ok(quote! { ::std::borrow::Cow::Owned(#value.iter().map(#mapper).collect()) });
        }
        if let Some(element) = property.items.as_deref() {
            // The element hint has to match the one used for the field type, or an inline object
            // element resolves to a different generated type.
            let element_hint = format!("{hint}Item");
            self.array_element_type(element, &element_hint, RequestStorage::Borrowed)?;
            let mapper = self.sequence_mapper(element, &element_hint)?;
            return Ok(quote! { ::std::borrow::Cow::Owned(#value.iter().map(#mapper).collect()) });
        }
        Ok(quote! { #value.clone() })
    }

    /// Returns the mapper for the elements of a borrowed sequence.
    ///
    /// The mapper is a plain function path where the element conversion is one, so the generated
    /// code carries no redundant closures.
    fn sequence_mapper(
        &mut self,
        element: &JsonSchema,
        hint: &str,
    ) -> Result<TokenStream, GenerationError> {
        if is_string(element) {
            return Ok(quote! { ::std::string::String::as_str });
        }
        if is_model(element) {
            return Ok(quote! { ::borrowed_or_owned::BorrowModel::as_borrowed });
        }
        let item = self.borrowed_value(element, quote! { item }, hint, true)?;
        Ok(quote! { |item| #item })
    }

    fn object_type(
        &mut self,
        storage: RequestStorage,
        hint: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        if has_properties(schema) {
            return Ok(self.object_types(hint, hint, schema, None)?.field(storage));
        }

        if let Some(additional) = schema.additional_properties.as_deref() {
            let value_type =
                self.schema_type_with_storage(additional, &format!("{hint}Value"), storage)?;
            return Ok(quote! {
                ::std::collections::BTreeMap<::std::string::String, #value_type>
            });
        }

        Ok(quote! { ::serde_json::Value })
    }

    /// Returns the shared enum type for `schema`, generating it in the owned module.
    ///
    /// Enums are copied values, so every storage variant uses the same type instead of generating
    /// a copy per variant.
    fn enum_path(
        &mut self,
        hint: &str,
        schema: &JsonSchema,
    ) -> Result<TokenStream, GenerationError> {
        let is_new = self.has_generated_hint(RequestStorage::Owned, hint).not();
        let ident = self.fresh_type_ident(RequestStorage::Owned, hint);
        if is_new {
            let item = enum_item(&ident, hint, schema)?;
            self.generated
                .entry(RequestStorage::Owned)
                .or_default()
                .push(item);
        }
        Ok(self.module_path(RequestStorage::Owned, &ident))
    }

    fn record_lifetime(&mut self, storage: RequestStorage, ident: &Ident, has_lifetime: bool) {
        if has_lifetime {
            self.lifetime_required
                .entry(storage)
                .or_default()
                .insert(ident.to_string());
        }
    }

    fn module_path(&self, storage: RequestStorage, ident: &Ident) -> TokenStream {
        let has_lifetime = self
            .lifetime_required
            .get(&storage)
            .is_some_and(|names| names.contains(&ident.to_string()));
        match (storage, has_lifetime) {
            (RequestStorage::Owned, _) => quote! { crate::schemas::owned::#ident },
            (RequestStorage::Borrowed, true) => quote! { crate::schemas::borrowed::#ident<'a> },
            (RequestStorage::Borrowed, false) => quote! { crate::schemas::borrowed::#ident },
            (RequestStorage::Cow, true) => quote! { crate::schemas::cow::#ident<'a> },
            (RequestStorage::Cow, false) => quote! { crate::schemas::cow::#ident },
        }
    }

    fn has_generated_hint(&self, storage: RequestStorage, hint: &str) -> bool {
        self.generated_names
            .get(&storage)
            .is_some_and(|names| names.values().any(|source| source == hint))
    }

    fn fresh_type_ident(&mut self, storage: RequestStorage, hint: &str) -> Ident {
        let names = self.generated_names.entry(storage).or_default();
        if let Some(generated) = names
            .iter()
            .find_map(|(generated, source)| (source == hint).then_some(generated))
        {
            return Ident::new(generated, proc_macro2::Span::call_site());
        }
        let base = identifier::normalized(hint);
        let mut candidate = base.as_ref().to_owned();
        let mut suffix = 2;
        while names.contains_key(&candidate) {
            candidate = format!("{base}{suffix}");
            suffix += 1;
        }
        let ident = Ident::new(&candidate, proc_macro2::Span::call_site());
        names.insert(candidate, hint.to_owned());
        ident
    }

    fn property_type(
        &mut self,
        property: &JsonSchema,
        hint: &str,
        storage: RequestStorage,
    ) -> Result<TokenStream, GenerationError> {
        let repeated =
            property.repeated.unwrap_or(false) && property.schema_type.as_deref() != Some("array");
        if repeated {
            let base_storage = if matches!(storage, RequestStorage::Borrowed)
                && property.schema_type.as_deref() == Some("string")
            {
                RequestStorage::Borrowed
            } else {
                RequestStorage::Owned
            };
            let base = self.schema_type_with_storage(property, hint, base_storage)?;
            return Ok(storage.sequence(&base));
        }
        self.schema_type_with_storage(property, hint, storage)
    }
}

/// Emits a struct for the fields of `schema`.
fn struct_item(
    storage: RequestStorage,
    ident: &Ident,
    name: &str,
    schema: &JsonSchema,
    fields: &StructFields,
) -> TokenStream {
    let generics = lifetime_generics(struct_uses_lifetime(storage, schema));
    let doc = documentation(schema.description.as_deref(), || {
        format!("Generated model for the `{name}` schema.")
    });
    let mut derives = vec![quote! { Debug }, quote! { Clone }];
    if fields.all_optional {
        derives.push(quote! { Default });
    }
    derives.push(quote! { PartialEq });
    if storage.is_owned() {
        derives.push(quote! { Serialize });
        derives.push(quote! { Deserialize });
    }
    let derives = quote! { #(#derives),* };
    let items = &fields.fields;
    quote! {
        #doc
        #[derive(#derives)]
        pub struct #ident #generics {
            #(#items)*
        }
    }
}

/// Emits the [`ToOwnedModel`](borrowed_or_owned::ToOwnedModel) impl for a borrowed model.
fn to_owned_model_item(
    owned: &TokenStream,
    borrowed: &TokenStream,
    has_lifetime: bool,
    fields: &[(Ident, TokenStream)],
) -> TokenStream {
    let target = if has_lifetime {
        quote! { #borrowed<'_> }
    } else {
        borrowed.clone()
    };
    let assignments = fields
        .iter()
        .map(|(field, expression)| quote! { #field: #expression });
    quote! {
        #[doc = "Copies this borrowed model into its owned counterpart."]
        impl ::borrowed_or_owned::ToOwnedModel for #target {
            type Owned = #owned;

            #[inline]
            fn to_owned_model(&self) -> Self::Owned {
                #owned {
                    #(#assignments),*
                }
            }
        }
    }
}

/// Emits the [`BorrowModel`](borrowed_or_owned::BorrowModel) impl for an owned model.
fn borrow_model_item(
    owned: &TokenStream,
    borrowed: &TokenStream,
    has_lifetime: bool,
    fields: &[(Ident, TokenStream)],
) -> TokenStream {
    let view = if has_lifetime {
        quote! { #borrowed<'a> }
    } else {
        borrowed.clone()
    };
    let assignments = fields
        .iter()
        .map(|(field, expression)| quote! { #field: #expression });
    quote! {
        #[doc = "Builds a borrowed view of this owned model, borrowing every field in place."]
        impl ::borrowed_or_owned::BorrowModel for #owned {
            type Borrowed<'a>
                = #view
            where
                Self: 'a;

            #[inline]
            fn as_borrowed(&self) -> Self::Borrowed<'_> {
                #borrowed {
                    #(#assignments),*
                }
            }
        }
    }
}

fn has_properties(schema: &JsonSchema) -> bool {
    schema
        .properties
        .as_ref()
        .is_some_and(|properties| properties.is_empty().not())
}

/// Returns whether a value of `schema` is a model, which converts through the model pair.
fn is_model(schema: &JsonSchema) -> bool {
    schema.schema_ref.is_some() || has_properties(schema)
}

/// Returns whether a value of `schema` is stored by value in the borrowed and owned variants.
fn is_copied(schema: &JsonSchema) -> bool {
    is_enum(schema)
        || matches!(
            schema.schema_type.as_deref(),
            Some("boolean" | "integer" | "number")
        )
}

/// Returns whether a value of `schema` is a string.
fn is_string(schema: &JsonSchema) -> bool {
    schema.schema_type.as_deref() == Some("string")
}

/// Returns whether a value of `schema` falls back to an untyped JSON value.
fn is_json_value(schema: &JsonSchema) -> bool {
    schema.schema_ref.is_none()
        && !is_enum(schema)
        && !is_object(schema)
        && !matches!(
            schema.schema_type.as_deref(),
            Some("array" | "boolean" | "integer" | "number" | "string")
        )
}

/// Emits an enum for the values of `schema`.
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
        let serde_attribute = quote! { #[serde(rename = #value)] };
        variants.push(quote! {
            #doc
            #serde_attribute
            #variant,
        });
    }

    let unknown = if used_names.contains_key("Unknown") {
        "Other"
    } else {
        "Unknown"
    };
    let unknown = Ident::new(unknown, proc_macro2::Span::call_site());
    let unknown_attribute = quote! { #[serde(other)] };
    variants.push(quote! {
        #[doc = "An unrecognized value returned by the API."]
        #unknown_attribute
        #unknown,
    });

    let doc = documentation(schema.description.as_deref(), || {
        format!("Generated values for the `{name}` schema.")
    });
    let derives = quote! { Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize };
    Ok(quote! {
        #doc
        #[derive(#derives)]
        pub enum #ident {
            #(#variants)*
        }
    })
}

/// Returns whether a struct for `schema` needs a lifetime parameter.
///
/// Struct fields are optional wrappers around [`type_uses_lifetime`], and additional properties
/// are always wrapped in a borrowed or copied map.
fn struct_uses_lifetime(storage: RequestStorage, schema: &JsonSchema) -> bool {
    !storage.is_owned()
        && (schema.additional_properties.is_some()
            || schema.properties.as_ref().is_some_and(|properties| {
                properties
                    .values()
                    .any(|property| type_uses_lifetime(storage, property))
            }))
}

/// Returns whether a value of `schema` carries the `'a` lifetime for `storage`.
fn type_uses_lifetime(storage: RequestStorage, schema: &JsonSchema) -> bool {
    storage.needs_lifetime(schema)
}

fn lifetime_generics(has_lifetime: bool) -> TokenStream {
    if has_lifetime {
        quote! { <'a> }
    } else {
        TokenStream::new()
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
