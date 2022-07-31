mod field;

use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::fmt::Write;
use std::time::Duration;
use syn::{Attribute, Data, DeriveInput, Ident, Lit, Meta, MetaNameValue, NestedMeta};

pub fn generate(input: DeriveInput) -> Result<TokenStream, (String, Span)> {
    let ident = input.ident;
    let attributes = StructAttributes::parse(&ident, &input.attrs)?;
    let fields = if let Data::Struct(input) = input.data {
        field::parse(input)?
    } else {
        return Err(("Only structs are supported".to_string(), ident.span()));
    };

    let url = attributes.url;
    let mut html = format!("<div id=\"page_{}\" data-bind=\"with: model\">", ident);
    let mut javascript = String::new();
    for field in fields {
        field.write_html(&mut html);
        field.write_javascript(&mut javascript);
    }
    html += "</div><script type='text/javascript'>";
    html += &javascript;
    let _ = write!(&mut html, "load('page_{}', {{}});", ident);
    if let Some(refresh) = attributes.refresh {
        let _ = write!(&mut html, "enable_auto_reload({});", refresh.as_millis());
    }
    html += "</script>";

    Ok(quote! {
        impl framework::Page for #ident {
            const URL: &'static str = #url;

            fn html(self) -> String {
                format!(#html, serde_json::to_string(&self).unwrap())
            }
        }
    })
}

#[derive(Default)]
struct StructAttributes {
    url: String,
    refresh: Option<Duration>,
}

impl StructAttributes {
    fn parse(ident: &Ident, attributes: &[Attribute]) -> Result<Self, (String, Span)> {
        let mut result = Self::default();

        for attribute in attributes.iter().filter_map(|a| a.parse_meta().ok()) {
            let meta = match attribute {
                Meta::List(list) => list,
                _ => continue,
            };
            if meta.path.get_ident().map(ToString::to_string).as_deref() != Some("page") {
                continue;
            }

            for item in meta.nested {
                if let NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit, .. })) = item {
                    if let Some(ident) = path.get_ident() {
                        let ident_string = ident.to_string();
                        match (ident_string.as_str(), lit) {
                            ("path", Lit::Str(str)) => result.url = str.value(),
                            ("refresh", Lit::Str(str)) => {
                                if let Some(dt) = parse_datetime(str.value()) {
                                    result.refresh = Some(dt);
                                } else {
                                    return Err((String::from("Invalid duration"), str.span()));
                                }
                            }
                            (_, _) => return Err(("Unknown attribute".to_string(), ident.span())),
                        }
                    }
                }
            }
        }

        if result.url.is_empty() {
            Err((
                "Missing attribute #[page(path = \"...\")]".to_string(),
                ident.span(),
            ))
        } else {
            Ok(result)
        }
    }
}

fn parse_datetime(str: String) -> Option<Duration> {
    if let Some(ms) = str.strip_suffix("ms") {
        ms.parse().map(Duration::from_millis).ok()
    } else if let Some(sec) = str.strip_suffix('s') {
        sec.parse().map(Duration::from_secs).ok()
    } else {
        None
    }
}
