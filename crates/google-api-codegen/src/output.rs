//! Hierarchical output types for generated Rust files and modules.

use std::{
    borrow::Cow,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};

use crate::{
    attributes::DocumentationLines,
    error::{GenerationError, IdentifierKind},
    identifier,
};

/// A complete generated crate represented as named files.
///
/// [`ToTokens`] renders every file as an inline module declaration. Iterate over
/// [`Generated::files`] to write each file separately.
#[derive(Debug, Clone, Default)]
pub struct Generated<'a> {
    /// Files in stable output order.
    pub files: Vec<GeneratedFile<'a>>,
}

impl Generated<'_> {
    /// Returns tokens declaring the generated files as modules.
    ///
    /// This is useful when each [`GeneratedFile`] is written to its `path` and this token stream
    /// is written to the consuming crate's module declarations.
    #[must_use]
    pub fn file_declarations(&self) -> FileDeclarations<'_> {
        FileDeclarations { files: &self.files }
    }
}

impl ToTokens for Generated<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for file in &self.files {
            file.module.to_tokens(tokens);
        }
    }
}

/// A token stream that declares generated files as modules.
#[derive(Debug, Clone, Copy)]
pub struct FileDeclarations<'a> {
    files: &'a [GeneratedFile<'a>],
}

impl ToTokens for FileDeclarations<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for file in self.files {
            let name = file.module.name_ident();
            tokens.extend(quote! { pub mod #name; });
        }
    }
}

/// A generated file and the module contents written to it.
#[derive(Debug, Clone)]
pub struct GeneratedFile<'a> {
    path: PathBuf,
    module: GeneratedModule<'a>,
}

impl<'a> GeneratedFile<'a> {
    /// Creates a file from a path and its module tree.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>, module: GeneratedModule<'a>) -> Self {
        Self {
            path: path.into(),
            module,
        }
    }

    /// Returns the suggested path for this file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the module tree stored in this file.
    #[must_use]
    pub fn module(&self) -> &GeneratedModule<'_> {
        &self.module
    }
}

impl ToTokens for GeneratedFile<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.module.append_body(tokens);
    }
}

/// A named Rust module containing generated items and child modules.
#[derive(Debug, Clone)]
pub struct GeneratedModule<'a> {
    source_name: String,
    name: String,
    ident: Ident,
    documentation: DocumentationLines<'a>,
    items: Vec<TokenStream>,
    children: BTreeMap<String, GeneratedModule<'a>>,
}

impl<'a> GeneratedModule<'a> {
    /// Creates a module from a Discovery or caller-provided name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let source_name = name.into();
        let ident = identifier::IdentifierStyle::Field.ident(&source_name);
        let name = ident.to_string();
        Self {
            source_name,
            name,
            ident,
            documentation: DocumentationLines("".into()),
            items: Vec::new(),
            children: BTreeMap::new(),
        }
    }

    /// Returns the normalized Rust module name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the original name supplied by the caller.
    #[must_use]
    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    /// Sets the module-level documentation.
    pub fn set_documentation(&mut self, documentation: impl Into<Cow<'a, str>>) -> &mut Self {
        self.documentation = DocumentationLines(documentation.into());
        self
    }

    /// Adds an item to this module.
    pub fn add_item(&mut self, item: TokenStream) -> &mut Self {
        self.items.push(item);
        self
    }

    /// Adds multiple items to this module.
    pub fn add_items(&mut self, items: impl IntoIterator<Item = TokenStream> + 'a) -> &mut Self {
        self.items.extend(items);
        self
    }

    /// Adds a child module.
    ///
    /// # Errors
    ///
    /// Returns [`GenerationError::DuplicateIdentifier`] when the child name conflicts with an
    /// existing child after Rust identifier normalization.
    pub fn add_module(&mut self, module: GeneratedModule<'a>) -> Result<(), GenerationError> {
        let key = module.name.clone();
        if let Some(existing) = self.children.get(&key) {
            return Err(GenerationError::DuplicateIdentifier {
                kind: IdentifierKind::Module,
                first: existing.source_name.clone(),
                second: module.source_name,
                generated: key,
            });
        }
        self.children.insert(key, module);
        Ok(())
    }

    /// Returns this module's generated items.
    pub fn items(&self) -> impl Iterator<Item = &TokenStream> {
        self.items.iter()
    }

    /// Returns this module's child modules in stable name order.
    pub fn children(&self) -> impl Iterator<Item = &GeneratedModule<'_>> {
        self.children.values()
    }

    pub(crate) fn name_ident(&self) -> &Ident {
        &self.ident
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.items.is_empty() && self.children.is_empty()
    }

    pub(crate) fn append_body(&self, tokens: &mut TokenStream) {
        self.documentation.to_tokens(tokens);
        for item in &self.items {
            item.to_tokens(tokens);
        }
        for child in self.children.values() {
            child.to_tokens(tokens);
        }
    }
}

impl ToTokens for GeneratedModule<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.ident;
        let documentation = &self.documentation;
        let items = &self.items;
        let children = self.children.values();
        tokens.extend(quote! {
            pub mod #name {
                #documentation
                #(#items)*
                #(#children)*
            }
        });
    }
}
