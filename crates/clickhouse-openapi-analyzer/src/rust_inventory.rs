use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::{
    Attribute, Expr, Fields, FnArg, GenericArgument, ImplItem, Item, Lit, Meta, Pat, PathArguments,
    Type, Visibility,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RustSourceError {
    #[error("failed to read Rust source {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse Rust source {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: syn::Error,
    },
    #[error(
        "could not resolve module `{module}` declared in {declared_in}; tried {first} and {second}"
    )]
    ModuleNotFound {
        module: String,
        declared_in: PathBuf,
        first: PathBuf,
        second: PathBuf,
    },
    #[error("invalid Rust source: {0}")]
    Inventory(#[from] syn::Error),
}

#[derive(Default)]
struct ModuleTree {
    items: Vec<Item>,
}

impl ModuleTree {
    fn load(source_root: &Path, root_name: &str) -> Result<Self, RustSourceError> {
        let file_root = source_root.join(format!("{root_name}.rs"));
        let directory_root = source_root.join(root_name).join("mod.rs");
        let root = if file_root.is_file() {
            file_root
        } else if directory_root.is_file() {
            directory_root
        } else {
            return Err(RustSourceError::ModuleNotFound {
                module: root_name.to_string(),
                declared_in: source_root.join("lib.rs"),
                first: file_root,
                second: directory_root,
            });
        };

        let mut tree = Self::default();
        let mut loaded = BTreeSet::new();
        tree.load_file(&root, &mut loaded)?;
        Ok(tree)
    }

    #[cfg(test)]
    fn parse(source: &str, name: &str) -> Result<Self, RustSourceError> {
        let file = syn::parse_file(source).map_err(|source| RustSourceError::Parse {
            path: PathBuf::from(name),
            source,
        })?;
        Ok(Self { items: file.items })
    }

    fn load_file(
        &mut self,
        path: &Path,
        loaded: &mut BTreeSet<PathBuf>,
    ) -> Result<(), RustSourceError> {
        if !loaded.insert(path.to_path_buf()) {
            return Ok(());
        }
        let source = fs::read_to_string(path).map_err(|source| RustSourceError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        let file = syn::parse_file(&source).map_err(|source| RustSourceError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        let module_dir = child_module_dir(path);
        self.collect_items(path, &module_dir, false, file.items, loaded)
    }

    fn collect_items(
        &mut self,
        source_file: &Path,
        module_dir: &Path,
        inside_inline_module: bool,
        items: Vec<Item>,
        loaded: &mut BTreeSet<PathBuf>,
    ) -> Result<(), RustSourceError> {
        for item in items {
            let Item::Mod(item_mod) = item else {
                self.items.push(item);
                continue;
            };

            let module_name = item_mod.ident.unraw().to_string();
            let options = module_options(&item_mod.attrs)?;
            if !options.active {
                continue;
            }

            if let Some((_, items)) = item_mod.content {
                let child_dirs = if let Some(path) = options.path {
                    BTreeSet::from([module_path_base(
                        source_file,
                        module_dir,
                        inside_inline_module,
                    )
                    .join(path)])
                } else if options.conditional_paths.is_empty() {
                    BTreeSet::from([module_dir.join(&module_name)])
                } else {
                    let base = module_path_base(source_file, module_dir, inside_inline_module);
                    let conventional = module_dir.join(&module_name);
                    let mut paths = options
                        .conditional_paths
                        .into_iter()
                        .map(|path| base.join(path))
                        .filter(|path| path.exists())
                        .collect::<BTreeSet<_>>();
                    if conventional.exists() {
                        paths.insert(conventional.clone());
                    }
                    if paths.is_empty() {
                        paths.insert(conventional);
                    }
                    paths
                };
                for child_dir in child_dirs {
                    self.collect_items(source_file, &child_dir, true, items.clone(), loaded)?;
                }
                continue;
            }

            if let Some(path) = options.path {
                let path =
                    module_path_base(source_file, module_dir, inside_inline_module).join(path);
                self.load_file(&path, loaded)?;
                continue;
            }

            let base = module_path_base(source_file, module_dir, inside_inline_module);
            let mut module_loaded = false;
            for path in options.conditional_paths {
                let path = base.join(path);
                if path.is_file() {
                    self.load_file(&path, loaded)?;
                    module_loaded = true;
                }
            }

            let file_path = module_dir.join(format!("{module_name}.rs"));
            let directory_path = module_dir.join(&module_name).join("mod.rs");
            if file_path.is_file() {
                self.load_file(&file_path, loaded)?;
            } else if directory_path.is_file() {
                self.load_file(&directory_path, loaded)?;
            } else if !module_loaded {
                return Err(RustSourceError::ModuleNotFound {
                    module: module_name,
                    declared_in: source_file.to_path_buf(),
                    first: file_path,
                    second: directory_path,
                });
            }
        }
        Ok(())
    }
}

fn module_path_base<'a>(
    source_file: &'a Path,
    module_dir: &'a Path,
    inside_inline_module: bool,
) -> &'a Path {
    if inside_inline_module {
        module_dir
    } else {
        source_file.parent().unwrap_or(Path::new(""))
    }
}

fn child_module_dir(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|name| name == "mod.rs") {
        path.parent().unwrap_or(Path::new("")).to_path_buf()
    } else {
        path.with_extension("")
    }
}

struct ModuleOptions {
    active: bool,
    path: Option<PathBuf>,
    conditional_paths: BTreeSet<PathBuf>,
}

fn module_options(attributes: &[Attribute]) -> syn::Result<ModuleOptions> {
    let mut options = ModuleOptions {
        active: true,
        path: None,
        conditional_paths: BTreeSet::new(),
    };
    for attribute in attributes {
        apply_module_attribute(&attribute.meta, &mut options)?;
    }
    Ok(options)
}

