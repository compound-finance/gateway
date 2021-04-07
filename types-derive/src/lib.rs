extern crate proc_macro;
use lazy_static::lazy_static;
use proc_macro::TokenStream;
use serde_json;
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::Write;
use std::io::{Seek, SeekFrom};
use std::sync::Mutex;

lazy_static! {
    static ref TYPES: Mutex<Vec<(String, serde_json::Value)>> = Mutex::new(vec![]);
    static ref TYPES_JSON: Mutex<File> = Mutex::new(
        File::create(env::var("TYPES_FILE").unwrap_or(String::from("/tmp/types.json")))
            .expect("Unable to create file")
    );
}

#[proc_macro_attribute]
pub fn type_alias(attr: TokenStream, item: TokenStream) -> TokenStream {
    if let Ok(_) = env::var("TYPES_FILE") {
        let prefix: Option<syn::LitStr> = syn::parse(attr).ok();
        let ast: syn::ItemType = syn::parse(item.clone()).expect("was parsing...");

        let mut new_types = TYPES.lock().unwrap();

        new_types.push((
            format!(
                "{}{}",
                prefix.map(|l| l.value()).unwrap_or(String::from("")),
                ast.ident
            ),
            type_to_json(&*ast.ty),
        ));

        write_types(new_types.clone());
    }

    item
}

#[proc_macro_derive(Types)]
pub fn derive_types(input: TokenStream) -> TokenStream {
    if let Ok(_) = env::var("TYPES_FILE") {
        let ast = syn::parse(input).unwrap();

        derive_and_write_types(&ast);
    }

    TokenStream::new()
}

fn write_types(new_types: Vec<(String, serde_json::Value)>) {
    let data = serde_json::to_string(&json!(new_types
        .clone()
        .into_iter()
        .collect::<serde_json::Map<_, _>>()))
    .expect("unable to serialize json");

    let mut types_json = TYPES_JSON.lock().unwrap();

    types_json
        .set_len(0)
        .expect("Unable to truncate types.json");

    types_json
        .seek(SeekFrom::Start(0))
        .expect("Unable to seek to file start");

    types_json
        .write_all(data.as_bytes())
        .expect("Unable to write data");
}

fn show_arg(arg: &syn::GenericArgument) -> String {
    match arg {
        syn::GenericArgument::Type(ty) => type_to_str(ty),
        _ => String::from("unknown arg"),
    }
}

fn show_segment(seg: &syn::PathSegment) -> String {
    let args = match &seg.arguments {
        syn::PathArguments::AngleBracketed(bracket_args) => {
            let inner = bracket_args
                .args
                .iter()
                .map(show_arg)
                .collect::<Vec<_>>()
                .join(",");

            format!("<{}>", inner)
        }
        _ => format!(""),
    };

    format!("{}{}", seg.ident, args)
}

fn show_expr(e: &syn::Expr) -> String {
    match e {
        syn::Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => i.to_string(),
            _ => String::from("unknown lit"),
        },
        _ => String::from("unknown expr"),
    }
}

fn type_to_str(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => {
            let prefix = match &p.qself {
                Some(qself) => format!("{}__", type_to_str(&*qself.ty)),
                None => String::from(""),
            };

            let postfix = if p.path.segments.len() == 1 {
                show_segment(p.path.segments.iter().next().unwrap())
            } else {
                p.path
                    .segments
                    .iter()
                    .map(show_segment)
                    .collect::<Vec<_>>()
                    .join("__")
            };

            format!("{}{}", prefix, postfix)
        }
        syn::Type::Array(a) => {
            let elem = type_to_str(&*a.elem);

            format!("[{}; {}]", elem, show_expr(&a.len))
        }
        syn::Type::Tuple(tuple) => {
            let inner = tuple
                .elems
                .iter()
                .map(|ty_| type_to_str(ty_))
                .collect::<Vec<_>>()
                .join(",");
            format!("({})", inner)
        }
        _ => String::from("unknown type"),
    }
}

fn type_to_json(ty: &syn::Type) -> serde_json::Value {
    json!(type_to_str(ty))
}

fn process_fields(
    new_types: &mut Vec<(String, serde_json::Value)>,
    prefix_opt: Option<String>,
    fields: &syn::Fields,
) -> serde_json::Value {
    match fields {
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                type_to_json(&fields.unnamed.iter().next().unwrap().ty)
            } else {
                let ty_fields = fields
                    .unnamed
                    .iter()
                    .map(|field| type_to_str(&field.ty))
                    .collect::<Vec<_>>()
                    .join(",");

                let ty = json!(format!("({})", ty_fields));

                match prefix_opt {
                    Some(prefix) => {
                        let new_type_name = format!("{}", prefix);
                        new_types.push((new_type_name.clone(), ty));
                        json!(new_type_name)
                    }
                    None => ty,
                }
            }
        }
        syn::Fields::Named(fields) => {
            let ty = json!(fields
                .named
                .iter()
                .map(|field| (
                    field.ident.clone().unwrap().to_string(),
                    type_to_json(&field.ty)
                ))
                .collect::<serde_json::Map<_, _>>());
            match prefix_opt {
                Some(prefix) => {
                    let new_type_name = format!("{}", prefix);
                    new_types.push((new_type_name.clone(), ty));
                    json!(new_type_name)
                }
                None => json!(ty),
            }
        }
        syn::Fields::Unit => {
            json!("")
        }
    }
}

fn merge_ident(a: &syn::Ident, b: &syn::Ident) -> String {
    format!("{}{}", a.to_string(), b.to_string())
}

fn derive_and_write_types(ast: &syn::DeriveInput) {
    let name = &ast.ident;

    let mut new_types = TYPES.lock().unwrap();

    let inner: serde_json::Value = match &ast.data {
        syn::Data::Struct(data_struct) => process_fields(&mut new_types, None, &data_struct.fields),
        syn::Data::Enum(data_enum) => json!({"_enum": data_enum
            .variants
            .iter()
            .map(|variant| (variant.ident.to_string(), process_fields(&mut new_types, Some(merge_ident(&ast.ident, &variant.ident)), &variant.fields)))
            .collect::<serde_json::Map<_, _>>()}),
        _ => json!("not implemented"),
    };

    new_types.push((name.to_string(), inner));

    write_types(new_types.clone());
}
