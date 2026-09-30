//! Generation of sans-I/O resource and method descriptors.

use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    ops::Not,
};

use google_api_discovery::{
    models::{JsonSchema, RestMethod, RestResource},
    RestDescription,
};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

use crate::{
    attributes::{documentation, field_attributes, Documentation},
    error::{GenerationError, IdentifierKind},
    identifier,
    output::GeneratedModule,
    schema::{schema_type_name, RequestStorage, SchemaGenerator},
};

pub(crate) fn generate_resources(
    description: &RestDescription,
    schemas: &mut SchemaGenerator<'_>,
) -> Result<(GeneratedModule, GeneratedModule), GenerationError> {
    let mut api = generate_api_methods(description, schemas)?;
    let mut resources = GeneratedModule::new("resources");
    generate_resource_set(
        &mut resources,
        description,
        &description.resources,
        schemas,
        "",
    )?;
    if api.is_empty().not() {
        api.set_documentation("Methods exposed directly by the API.");
    }
    if resources.is_empty().not() {
        resources.set_documentation("Resources exposed by the API.");
    }
    Ok((api, resources))
}

fn generate_api_methods(
    description: &RestDescription,
    schemas: &mut SchemaGenerator<'_>,
) -> Result<GeneratedModule, GenerationError> {
    let groups = match description.methods.as_ref() {
        Some(methods) => generate_method_groups(methods, description, schemas, "")?,
        None => Vec::new(),
    };
    let mut module = GeneratedModule::new("api");
    for group in groups {
        let (method_module, descriptor) = method_group_module(group)?;
        module.add_module(method_module)?;
        module.add_item(descriptor);
    }
    Ok(module)
}