fn apply_module_attribute(meta: &Meta, options: &mut ModuleOptions) -> syn::Result<()> {
    if meta.path().is_ident("cfg") {
        let Meta::List(list) = meta else {
            return Err(syn::Error::new_spanned(meta, "expected #[cfg(...)]"));
        };
        let predicate = syn::parse2::<Meta>(list.tokens.clone())?;
        if evaluate_cfg(&predicate)? == CfgValue::False {
            options.active = false;
        }
    } else if meta.path().is_ident("cfg_attr") {
        let Meta::List(list) = meta else {
            return Err(syn::Error::new_spanned(meta, "expected #[cfg_attr(...)]"));
        };
        let nested = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())?;
        let mut nested = nested.iter();
        let Some(predicate) = nested.next() else {
            return Err(syn::Error::new_spanned(
                meta,
                "cfg_attr requires a predicate",
            ));
        };
        match evaluate_cfg(predicate)? {
            CfgValue::True => {
                for attribute in nested {
                    apply_module_attribute(attribute, options)?;
                }
            }
            CfgValue::Unknown => {
                // Unknown cfgs must not hide API through nested cfg attributes,
                // but every path they could select remains inventory-relevant.
                for attribute in nested {
                    collect_conditional_module_paths(attribute, options)?;
                }
            }
            CfgValue::False => {}
        }
    } else if meta.path().is_ident("path") {
        options.path = Some(parse_module_path(meta)?);
    }
    Ok(())
}

fn collect_conditional_module_paths(meta: &Meta, options: &mut ModuleOptions) -> syn::Result<()> {
    if meta.path().is_ident("path") {
        options.conditional_paths.insert(parse_module_path(meta)?);
    } else if meta.path().is_ident("cfg_attr") {
        let Meta::List(list) = meta else {
            return Err(syn::Error::new_spanned(meta, "expected #[cfg_attr(...)]"));
        };
        let nested = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
            .parse2(list.tokens.clone())?;
        let mut nested = nested.iter();
        let Some(predicate) = nested.next() else {
            return Err(syn::Error::new_spanned(
                meta,
                "cfg_attr requires a predicate",
            ));
        };
        if evaluate_cfg(predicate)? != CfgValue::False {
            for attribute in nested {
                collect_conditional_module_paths(attribute, options)?;
            }
        }
    }
    Ok(())
}

