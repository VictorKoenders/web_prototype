use proc_macro2::Span;
use std::fmt::Write;
use syn::{
    spanned::Spanned, Attribute, DataStruct, Ident, Lit, Meta, MetaList, MetaNameValue, NestedMeta,
};

pub fn parse(input: DataStruct) -> Result<Vec<Field>, (String, Span)> {
    let mut result = Vec::new();
    for field in input.fields {
        let ident = field.ident.expect("Nameless structs not supported");
        let attributes = FieldAttributes::parse(&field.attrs)?;

        if attributes.is_table {
            result.push(Field::Table(TableField {
                field: ident,
                actions: attributes.actions,
                columns: attributes.table_columns,
            }));
        } else {
            result.push(Field::Label(LabelField {
                field: ident,
                label: None,
            }));
        }
    }
    Ok(result)
}

#[derive(Default)]
struct FieldAttributes {
    is_table: bool,
    table_columns: Vec<TableColumn>,
    actions: Vec<Action>,
}

impl FieldAttributes {
    fn parse(attributes: &[Attribute]) -> Result<Self, (String, Span)> {
        let mut result = Self::default();

        for attribute in attributes.iter().filter_map(|a| a.parse_meta().ok()) {
            match attribute {
                Meta::List(meta) => {
                    let path = meta.path.get_ident().map(ToString::to_string);
                    match path.as_deref() {
                        Some("column") => {
                            result.parse_column(meta)?;
                        }
                        _ => {
                            continue;
                        }
                    }
                }
                Meta::Path(path) => {
                    let path = path.get_ident().map(ToString::to_string);
                    match path.as_deref() {
                        Some("table") => {
                            result.is_table = true;
                        }
                        _ => {
                            return Err(("Unknown path label".to_string(), path.span()));
                        }
                    }
                }
                Meta::NameValue(path) => unimplemented!("name_value: {:?}", path),
            };
        }

        Ok(result)
    }

    fn parse_column(&mut self, meta: MetaList) -> Result<(), (String, Span)> {
        let mut field = None;
        let mut header = None;
        for item in &meta.nested {
            if let NestedMeta::Meta(Meta::NameValue(MetaNameValue { path, lit, .. })) = item {
                if let Some(ident) = path.get_ident() {
                    let ident_string = ident.to_string();
                    match (ident_string.as_str(), lit) {
                        ("field", Lit::Str(str)) => {
                            if field.is_some() {
                                return Err((
                                    "Duplicate field attribute".to_string(),
                                    ident.span(),
                                ));
                            }
                            field = Some(str.value());
                        }
                        ("header", Lit::Str(str)) => {
                            if header.is_some() {
                                return Err((
                                    "Duplicate header attribute".to_string(),
                                    ident.span(),
                                ));
                            }
                            header = Some(str.value());
                        }
                        _ => {
                            return Err(("Unknown attribute".to_string(), ident.span()));
                        }
                    }
                }
            }
        }
        if let Some(field) = field {
            self.table_columns.push(TableColumn { field, header });
            Ok(())
        } else {
            Err(("Missing 'field = \"...\"'".to_string(), meta.span()))
        }
    }
}

pub enum Field {
    Label(LabelField),
    Table(TableField),
}

impl Field {
    pub fn write_html(&self, out: &mut impl Write) {
        match self {
            Self::Label(inner) => inner.write_html(out),
            Self::Table(inner) => inner.write_html(out),
        }
    }
    pub fn write_javascript(&self, out: &mut impl Write) {
        match self {
            Self::Label(inner) => inner.write_javascript(out),
            Self::Table(inner) => inner.write_javascript(out),
        }
    }
}

pub struct LabelField {
    label: Option<String>,
    field: Ident,
}
impl LabelField {
    pub fn write_html(&self, out: &mut impl Write) {
        let _ = write!(
            out,
            "{}: <label data-bind=\"text: {}\"></label>",
            self.label.clone().unwrap_or_else(|| self.field.to_string()),
            self.field
        );
    }
    pub fn write_javascript(&self, _out: &mut impl Write) {}
}

pub struct TableField {
    columns: Vec<TableColumn>,
    actions: Vec<Action>,
    field: Ident,
}

impl TableField {
    pub fn write_html(&self, out: &mut impl Write) {
        let _ = write!(out, "<table><thead><tr>");
        for column in &self.columns {
            let _ = write!(
                out,
                "<th>{}</th>",
                column.header.as_ref().unwrap_or(&column.field)
            );
        }
        if !self.actions.is_empty() {
            let _ = write!(out, "<th></th>");
        }
        let _ = write!(out, "</tr></thead>");
        let _ = write!(out, "<tbody data-bind=\"foreach: {}\">", self.field);
        let _ = write!(out, "<tr>");
        for column in &self.columns {
            let _ = write!(out, "<td data-bind=\"text: {}\"></td>", column.field);

            if !self.actions.is_empty() {
                let _ = write!(out, "<td>");
                for action in &self.actions {
                    let _ = write!(out, "<a href='#'>{}</a>", action.name);
                }
                let _ = write!(out, "</td>");
            }
        }
        let _ = write!(out, "</tr></tbody></table>");
    }
    pub fn write_javascript(&self, _out: &mut impl Write) {}
}

pub struct Action {
    pub name: String,
    pub action: String,
}

pub struct TableColumn {
    pub field: String,
    pub header: Option<String>,
}