/// Wraps one method's items in a module that groups the storage variants.
///
/// Returns the method module and the expression that reaches its borrowed request descriptor.
fn method_group_module(
    group: MethodGroup,
) -> Result<(GeneratedModule, TokenStream), GenerationError> {
    let mut module = GeneratedModule::new(&group.name);
    module.set_documentation(format!(
        "Request values for the `{}` method.",
        group.method_id
    ));
    module.add_items(group.aliases);

    let mut borrowed = GeneratedModule::new("borrowed");
    borrowed.set_documentation(format!(
        "Borrowed request values for the `{}` method.",
        group.method_id
    ));
    borrowed.add_item(group.borrowed);
    module.add_module(borrowed)?;

    let mut owned = GeneratedModule::new("owned");
    owned.set_documentation(format!(
        "Owned request values for the `{}` method.",
        group.method_id
    ));
    owned.add_item(quote! { use ::serde::{Deserialize, Serialize}; });
    owned.add_item(group.owned);
    module.add_module(owned)?;

    let mut cow = GeneratedModule::new("cow");
    cow.set_documentation(format!(
        "Cow-backed request values for the `{}` method.",
        group.method_id
    ));
    cow.add_item(group.cow);
    module.add_module(cow)?;

    let method = module.name_ident().clone();
    let borrowed = identifier::IdentifierStyle::Field.ident("borrowed");
    let ident = group.ident;
    Ok((module, quote! { #method::#borrowed::#ident::DESCRIPTOR }))
}

fn generate_resource_set(
    parent: &mut GeneratedModule,
    description: &RestDescription,
    resources: &BTreeMap<String, RestResource>,
    schemas: &mut SchemaGenerator<'_>,
    parent_path: &str,
) -> Result<Vec<Ident>, GenerationError> {
    let mut child_names = Vec::new();

    for (name, resource) in resources {
        let mut resource_module = GeneratedModule::new(name.clone());
        let module_ident = resource_module.name_ident().clone();
        resource_module
            .set_documentation(format!("Resource `{name}` from the Discovery document."));
        let path = if parent_path.is_empty() {
            Cow::Borrowed(name.as_str())
        } else {
            Cow::Owned(format!("{parent_path}.{name}"))
        };
        let groups =
            generate_method_groups(&resource.methods, description, schemas, path.as_ref())?;
        let mut method_descriptors = Vec::with_capacity(groups.len());
        for group in groups {
            let (method_module, descriptor) = method_group_module(group)?;
            method_descriptors.push(descriptor);
            resource_module.add_module(method_module)?;
        }

        let empty_resources = BTreeMap::new();
        let nested_resources = resource.resources.as_ref().unwrap_or(&empty_resources);
        let nested_names = generate_resource_set(
            &mut resource_module,
            description,
            nested_resources,
            schemas,
            path.as_ref(),
        )?;
        let nested_descriptors = nested_names
            .iter()
            .map(|module| quote! { #module::DESCRIPTOR });
        resource_module.add_item(quote! {
            #[doc = "Resource generated from the Discovery document."]
            pub const DESCRIPTOR: crate::protocol::Resource = crate::protocol::Resource {
                name: #name,
                methods: &[#(#method_descriptors),*],
                resources: &[#(#nested_descriptors),*],
            };

            #[doc = "Returns the resource descriptor."]
            pub const fn descriptor() -> crate::protocol::Resource {
                DESCRIPTOR
            }
        });
        parent.add_module(resource_module)?;
        child_names.push(module_ident);
    }

    Ok(child_names)
}

/// The generated items for a single method, before they are placed in the module tree.
struct MethodGroup {
    /// The Discovery method name, used for the module name.
    name: String,
    /// The method's fully qualified identifier, used for documentation.
    method_id: String,
    /// The request struct name, which may differ from the method name after normalization.
    ident: Ident,
    /// The `RequestBody` and `Response` type aliases.
    aliases: Vec<TokenStream>,
    borrowed: TokenStream,
    owned: TokenStream,
    cow: TokenStream,
}

fn generate_method_groups(
    methods: &BTreeMap<String, RestMethod>,
    description: &RestDescription,
    schemas: &mut SchemaGenerator<'_>,
    resource_path: &str,
) -> Result<Vec<MethodGroup>, GenerationError> {
    let mut used_names = BTreeMap::new();
    let mut groups = Vec::with_capacity(methods.len());

    for (method_name, method) in methods {
        let ident = unique_type_ident(method_name, &mut used_names, IdentifierKind::Method)?;
        let hint = if resource_path.is_empty() {
            Cow::Borrowed(method_name.as_str())
        } else {
            Cow::Owned(format!("{resource_path}.{method_name}"))
        };
        let (borrowed, owned, cow, aliases) =
            generate_method_item_variants(method, &ident, description, schemas, hint.as_ref())?;
        groups.push(MethodGroup {
            name: method_name.clone(),
            method_id: method.id.clone(),
            ident,
            aliases,
            borrowed,
            owned,
            cow,
        });
    }

    Ok(groups)
}

struct MethodShape {
    fields: Vec<TokenStream>,
    required_arguments: Vec<TokenStream>,
    setters: Vec<TokenStream>,
    initializers: Vec<TokenStream>,
    body_type: Option<TokenStream>,
    uses_lifetime: bool,
}

fn generate_method_item_variants(
    method: &RestMethod,
    ident: &Ident,
    description: &RestDescription,
    schemas: &mut SchemaGenerator<'_>,
    hint: &str,
) -> Result<(TokenStream, TokenStream, TokenStream, Vec<TokenStream>), GenerationError> {
    let parameters = merged_parameters(description, method);
    let method_doc = documentation(Some(method.description.as_str()), || {
        format!("Request parameters for the `{}` method.", method.id)
    });
    let (owned_item, owned_shape) = generate_method_variant(
        method,
        ident,
        schemas,
        hint,
        &parameters,
        RequestStorage::Owned,
        &method_doc,
    )?;
    let mut alias_items = Vec::new();
    if let Some(body_type) = owned_shape.body_type.as_ref() {
        let doc = format!("Request body for the `{}` method.", method.id);
        alias_items.push(quote! {
            #[doc = #doc]
            pub type RequestBody = #body_type;
        });
    }
    if let Some(response_type) = method
        .response
        .as_ref()
        .map(|response| schemas.reference_type(RequestStorage::Owned, &response.schema_ref))
        .transpose()?
    {
        let doc = format!("Response body for the `{}` method.", method.id);
        alias_items.push(quote! {
            #[doc = #doc]
            pub type Response = #response_type;
        });
    }
    let (borrowed_item, _) = generate_method_variant(
        method,
        ident,
        schemas,
        hint,
        &parameters,
        RequestStorage::Borrowed,
        &method_doc,
    )?;
    let (cow_item, _) = generate_method_variant(
        method,
        ident,
        schemas,
        hint,
        &parameters,
        RequestStorage::Cow,
        &method_doc,
    )?;
    Ok((borrowed_item, owned_item, cow_item, alias_items))
}

fn generate_method_variant(
    method: &RestMethod,
    ident: &Ident,
    schemas: &mut SchemaGenerator<'_>,
    hint: &str,
    parameters: &[ParameterRef<'_>],
    storage: RequestStorage,
    documentation: &Documentation,
) -> Result<(TokenStream, MethodShape), GenerationError> {
    let shape = generate_method_shape(method, parameters, schemas, hint, storage)?;
    let has_lifetime = shape.uses_lifetime;
    let impl_generics = lifetime_generics(has_lifetime);
    let type_generics = lifetime_generics(has_lifetime);
    let struct_body = method_struct(ident, &shape.fields, documentation, has_lifetime, storage);
    let new_function =
        method_new_function(&shape.required_arguments, &shape.initializers, &method.id);
    let setters = &shape.setters;
    let descriptor = method_descriptor(method, parameters);
    let request_impl = method_request_impl(ident, has_lifetime);
    let item = quote! {
        #struct_body

        #request_impl
    };
    Ok((
        quote! {
            #item

            impl #impl_generics #ident #type_generics {
                #new_function

                #(#setters)*

                #descriptor

                #[doc = "Returns the method descriptor."]
                pub const fn descriptor() -> crate::protocol::Method {
                    Self::DESCRIPTOR
                }
            }
        },
        shape,
    ))
}

fn method_request_impl(ident: &Ident, has_lifetime: bool) -> TokenStream {
    let type_generics = if has_lifetime {
        quote! { <'_> }
    } else {
        TokenStream::new()
    };
    quote! {
        impl crate::protocol::MethodRequest for #ident #type_generics {
            const DESCRIPTOR: crate::protocol::Method = Self::DESCRIPTOR;
        }
    }
}

fn lifetime_generics(has_lifetime: bool) -> TokenStream {
    if has_lifetime {
        quote! { <'a> }
    } else {
        TokenStream::new()
    }
}

type ParameterRef<'a> = (&'a str, &'a JsonSchema);

#[expect(
    clippy::too_many_lines,
    reason = "parameter and body shapes are generated in one storage-aware pass"
)]
fn generate_method_shape(
    method: &RestMethod,
    parameters: &[ParameterRef<'_>],
    schemas: &mut SchemaGenerator<'_>,
    hint: &str,
    storage: RequestStorage,
) -> Result<MethodShape, GenerationError> {
    let mut used_names = BTreeMap::new();
    let mut shape = MethodShape {
        fields: Vec::new(),
        required_arguments: Vec::new(),
        setters: Vec::new(),
        initializers: Vec::new(),
        body_type: None,
        uses_lifetime: false,
    };

    for (parameter_name, parameter) in parameters {
        let field = unique_field_ident(parameter_name, &mut used_names, IdentifierKind::Parameter)?;
        let repeated = parameter.repeated.unwrap_or(false)
            && matches!(parameter.schema_type.as_deref(), Some("array")).not();
        let base_storage = if repeated {
            RequestStorage::Owned
        } else {
            storage
        };
        let base_type = schemas.schema_type_with_storage(
            parameter,
            &format!("{hint}{}", identifier::normalized(parameter_name)),
            base_storage,
        )?;
        let parameter_type = if repeated {
            storage.sequence(&base_type)
        } else {
            base_type
        };
        let required = parameter.required.unwrap_or(false);
        let needs_lifetime = storage.needs_lifetime(parameter)
            || (repeated && matches!(storage, RequestStorage::Owned).not());
        shape.uses_lifetime = shape.uses_lifetime || needs_lifetime;
        let is_cow_value = matches!(storage, RequestStorage::Cow) && needs_lifetime;
        let argument_type = if is_cow_value {
            quote! { impl ::core::convert::Into<#parameter_type> }
        } else {
            parameter_type.clone()
        };
        let stored_value = if is_cow_value {
            quote! { value.into() }
        } else {
            quote! { value }
        };
        let required_initializer = if is_cow_value {
            quote! { #field: #field.into() }
        } else {
            quote! { #field }
        };
        let field_type = if required {
            parameter_type
        } else {
            quote! { ::core::option::Option<#parameter_type> }
        };
        if required {
            shape
                .required_arguments
                .push(quote! { #field: #argument_type });
        }
        let attributes = field_attributes(
            parameter_name,
            &field,
            required.not(),
            false,
            parameter.description.as_deref(),
            matches!(storage, RequestStorage::Owned),
        );
        shape.fields.push(quote! {
            #(#attributes)*
            pub #field: #field_type,
        });
        if required {
            shape.initializers.push(required_initializer);
        } else {
            shape
                .initializers
                .push(quote! { #field: ::core::option::Option::None });
            let setter = format_ident!("with_{field}");
            let setter_doc = documentation(parameter.description.as_deref(), || {
                format!("Sets the `{parameter_name}` parameter.")
            });
            shape.setters.push(quote! {
                #setter_doc
                pub fn #setter(mut self, value: #argument_type) -> Self {
                    self.#field = ::core::option::Option::Some(#stored_value);
                    self
                }
            });
        }
    }

    shape.body_type = method
        .request
        .as_ref()
        .map(|request| schemas.reference_type(storage, &request.schema_ref))
        .transpose()?
        .map(|body_type| {
            if matches!(storage, RequestStorage::Cow) {
                // A smart reference is already a complete type.
                body_type
            } else {
                storage.apply(body_type)
            }
        });
    if let Some(body_type) = shape.body_type.as_ref() {
        shape.uses_lifetime = shape.uses_lifetime || matches!(storage, RequestStorage::Owned).not();
        let body_field =
            unique_field_ident("request_body", &mut used_names, IdentifierKind::RequestBody)?;
        let attributes = field_attributes(
            "body",
            &body_field,
            true,
            false,
            Some("Request body for this method."),
            matches!(storage, RequestStorage::Owned),
        );
        shape.fields.push(quote! {
            #(#attributes)*
            pub #body_field: ::core::option::Option<#body_type>,
        });
        shape
            .initializers
            .push(quote! { #body_field: ::core::option::Option::None });
        let setter = format_ident!("with_{body_field}");
        let is_cow_value = matches!(storage, RequestStorage::Cow);
        let setter_argument = if is_cow_value {
            quote! { impl ::core::convert::Into<#body_type> }
        } else {
            body_type.clone()
        };
        let stored_value = if is_cow_value {
            quote! { value.into() }
        } else {
            quote! { value }
        };
        shape.setters.push(quote! {
            #[doc = "Sets the request body for this method."]
            pub fn #setter(mut self, value: #setter_argument) -> Self {
                self.#body_field = ::core::option::Option::Some(#stored_value);
                self
            }
        });
    }

    Ok(shape)
}

fn method_descriptor(method: &RestMethod, parameters: &[ParameterRef<'_>]) -> TokenStream {
    let parameter_descriptors = parameters.iter().map(|(name, parameter)| {
        let location = parameter.location.as_deref().unwrap_or_default();
        let required = parameter.required.unwrap_or(false);
        let repeated = parameter.repeated.unwrap_or(false)
            || parameter.schema_type.as_deref() == Some("array");
        let schema_type = schema_type_name(parameter);
        quote! {
            crate::protocol::Parameter {
                name: #name,
                location: #location,
                required: #required,
                repeated: #repeated,
                schema_type: #schema_type,
            }
        }
    });
    let request = method.request.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |request| {
            let reference = &request.schema_ref;
            quote! { ::core::option::Option::Some(#reference) }
        },
    );
    let flat_path = method.flat_path.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |path| quote! { ::core::option::Option::Some(#path) },
    );
    let response = method.response.as_ref().map_or_else(
        || quote! { ::core::option::Option::None },
        |response| {
            let reference = &response.schema_ref;
            quote! { ::core::option::Option::Some(#reference) }
        },
    );
    let scopes = method.scopes.as_deref().unwrap_or_default();
    let method_id = &method.id;
    let http_method = &method.http_method;
    let method_path = &method.path;

    quote! {
        #[doc = "Static request metadata for this method."]
        pub const DESCRIPTOR: crate::protocol::Method = crate::protocol::Method {
            id: #method_id,
            http_method: #http_method,
            path: #method_path,
            flat_path: #flat_path,
            parameters: &[#(#parameter_descriptors),*],
            request: #request,
            response: #response,
            scopes: &[#(#scopes),*],
        };
    }
}

fn method_new_function(
    required_arguments: &[TokenStream],
    initializers: &[TokenStream],
    method_id: &str,
) -> TokenStream {
    let doc = format!("Creates a request for the `{method_id}` method.");
    if required_arguments.is_empty() {
        quote! {
            #[doc = #doc]
            pub fn new() -> Self {
                Self {
                    #(#initializers),*
                }
            }
        }
    } else {
        quote! {
            #[doc = #doc]
            pub fn new(#(#required_arguments),*) -> Self {
                Self {
                    #(#initializers),*
                }
            }
        }
    }
}

fn method_struct(
    ident: &Ident,
    fields: &[TokenStream],
    description: &Documentation,
    has_lifetime: bool,
    storage: RequestStorage,
) -> TokenStream {
    let generics = lifetime_generics(has_lifetime);
    let derives = if matches!(storage, RequestStorage::Owned) {
        quote! { Debug, Clone, PartialEq, Serialize, Deserialize }
    } else {
        quote! { Debug, Clone, PartialEq }
    };
    if fields.is_empty() {
        quote! {
            #description
            #[derive(#derives)]
            pub struct #ident #generics;
        }
    } else {
        quote! {
            #description
            #[derive(#derives)]
            pub struct #ident #generics {
                #(#fields)*
            }
        }
    }
}

fn merged_parameters<'a>(
    description: &'a RestDescription,
    method: &'a RestMethod,
) -> Vec<ParameterRef<'a>> {
    let mut parameters = description
        .parameters
        .iter()
        .map(|(name, schema)| (name.as_str(), schema))
        .collect::<BTreeMap<_, _>>();
    parameters.extend(
        method
            .parameters
            .iter()
            .map(|(name, schema)| (name.as_str(), schema)),
    );

    let mut ordered = Vec::with_capacity(parameters.len());
    let mut seen = BTreeSet::new();
    if let Some(parameter_order) = method.parameter_order.as_ref() {
        for name in parameter_order {
            let name = name.as_str();
            if let Some(parameter) = parameters.get(name) {
                if seen.insert(name) {
                    ordered.push((name, *parameter));
                }
            }
        }
    }
    for (name, parameter) in parameters {
        if seen.insert(name) {
            ordered.push((name, parameter));
        }
    }
    ordered
}

fn unique_type_ident(
    name: &str,
    used: &mut BTreeMap<String, String>,
    kind: IdentifierKind,
) -> Result<Ident, GenerationError> {
    let ident = identifier::IdentifierStyle::Type.ident(name);
    let generated = ident.to_string();
    if let Some(first) = used.get(&generated) {
        return Err(GenerationError::DuplicateIdentifier {
            kind,
            first: first.clone(),
            second: name.to_owned(),
            generated,
        });
    }
    used.insert(generated, name.to_owned());
    Ok(ident)
}

fn unique_field_ident(
    name: &str,
    used: &mut BTreeMap<String, String>,
    kind: IdentifierKind,
) -> Result<Ident, GenerationError> {
    let ident = identifier::IdentifierStyle::Field.ident(name);
    let generated = ident.to_string();
    if let Some(first) = used.get(&generated) {
        return Err(GenerationError::DuplicateIdentifier {
            kind,
            first: first.clone(),
            second: name.to_owned(),
            generated,
        });
    }
    used.insert(generated, name.to_owned());
    Ok(ident)
}