fn parse_module_path(meta: &Meta) -> syn::Result<PathBuf> {
    let Meta::NameValue(name_value) = meta else {
        return Err(syn::Error::new_spanned(meta, "expected #[path = \"...\"]"));
    };
    let Expr::Lit(literal) = &name_value.value else {
        return Err(syn::Error::new_spanned(
            meta,
            "expected a string module path",
        ));
    };
    let Lit::Str(value) = &literal.lit else {
        return Err(syn::Error::new_spanned(
            meta,
            "expected a string module path",
        ));
    };
    Ok(PathBuf::from(value.value()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CfgValue {
    True,
    False,
    Unknown,
}

fn evaluate_cfg(predicate: &Meta) -> syn::Result<CfgValue> {
    match predicate {
        Meta::Path(path) if path.is_ident("test") || path.is_ident("false") => Ok(CfgValue::False),
        Meta::Path(path) if path.is_ident("true") => Ok(CfgValue::True),
        Meta::Path(path) if path.is_ident("unix") => Ok(CfgValue::from(cfg!(unix))),
        Meta::Path(path) if path.is_ident("windows") => Ok(CfgValue::from(cfg!(windows))),
        Meta::Path(path) if path.is_ident("debug_assertions") => {
            Ok(CfgValue::from(cfg!(debug_assertions)))
        }
        Meta::Path(path) if path.is_ident("proc_macro") => Ok(CfgValue::from(cfg!(proc_macro))),
        // Unknown custom cfgs are retained so API inventory cannot silently
        // disappear just because the analyzer was not passed a crate-specific
        // `--cfg` flag.
        Meta::Path(_) => Ok(CfgValue::Unknown),
        Meta::NameValue(value) => evaluate_name_value_cfg(value),
        Meta::List(list) if list.path.is_ident("not") => {
            let nested = syn::parse2::<Meta>(list.tokens.clone())?;
            Ok(match evaluate_cfg(&nested)? {
                CfgValue::True => CfgValue::False,
                CfgValue::False => CfgValue::True,
                CfgValue::Unknown => CfgValue::Unknown,
            })
        }
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let nested = syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated
                .parse2(list.tokens.clone())?;
            let values = nested
                .iter()
                .map(evaluate_cfg)
                .collect::<syn::Result<Vec<_>>>()?;
            if list.path.is_ident("all") {
                if values.contains(&CfgValue::False) {
                    Ok(CfgValue::False)
                } else if values.iter().all(|value| *value == CfgValue::True) {
                    Ok(CfgValue::True)
                } else {
                    Ok(CfgValue::Unknown)
                }
            } else if values.contains(&CfgValue::True) {
                Ok(CfgValue::True)
            } else if values.iter().all(|value| *value == CfgValue::False) {
                Ok(CfgValue::False)
            } else {
                Ok(CfgValue::Unknown)
            }
        }
        Meta::List(_) => Ok(CfgValue::Unknown),
    }
}

impl From<bool> for CfgValue {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

fn evaluate_name_value_cfg(value: &syn::MetaNameValue) -> syn::Result<CfgValue> {
    let Expr::Lit(literal) = &value.value else {
        return Ok(CfgValue::Unknown);
    };
    let Lit::Str(configured) = &literal.lit else {
        return Ok(CfgValue::Unknown);
    };
    let configured = configured.value();

    // Analyzer inventory is intentionally all-features: feature-gated public
    // API (including deprecated fields) must remain visible to drift checks.
    if value.path.is_ident("feature") {
        return Ok(CfgValue::True);
    }
    let actual = if value.path.is_ident("target_arch") {
        Some(std::env::consts::ARCH)
    } else if value.path.is_ident("target_os") {
        Some(std::env::consts::OS)
    } else if value.path.is_ident("target_family") {
        Some(std::env::consts::FAMILY)
    } else if value.path.is_ident("target_env") {
        Some(env!("ANALYZER_TARGET_ENV"))
    } else if value.path.is_ident("target_vendor") {
        Some(env!("ANALYZER_TARGET_VENDOR"))
    } else if value.path.is_ident("target_endian") {
        Some(if cfg!(target_endian = "little") {
            "little"
        } else {
            "big"
        })
    } else if value.path.is_ident("target_pointer_width") {
        Some(if usize::BITS == 64 {
            "64"
        } else if usize::BITS == 32 {
            "32"
        } else {
            "16"
        })
    } else if value.path.is_ident("panic") {
        Some(if cfg!(panic = "unwind") {
            "unwind"
        } else {
            "abort"
        })
    } else {
        None
    };

    Ok(actual
        .map(|actual| CfgValue::from(actual == configured))
        .unwrap_or(CfgValue::Unknown))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypeNode {
    Option(Box<TypeNode>),
    Vec(Box<TypeNode>),
    Boxed(Box<TypeNode>),
    Reference(Box<TypeNode>),
    Path(String),
    Other(String),
}

impl TypeNode {
    fn from_syn(ty: &Type) -> Self {
        match ty {
            Type::Reference(reference) => {
                Self::Reference(Box::new(Self::from_syn(&reference.elem)))
            }
            Type::Path(type_path) => {
                let Some(segment) = type_path.path.segments.last() else {
                    return Self::Other(ty.to_token_stream().to_string());
                };
                let name = segment.ident.unraw().to_string();
                let first_type = match &segment.arguments {
                    PathArguments::AngleBracketed(arguments) => {
                        arguments.args.iter().find_map(|arg| {
                            if let GenericArgument::Type(inner) = arg {
                                Some(Self::from_syn(inner))
                            } else {
                                None
                            }
                        })
                    }
                    _ => None,
                };
                match (name.as_str(), first_type) {
                    ("Option", Some(inner)) => Self::Option(Box::new(inner)),
                    ("Vec", Some(inner)) => Self::Vec(Box::new(inner)),
                    ("Box", Some(inner)) => Self::Boxed(Box::new(inner)),
                    _ => Self::Path(name),
                }
            }
            Type::Slice(slice) => Self::Vec(Box::new(Self::from_syn(&slice.elem))),
            Type::Array(array) => Self::Vec(Box::new(Self::from_syn(&array.elem))),
            _ => Self::Other(ty.to_token_stream().to_string()),
        }
    }

    pub(crate) fn is_option(&self) -> bool {
        match self {
            Self::Option(_) => true,
            Self::Reference(inner) | Self::Boxed(inner) => inner.is_option(),
            _ => false,
        }
    }

    fn terminal(&self) -> Option<&str> {
        match self {
            Self::Option(inner)
            | Self::Vec(inner)
            | Self::Boxed(inner)
            | Self::Reference(inner) => inner.terminal(),
            Self::Path(name) => Some(name),
            Self::Other(_) => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn display(&self) -> String {
        match self {
            Self::Option(inner) => format!("Option<{}>", inner.display()),
            Self::Vec(inner) => format!("Vec<{}>", inner.display()),
            Self::Boxed(inner) => format!("Box<{}>", inner.display()),
            Self::Reference(inner) => format!("&{}", inner.display()),
            Self::Path(name) | Self::Other(name) => name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldInfo {
    pub(crate) rust_name: String,
    pub(crate) rust_type: TypeNode,
    /// Every named type mentioned anywhere in the field's type, including
    /// inside generic arguments the [`TypeNode`] shape model does not follow
    /// (e.g. map values). Used for response-tree reachability.
    pub(crate) type_names: BTreeSet<String>,
    pub(crate) deprecated_marker: bool,
    /// Whether the field carries a field-level `#[serde(default)]` (bare or
    /// `default = "path"`). Banned repository-wide: on a required request
    /// field it fabricates a value the server never sent, and on `Option`
    /// response fields it is meaningless (see the policy test in
    /// `clickhouse-cloud-api/tests/spec_coverage_test.rs`).
    pub(crate) serde_default: bool,
    /// Whether the field carries `#[serde(skip_serializing_if = "...")]`.
    /// Response-tree `Option` fields must all carry it so an absent field is
    /// omitted from serialized output rather than emitted as `null`.
    pub(crate) skip_serializing_if: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructInfo {
    pub(crate) fields: BTreeMap<String, FieldInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnumInfo {
    pub(crate) values: BTreeSet<String>,
    pub(crate) is_value_enum: bool,
    pub(crate) values_const: Option<BTreeSet<String>>,
    /// Named types carried by any variant's payload (union arms), so
    /// response-tree reachability can traverse data-carrying enums.
    pub(crate) variant_type_names: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodInfo {
    pub(crate) arguments: BTreeMap<String, TypeNode>,
    /// Every named type mentioned anywhere in the method's return type
    /// (e.g. `ApiResponse` and `Service` in `Result<ApiResponse<Vec<Service>>, Error>`).
    pub(crate) return_type_names: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MetadataInventory {
    pub(crate) beta_operations: BTreeSet<String>,
    pub(crate) deprecated_fields: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RustInventory {
    pub(crate) client_methods: BTreeMap<String, MethodInfo>,
    pub(crate) model_types: BTreeSet<String>,
    pub(crate) structs: BTreeMap<String, StructInfo>,
    pub(crate) enums: BTreeMap<String, EnumInfo>,
    pub(crate) aliases: BTreeMap<String, TypeNode>,
    /// Named types mentioned anywhere in each alias's target type, for
    /// response-tree reachability (parallel to `aliases`).
    pub(crate) alias_type_names: BTreeMap<String, BTreeSet<String>>,
    pub(crate) manual_default_impls: BTreeSet<String>,
    pub(crate) metadata: MetadataInventory,
}

impl RustInventory {
    pub(crate) fn load(source_root: &Path) -> Result<Self, RustSourceError> {
        let client = ModuleTree::load(source_root, "client")?;
        let models = ModuleTree::load(source_root, "models")?;
        let meta = ModuleTree::load(source_root, "meta")?;
        Self::from_trees(&client, &models, &meta)
    }

    #[cfg(test)]
    pub(crate) fn parse(client: &str, models: &str, meta: &str) -> Result<Self, RustSourceError> {
        let client = ModuleTree::parse(client, "client.rs")?;
        let models = ModuleTree::parse(models, "models.rs")?;
        let meta = ModuleTree::parse(meta, "meta.rs")?;
        Self::from_trees(&client, &models, &meta)
    }

    fn from_trees(
        client: &ModuleTree,
        models: &ModuleTree,
        meta: &ModuleTree,
    ) -> Result<Self, RustSourceError> {
        let mut inventory = Self::default();
        inventory.collect_client(&client.items)?;
        inventory.collect_models(&models.items)?;
        inventory.collect_metadata(&meta.items);
        Ok(inventory)
    }

    fn collect_client(&mut self, items: &[Item]) -> syn::Result<()> {
        for item in items {
            let Item::Impl(item_impl) = item else {
                continue;
            };
            let Type::Path(self_type) = item_impl.self_ty.as_ref() else {
                continue;
            };
            if self_type
                .path
                .segments
                .last()
                .map(|segment| segment.ident.unraw().to_string())
                != Some("Client".to_string())
            {
                continue;
            }

            for impl_item in &item_impl.items {
                let ImplItem::Fn(function) = impl_item else {
                    continue;
                };
                if !matches!(function.vis, Visibility::Public(_))
                    || function.sig.asyncness.is_none()
                {
                    continue;
                }
                let mut arguments = BTreeMap::new();
                for input in &function.sig.inputs {
                    let FnArg::Typed(argument) = input else {
                        continue;
                    };
                    let Pat::Ident(ident) = argument.pat.as_ref() else {
                        continue;
                    };
                    arguments.insert(
                        ident.ident.unraw().to_string(),
                        TypeNode::from_syn(&argument.ty),
                    );
                }
                let mut return_type_names = BTreeSet::new();
                if let syn::ReturnType::Type(_, ty) = &function.sig.output {
                    collect_type_names(ty, &mut return_type_names);
                }
                self.client_methods.insert(
                    function.sig.ident.unraw().to_string(),
                    MethodInfo {
                        arguments,
                        return_type_names,
                    },
                );
            }
        }
        Ok(())
    }

    fn collect_models(&mut self, items: &[Item]) -> syn::Result<()> {
        for item in items {
            match item {
                Item::Struct(item_struct) if matches!(item_struct.vis, Visibility::Public(_)) => {
                    let name = item_struct.ident.unraw().to_string();
                    self.model_types.insert(name.clone());
                    // Also rejects a banned `rename_all` on the struct container.
                    let container = serde_options(&item_struct.attrs)?;
                    let mut fields = BTreeMap::new();
                    if let Fields::Named(named) = &item_struct.fields {
                        for field in &named.named {
                            if !matches!(field.vis, Visibility::Public(_)) {
                                continue;
                            }
                            let Some(ident) = &field.ident else {
                                continue;
                            };
                            let rust_name = ident.unraw().to_string();
                            let options = serde_options(&field.attrs)?;
                            let spec_name = options.rename.unwrap_or_else(|| rust_name.clone());
                            let mut type_names = BTreeSet::new();
                            collect_type_names(&field.ty, &mut type_names);
                            fields.insert(
                                spec_name,
                                FieldInfo {
                                    rust_name,
                                    rust_type: TypeNode::from_syn(&field.ty),
                                    type_names,
                                    deprecated_marker: has_deprecated_cfg(&field.attrs)?,
                                    // A container-level `#[serde(default)]` fills every
                                    // missing field, so it marks each field individually —
                                    // the serde(default) ban cannot be dodged by moving
                                    // the attribute up to the struct.
                                    serde_default: options.default || container.default,
                                    skip_serializing_if: options.skip_serializing_if,
                                },
                            );
                        }
                    }
                    self.structs.insert(name, StructInfo { fields });
                }
                Item::Enum(item_enum) if matches!(item_enum.vis, Visibility::Public(_)) => {
                    let name = item_enum.ident.unraw().to_string();
                    self.model_types.insert(name.clone());
                    let container = serde_options(&item_enum.attrs)?;
                    let mut values = BTreeSet::new();
                    let mut variant_type_names = BTreeSet::new();
                    let mut is_value_enum = !container.untagged;
                    for variant in &item_enum.variants {
                        for field in variant.fields.iter() {
                            collect_type_names(&field.ty, &mut variant_type_names);
                        }
                        let options = serde_options(&variant.attrs)?;
                        if options.untagged || options.other {
                            continue;
                        }
                        if !matches!(variant.fields, Fields::Unit) {
                            is_value_enum = false;
                            continue;
                        }
                        let rust_name = variant.ident.unraw().to_string();
                        values.insert(options.rename.unwrap_or(rust_name));
                    }
                    self.enums.insert(
                        name,
                        EnumInfo {
                            values,
                            is_value_enum,
                            values_const: None,
                            variant_type_names,
                        },
                    );
                }
                Item::Type(item_type) if matches!(item_type.vis, Visibility::Public(_)) => {
                    let name = item_type.ident.unraw().to_string();
                    self.model_types.insert(name.clone());
                    let mut type_names = BTreeSet::new();
                    collect_type_names(&item_type.ty, &mut type_names);
                    self.alias_type_names.insert(name.clone(), type_names);
                    self.aliases.insert(name, TypeNode::from_syn(&item_type.ty));
                }
                _ => {}
            }
        }
        // Second pass: impl blocks may lexically precede their enum declaration.
        for item in items {
            let Item::Impl(item_impl) = item else {
                continue;
            };
            let Type::Path(self_type) = item_impl.self_ty.as_ref() else {
                continue;
            };
            let target = self_type
                .path
                .segments
                .last()
                .map(|segment| segment.ident.unraw().to_string());
            let Some(name) = target else {
                continue;
            };
            if item_impl
                .trait_
                .as_ref()
                .and_then(|(_, path, _)| path.segments.last())
                .is_some_and(|segment| segment.ident == "Default")
            {
                self.manual_default_impls.insert(name);
                continue;
            }
            if item_impl.trait_.is_some() {
                continue;
            }
            let Some(enum_info) = self.enums.get_mut(&name) else {
                continue;
            };
            for impl_item in &item_impl.items {
                let ImplItem::Const(const_item) = impl_item else {
                    continue;
                };
                if const_item.ident.unraw() == "VALUES" {
                    enum_info.values_const = Some(string_array(&const_item.expr));
                }
            }
        }
        Ok(())
    }

    fn collect_metadata(&mut self, items: &[Item]) {
        for item in items {
            let Item::Const(item_const) = item else {
                continue;
            };
            let name = item_const.ident.unraw().to_string();
            if name == "BETA_OPERATIONS" {
                self.metadata.beta_operations = string_array(&item_const.expr);
            } else if name == "DEPRECATED_FIELDS" {
                self.metadata.deprecated_fields = string_pair_array(&item_const.expr);
            }
        }
    }

    /// The set of model types transitively reachable from `Client` method
    /// return types, traversing struct fields, enum variant payloads, and
    /// type aliases. This is the "response tree": the types the library
    /// deserializes API responses into.
    pub(crate) fn response_reachable_types(&self) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<String> = self
            .client_methods
            .values()
            .flat_map(|method| method.return_type_names.iter())
            .filter(|name| self.model_types.contains(*name))
            .cloned()
            .collect();
        while let Some(name) = stack.pop() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let mut neighbours = BTreeSet::new();
            if let Some(struct_info) = self.structs.get(&name) {
                for field in struct_info.fields.values() {
                    neighbours.extend(field.type_names.iter().cloned());
                }
            }
            if let Some(enum_info) = self.enums.get(&name) {
                neighbours.extend(enum_info.variant_type_names.iter().cloned());
            }
            if let Some(alias_names) = self.alias_type_names.get(&name) {
                neighbours.extend(alias_names.iter().cloned());
            }
            for neighbour in neighbours {
                if self.model_types.contains(&neighbour) && !seen.contains(&neighbour) {
                    stack.push(neighbour);
                }
            }
        }
        seen
    }

    pub(crate) fn terminal_type(&self, ty: &TypeNode) -> Option<String> {
        self.resolve_terminal(ty, &mut BTreeSet::new())
    }

    fn resolve_terminal(&self, ty: &TypeNode, seen: &mut BTreeSet<String>) -> Option<String> {
        let terminal = ty.terminal()?.to_string();
        let Some(alias) = self.aliases.get(&terminal) else {
            return Some(terminal);
        };
        if !seen.insert(terminal.clone()) {
            return Some(terminal);
        }
        self.resolve_terminal(alias, seen)
    }

    pub(crate) fn array_item_type(&self, ty: &TypeNode) -> Option<String> {
        let item = self.resolve_array_item(ty, &mut BTreeSet::new())?;
        self.resolve_terminal(item, &mut BTreeSet::new())
    }

    fn resolve_array_item<'a>(
        &'a self,
        ty: &'a TypeNode,
        seen: &mut BTreeSet<String>,
    ) -> Option<&'a TypeNode> {
        match ty {
            TypeNode::Vec(inner) => Some(inner),
            TypeNode::Option(inner) | TypeNode::Boxed(inner) | TypeNode::Reference(inner) => {
                self.resolve_array_item(inner, seen)
            }
            TypeNode::Path(name) => {
                if !seen.insert(name.clone()) {
                    return None;
                }
                self.aliases
                    .get(name)
                    .and_then(|alias| self.resolve_array_item(alias, seen))
            }
            TypeNode::Other(_) => None,
        }
    }
}

impl RustInventory {
    /// Lists every public model struct field that carries a field-level
    /// `#[serde(default)]` (bare or `default = "path"`), as
    /// `StructName.rust_field_name`, sorted by struct name then wire field name.
    ///
    /// `cfg`-gated deprecated-marker fields are included, and a container-level
    /// `#[serde(default)]` reports every field of its struct — the ban cannot be
    /// dodged by moving the attribute up to the container.
    pub(crate) fn model_fields_with_serde_default(&self) -> Vec<String> {
        self.structs
            .iter()
            .flat_map(|(struct_name, info)| {
                info.fields
                    .values()
                    .filter(|field| field.serde_default)
                    .map(move |field| format!("{struct_name}.{}", field.rust_name))
            })
            .collect()
    }
}

/// Collects every named type appearing anywhere in `ty`, including inside
/// generic arguments of wrappers [`TypeNode`] does not model (`Result`, maps,
/// tuples). Used for reachability, where dropping a nested name would silently
/// shrink the response tree.
fn collect_type_names(ty: &Type, output: &mut BTreeSet<String>) {
    match ty {
        Type::Path(type_path) => {
            if let Some(segment) = type_path.path.segments.last() {
                output.insert(segment.ident.unraw().to_string());
            }
            for segment in &type_path.path.segments {
                if let PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let GenericArgument::Type(inner) = argument {
                            collect_type_names(inner, output);
                        }
                    }
                }
            }
        }
        Type::Reference(reference) => collect_type_names(&reference.elem, output),
        Type::Slice(slice) => collect_type_names(&slice.elem, output),
        Type::Array(array) => collect_type_names(&array.elem, output),
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_type_names(element, output);
            }
        }
        Type::Paren(paren) => collect_type_names(&paren.elem, output),
        Type::Group(group) => collect_type_names(&group.elem, output),
        _ => {}
    }
}

#[derive(Default)]
struct SerdeOptions {
    rename: Option<String>,
    untagged: bool,
    other: bool,
    default: bool,
    skip_serializing_if: bool,
}

fn serde_options(attributes: &[Attribute]) -> syn::Result<SerdeOptions> {
    let mut options = SerdeOptions::default();
    for attribute in attributes
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                if meta.input.peek(syn::Token![=]) {
                    options.rename = Some(meta.value()?.parse::<syn::LitStr>()?.value());
                } else {
                    meta.parse_nested_meta(|nested| {
                        if nested.path.is_ident("serialize") {
                            options.rename = Some(nested.value()?.parse::<syn::LitStr>()?.value());
                        } else if nested.input.peek(syn::Token![=]) {
                            let _: Expr = nested.value()?.parse()?;
                        }
                        Ok(())
                    })?;
                }
            } else if meta.path.is_ident("rename_all") {
                return Err(meta.error(
                    "rename_all is not allowed in models.rs: wire names must be explicit \
                     #[serde(rename = \"...\")] literals so the drift analyzer can read them \
                     verbatim (see AGENTS.md, OpenAPI drift section)",
                ));
            } else if meta.path.is_ident("untagged") {
                options.untagged = true;
            } else if meta.path.is_ident("other") {
                options.other = true;
            } else if meta.path.is_ident("default") {
                // Both `default` and `default = "path"` opt the field into value
                // fabrication, so this must precede the generic `key = value` arm below.
                options.default = true;
                if meta.input.peek(syn::Token![=]) {
                    let _: Expr = meta.value()?.parse()?;
                }
            } else if meta.path.is_ident("skip_serializing_if") {
                options.skip_serializing_if = true;
                let _: Expr = meta.value()?.parse()?;
            } else if meta.input.peek(syn::Token![=]) {
                let _: Expr = meta.value()?.parse()?;
            }
            Ok(())
        })?;
    }
    Ok(options)
}

fn has_deprecated_cfg(attributes: &[Attribute]) -> syn::Result<bool> {
    let mut found = false;
    for attribute in attributes.iter().filter(|attr| attr.path().is_ident("cfg")) {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("feature") {
                let value = meta.value()?.parse::<syn::LitStr>()?.value();
                if value == "deprecated-fields" {
                    found = true;
                }
            } else if meta.input.peek(syn::token::Paren) {
                meta.parse_nested_meta(|nested| {
                    if nested.path.is_ident("feature") {
                        let value = nested.value()?.parse::<syn::LitStr>()?.value();
                        if value == "deprecated-fields" {
                            found = true;
                        }
                    }
                    Ok(())
                })?;
            }
            Ok(())
        })?;
    }
    Ok(found)
}

