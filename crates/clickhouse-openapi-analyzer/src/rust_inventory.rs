use std::collections::{BTreeMap, BTreeSet};

use quote::ToTokens;
use syn::ext::IdentExt;
use syn::{
    Attribute, Expr, Fields, FnArg, GenericArgument, ImplItem, Item, Lit, Pat, PathArguments, Type,
    Visibility,
};

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

#[derive(Debug, Clone)]
pub(crate) struct FieldInfo {
    pub(crate) rust_name: String,
    pub(crate) rust_type: TypeNode,
    pub(crate) deprecated_marker: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StructInfo {
    pub(crate) fields: BTreeMap<String, FieldInfo>,
}

#[derive(Debug, Clone)]
pub(crate) struct EnumInfo {
    pub(crate) values: BTreeSet<String>,
    pub(crate) is_value_enum: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodInfo {
    pub(crate) arguments: BTreeMap<String, TypeNode>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MetadataInventory {
    pub(crate) beta_operations: BTreeSet<String>,
    pub(crate) deprecated_fields: BTreeSet<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RustInventory {
    pub(crate) client_methods: BTreeMap<String, MethodInfo>,
    pub(crate) model_types: BTreeSet<String>,
    pub(crate) structs: BTreeMap<String, StructInfo>,
    pub(crate) enums: BTreeMap<String, EnumInfo>,
    pub(crate) aliases: BTreeMap<String, TypeNode>,
    pub(crate) metadata: MetadataInventory,
}

impl RustInventory {
    pub(crate) fn parse(client: &str, models: &str, meta: &str) -> syn::Result<Self> {
        let client_file = syn::parse_file(client)?;
        let models_file = syn::parse_file(models)?;
        let meta_file = syn::parse_file(meta)?;

        let mut inventory = Self::default();
        inventory.collect_client(&client_file)?;
        inventory.collect_models(&models_file)?;
        inventory.collect_metadata(&meta_file);
        Ok(inventory)
    }

    fn collect_client(&mut self, file: &syn::File) -> syn::Result<()> {
        for item in &file.items {
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
                self.client_methods.insert(
                    function.sig.ident.unraw().to_string(),
                    MethodInfo { arguments },
                );
            }
        }
        Ok(())
    }

    fn collect_models(&mut self, file: &syn::File) -> syn::Result<()> {
        for item in &file.items {
            match item {
                Item::Struct(item_struct) if matches!(item_struct.vis, Visibility::Public(_)) => {
                    let name = item_struct.ident.unraw().to_string();
                    self.model_types.insert(name.clone());
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
                            let spec_name = options.rename.unwrap_or_else(|| {
                                apply_rename_rule(&rust_name, container.rename_all.as_deref())
                            });
                            fields.insert(
                                spec_name,
                                FieldInfo {
                                    rust_name,
                                    rust_type: TypeNode::from_syn(&field.ty),
                                    deprecated_marker: has_deprecated_cfg(&field.attrs)?,
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
                    let mut is_value_enum = !container.untagged;
                    for variant in &item_enum.variants {
                        let options = serde_options(&variant.attrs)?;
                        if options.untagged || options.other {
                            continue;
                        }
                        if !matches!(variant.fields, Fields::Unit) {
                            is_value_enum = false;
                            continue;
                        }
                        let rust_name = variant.ident.unraw().to_string();
                        values.insert(options.rename.unwrap_or_else(|| {
                            apply_rename_rule(&rust_name, container.rename_all.as_deref())
                        }));
                    }
                    self.enums.insert(
                        name,
                        EnumInfo {
                            values,
                            is_value_enum,
                        },
                    );
                }
                Item::Type(item_type) if matches!(item_type.vis, Visibility::Public(_)) => {
                    let name = item_type.ident.unraw().to_string();
                    self.model_types.insert(name.clone());
                    self.aliases.insert(name, TypeNode::from_syn(&item_type.ty));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_metadata(&mut self, file: &syn::File) {
        for item in &file.items {
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

#[derive(Default)]
struct SerdeOptions {
    rename: Option<String>,
    rename_all: Option<String>,
    untagged: bool,
    other: bool,
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
                options.rename_all = Some(meta.value()?.parse::<syn::LitStr>()?.value());
            } else if meta.path.is_ident("untagged") {
                options.untagged = true;
            } else if meta.path.is_ident("other") {
                options.other = true;
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

fn apply_rename_rule(name: &str, rule: Option<&str>) -> String {
    match rule {
        None => name.to_string(),
        Some("lowercase") => name.to_ascii_lowercase(),
        Some("UPPERCASE") => name.to_ascii_uppercase(),
        Some("snake_case") => words(name).join("_"),
        Some("SCREAMING_SNAKE_CASE") => words(name).join("_").to_ascii_uppercase(),
        Some("kebab-case") => words(name).join("-"),
        Some("SCREAMING-KEBAB-CASE") => words(name).join("-").to_ascii_uppercase(),
        Some("PascalCase") => words(name).into_iter().map(capitalize).collect(),
        Some("camelCase") => {
            let mut parts = words(name).into_iter();
            let first = parts.next().unwrap_or_default();
            first + &parts.map(capitalize).collect::<String>()
        }
        Some(_) => name.to_string(),
    }
}

fn words(value: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut current = String::new();
    for (index, character) in value.chars().enumerate() {
        let boundary = character == '_' || character == '-';
        let uppercase_boundary = character.is_ascii_uppercase()
            && index > 0
            && current
                .chars()
                .last()
                .is_some_and(|last| last.is_ascii_lowercase());
        if boundary || uppercase_boundary {
            if !current.is_empty() {
                output.push(current.to_ascii_lowercase());
                current.clear();
            }
            if boundary {
                continue;
            }
        }
        current.push(character);
    }
    if !current.is_empty() {
        output.push(current.to_ascii_lowercase());
    }
    output
}

fn capitalize(value: String) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
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
            #[serde(rename_all = "camelCase")]
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
}