fn dereference(expression: &Expr) -> &Expr {
    if let Expr::Reference(reference) = expression {
        &reference.expr
    } else {
        expression
    }
}

fn string_array(expression: &Expr) -> BTreeSet<String> {
    let Expr::Array(array) = dereference(expression) else {
        return BTreeSet::new();
    };
    array
        .elems
        .iter()
        .filter_map(|expr| match expr {
            Expr::Lit(literal) => match &literal.lit {
                Lit::Str(value) => Some(value.value()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn string_pair_array(expression: &Expr) -> BTreeSet<(String, String)> {
    let Expr::Array(array) = dereference(expression) else {
        return BTreeSet::new();
    };
    array
        .elems
        .iter()
        .filter_map(|expr| {
            let Expr::Tuple(tuple) = expr else {
                return None;
            };
            let mut values = tuple.elems.iter().filter_map(|item| match item {
                Expr::Lit(literal) => match &literal.lit {
                    Lit::Str(value) => Some(value.value()),
                    _ => None,
                },
                _ => None,
            });
            Some((values.next()?, values.next()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module_tree_fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("module_tree")
            .join(name)
    }

    #[test]
    fn private_nested_module_tree_matches_flat_inventory() {
        let flat = RustInventory::load(&module_tree_fixture("flat")).unwrap();
        let nested = RustInventory::load(&module_tree_fixture("nested")).unwrap();

        assert_eq!(nested, flat);
        assert!(nested.client_methods.contains_key("list_widgets"));
        assert_eq!(
            nested.model_types,
            BTreeSet::from([
                "Widget".to_string(),
                "WidgetAlias".to_string(),
                "WidgetLeaf".to_string(),
                "WidgetState".to_string(),
            ])
        );
        assert_eq!(
            nested.enums["WidgetState"].values_const,
            Some(BTreeSet::from(["ready".to_string()]))
        );
        assert_eq!(
            nested.manual_default_impls,
            BTreeSet::from(["WidgetState".to_string()])
        );
        assert_eq!(
            nested.model_fields_with_serde_default(),
            vec!["Widget.item_count".to_string()]
        );
        assert_eq!(
            nested.terminal_type(&nested.structs["Widget"].fields["itemCount"].rust_type),
            Some("f64".to_string())
        );
    }

    #[test]
    fn path_attributes_follow_rust_inline_module_context() {
        let inventory = RustInventory::load(&module_tree_fixture("path_context")).unwrap();

        assert_eq!(
            inventory.model_types,
            BTreeSet::from([
                "DirectPathModel".to_string(),
                "InlinePathModel".to_string(),
                "RelocatedInlinePathModel".to_string(),
            ])
        );
    }

    #[test]
    fn complementary_unknown_cfg_paths_are_all_inventoried() {
        let inventory = RustInventory::load(&module_tree_fixture("unknown_cfg_paths")).unwrap();

        assert_eq!(
            inventory.model_types,
            BTreeSet::from([
                "CustomDisabledModel".to_string(),
                "CustomEnabledModel".to_string(),
            ])
        );
    }

    #[test]
    fn inactive_cfg_modules_are_not_loaded_or_inventoried() {
        let inventory = RustInventory::load(&module_tree_fixture("cfg_modules")).unwrap();

        assert_eq!(
            inventory.model_types,
            BTreeSet::from([
                "CustomCfgModel".to_string(),
                "DeprecatedModel".to_string(),
                "FeatureModel".to_string(),
                "PlatformModel".to_string(),
                "ProductionModel".to_string(),
            ])
        );
        assert_eq!(
            inventory.client_methods.keys().collect::<Vec<_>>(),
            vec!["production_operation"]
        );
        assert_eq!(
            inventory.metadata.beta_operations,
            BTreeSet::from(["production_operation".to_string()])
        );
    }

    #[test]
    fn cfg_evaluation_matches_the_analyzer_target() {
        fn value(source: &str) -> CfgValue {
            evaluate_cfg(&syn::parse_str(source).unwrap()).unwrap()
        }

        fn assert_target_value(name: &str, actual: &str) {
            assert_eq!(value(&format!(r#"{name} = "{actual}""#)), CfgValue::True);
            assert_eq!(
                value(&format!(r#"{name} = "definitely-not-{actual}""#)),
                CfgValue::False
            );
        }

        assert_eq!(value("unix"), CfgValue::from(cfg!(unix)));
        assert_eq!(value("windows"), CfgValue::from(cfg!(windows)));
        assert_target_value("target_arch", std::env::consts::ARCH);
        assert_target_value("target_os", std::env::consts::OS);
        assert_target_value("target_family", std::env::consts::FAMILY);
        assert_target_value("target_env", env!("ANALYZER_TARGET_ENV"));
        assert_target_value("target_vendor", env!("ANALYZER_TARGET_VENDOR"));
        assert_target_value(
            "target_endian",
            if cfg!(target_endian = "little") {
                "little"
            } else {
                "big"
            },
        );
        assert_target_value("target_pointer_width", &usize::BITS.to_string());
        assert_eq!(value(r#"feature = "any-feature""#), CfgValue::True);
        assert_eq!(value("clickhouse_custom"), CfgValue::Unknown);
    }

    #[test]
    fn inventories_structural_rust_and_serde_details() {
        let client = r#"
            pub struct Client;
            impl Client {
                pub async fn list_widgets(&self, sort_by: Option<&WidgetSort>) {}
                fn private(&self) {}
            }
        "#;
        let models = r#"
            pub struct Widget {
                pub r#type: Option<Vec<Box<WidgetType>>>,
                #[serde(rename = "legacyName")]
                #[cfg(feature = "deprecated-fields")]
                pub old_name: String,
            }
            pub enum WidgetType {
                #[serde(rename = "ready-now")]
                Ready,
                Unknown,
                #[serde(untagged)]
                Other(String),
            }
            #[serde(untagged)]
            pub enum Union { Text(String), Count(i64) }
            pub type WidgetAlias = Option<Vec<Box<WidgetType>>>;
            pub struct Aliased { pub values: WidgetAlias }
        "#;
        let meta = r#"
            pub const BETA_OPERATIONS: &[&str] = &["list_widgets"];
            pub const DEPRECATED_FIELDS: &[(&str, &str)] = &[("Widget", "legacyName")];
        "#;

        let inventory = RustInventory::parse(client, models, meta).unwrap();
        assert!(inventory.client_methods.contains_key("list_widgets"));
        let fields = &inventory.structs["Widget"].fields;
        assert!(fields.contains_key("type"));
        assert!(fields["legacyName"].deprecated_marker);
        assert_eq!(
            fields["type"].rust_type.display(),
            "Option<Vec<Box<WidgetType>>>"
        );
        assert_eq!(
            inventory.enums["WidgetType"].values,
            BTreeSet::from(["Unknown".to_string(), "ready-now".to_string(),])
        );
        assert!(inventory.enums["WidgetType"].is_value_enum);
        assert!(!inventory.enums["Union"].is_value_enum);
        assert_eq!(
            inventory.array_item_type(&inventory.structs["Aliased"].fields["values"].rust_type),
            Some("WidgetType".to_string())
        );
        assert!(inventory.metadata.beta_operations.contains("list_widgets"));
    }

    #[test]
    fn response_reachability_walks_returns_fields_variants_and_aliases() {
        let client = r#"
            pub struct Client;
            impl Client {
                pub async fn get_widget(&self) -> Result<ApiResponse<Vec<Widget>>, Error> {
                    unimplemented!()
                }
                pub async fn mutate_widget(&self, body: WidgetPostRequest) {}
            }
        "#;
        let models = r#"
            pub struct ApiResponse<T> { pub result: Option<T> }
            pub struct Widget {
                pub union: Option<WidgetUnion>,
                pub rows: WidgetRows,
                pub map: std::collections::BTreeMap<String, MapValue>,
            }
            pub enum WidgetUnion {
                Known(WidgetVariant),
                #[serde(untagged)]
                Unknown(serde_json::Value),
            }
            pub struct WidgetVariant { pub leaf: Option<String> }
            pub type WidgetRows = Vec<WidgetRow>;
            pub struct WidgetRow { pub cell: Option<String> }
            pub struct MapValue { pub value: Option<String> }
            pub struct WidgetPostRequest { pub name: String }
            pub struct Unrelated { pub other: String }
        "#;
        let inventory = RustInventory::parse(client, models, "").unwrap();
        assert_eq!(
            inventory.client_methods["get_widget"].return_type_names,
            BTreeSet::from([
                "Result".to_string(),
                "ApiResponse".to_string(),
                "Vec".to_string(),
                "Widget".to_string(),
                "Error".to_string(),
            ])
        );
        assert_eq!(
            inventory.response_reachable_types(),
            BTreeSet::from([
                "ApiResponse".to_string(),
                "Widget".to_string(),
                "WidgetUnion".to_string(),
                "WidgetVariant".to_string(),
                "WidgetRows".to_string(),
                "WidgetRow".to_string(),
                "MapValue".to_string(),
            ]),
            "request-only and unrelated types must stay out of the response tree"
        );
    }

    #[test]
    fn tracks_serde_default_and_skip_serializing_if_in_all_attribute_forms() {
        let models = r#"
            pub struct Widget {
                #[serde(default)]
                pub bare: String,
                #[serde(rename = "renamedWithDefault", default)]
                pub renamed: String,
                #[serde(default = "some::path")]
                pub with_path: String,
                #[serde(skip_serializing_if = "Option::is_none")]
                pub skipped: Option<String>,
                pub plain: String,
            }
        "#;

        let inventory = RustInventory::parse("", models, "").unwrap();
        let fields = &inventory.structs["Widget"].fields;
        assert!(fields["bare"].serde_default);
        assert!(fields["renamedWithDefault"].serde_default);
        assert!(fields["with_path"].serde_default);
        assert!(!fields["plain"].serde_default);
        assert!(fields["skipped"].skip_serializing_if);
        assert!(!fields["plain"].skip_serializing_if);
    }

    #[test]
    fn model_fields_with_serde_default_lists_carriers_sorted() {
        let models = r#"
            pub struct Widget {
                #[serde(default)]
                pub name: String,
                pub description: Option<String>,
                #[serde(rename = "createdAt", default)]
                pub created_at: String,
                #[cfg(feature = "deprecated-fields")]
                #[serde(rename = "legacyName", default)]
                pub legacy_name: Option<String>,
            }
            pub struct Gadget {
                pub id: Option<String>,
            }
            pub enum State { Ready }
        "#;

        assert_eq!(
            RustInventory::parse("", models, "")
                .unwrap()
                .model_fields_with_serde_default(),
            vec![
                "Widget.created_at".to_string(),
                "Widget.legacy_name".to_string(),
                "Widget.name".to_string(),
            ]
        );
    }

    #[test]
    fn model_types_with_manual_default_impl_lists_only_hand_written_impls() {
        let models = r#"
            #[derive(Default)]
            pub struct Derived { pub id: Option<String> }
            pub enum Union { A(Widget), Unknown(serde_json::Value) }
            impl Default for Union {
                fn default() -> Self { Self::A(Widget::default()) }
            }
            pub struct Widget;
            impl Default for Widget {
                fn default() -> Self { Self }
            }
            impl std::fmt::Display for Union {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { Ok(()) }
            }
        "#;

        assert_eq!(
            RustInventory::parse("", models, "")
                .unwrap()
                .manual_default_impls,
            BTreeSet::from(["Union".to_string(), "Widget".to_string()])
        );
    }

    #[test]
    fn container_level_serde_default_marks_every_field() {
        let models = r#"
            #[serde(default)]
            pub struct Widget {
                pub name: String,
                pub description: Option<String>,
            }
        "#;

        assert_eq!(
            RustInventory::parse("", models, "")
                .unwrap()
                .model_fields_with_serde_default(),
            vec!["Widget.description".to_string(), "Widget.name".to_string()]
        );
    }

    #[test]
    fn serde_other_is_not_a_wire_value() {
        let models = r#"
            pub enum State { Ready, #[serde(other)] Unknown }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert_eq!(
            inventory.enums["State"].values,
            BTreeSet::from(["Ready".to_string()])
        );
    }

    #[test]
    fn inventories_values_const_from_impl_block() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(untagged)]
                Unknown(String),
            }
            impl Color {
                pub const VALUES: &'static [&'static str] = &["red", "blue"];
            }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert_eq!(
            inventory.enums["Color"].values_const,
            Some(BTreeSet::from(["red".to_string(), "blue".to_string()]))
        );
    }

    #[test]
    fn values_const_absent_when_not_declared() {
        let models = r#"
            pub enum State { Ready, #[serde(other)] Unknown }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert!(inventory.enums["State"].values_const.is_none());
    }

    #[test]
    fn values_const_ignored_for_non_enum_impl() {
        let models = r#"
            pub struct Widget { pub name: String }
            impl Widget {
                pub const VALUES: &'static [&'static str] = &["a"];
            }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert!(inventory.structs.contains_key("Widget"));
        assert!(inventory.enums.values().all(|e| e.values_const.is_none()));
    }

    #[test]
    fn inventories_values_const_when_impl_precedes_enum() {
        let models = r#"
            impl Color {
                pub const VALUES: &'static [&'static str] = &["red", "blue"];
            }
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(rename = "blue")]
                Blue,
                #[serde(untagged)]
                Unknown(String),
            }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert_eq!(
            inventory.enums["Color"].values_const,
            Some(BTreeSet::from(["red".to_string(), "blue".to_string()]))
        );
    }

    #[test]
    fn values_const_in_trait_impl_is_ignored() {
        let models = r#"
            pub enum Color {
                #[serde(rename = "red")]
                Red,
                #[serde(other)]
                Unknown,
            }
            impl Palette for Color {
                const VALUES: &'static [&'static str] = &["stale"];
            }
        "#;
        let inventory = RustInventory::parse("", models, "").unwrap();
        assert!(inventory.enums["Color"].values_const.is_none());
    }

    #[test]
    fn rejects_rename_all_on_struct() {
        let models = r#"
            #[serde(rename_all = "camelCase")]
            pub struct Widget { pub some_field: String }
        "#;
        let error = RustInventory::parse("", models, "").unwrap_err();
        assert!(
            error.to_string().contains("rename_all"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_rename_all_on_enum() {
        let models = r#"
            #[serde(rename_all = "snake_case")]
            pub enum State { ReadyNow, InProgress }
        "#;
        let error = RustInventory::parse("", models, "").unwrap_err();
        assert!(
            error.to_string().contains("rename_all"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn rejects_rename_all_serialize_form() {
        let models = r#"
            #[serde(rename_all(serialize = "kebab-case"))]
            pub struct Widget { pub some_field: String }
        "#;
        let error = RustInventory::parse("", models, "").unwrap_err();
        assert!(
            error.to_string().contains("rename_all"),
            "unexpected error: {error}"
        );
    }
}
