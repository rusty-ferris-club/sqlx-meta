use inflector::Inflector;
use proc_macro::{self, TokenStream};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident,
    Lit, LitStr, Meta, MetaNameValue,
};

/// Derive metadata for the entity
///
/// # Panics
///
/// Panics if conversions fail
#[proc_macro_derive(SqlxMeta, attributes(database, external_id, id))]
pub fn sqlx_meta(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident, data, attrs, ..
    } = parse_macro_input!(input);
    match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => {
            let config = Config::new(&attrs, &ident, &named);
            let static_model_schema = build_static_model_schema(&config);
            let sqlx_crud_impl = build_sqlx_crud_impl(&config);

            quote! {
                #static_model_schema
                #sqlx_crud_impl
            }
            .into()
        }
        _ => panic!("this derive macro only works on structs with named fields"),
    }
}

fn build_static_model_schema(config: &Config<'_>) -> TokenStream2 {
    let crate_name = &config.crate_name;
    let model_schema_ident = &config.model_schema_ident;
    let table_name = &config.table_name;

    let id_column = config.id_column_ident.to_string();
    let columns_len = config.named.iter().count();
    let columns = config
        .named
        .iter()
        .flat_map(|f| &f.ident)
        .map(|f| LitStr::new(format!("{f}").as_str(), f.span()));

    quote! {
        #[automatically_derived]
        static #model_schema_ident: #crate_name::schema::Metadata<'static, #columns_len> = #crate_name::schema::Metadata {
            table_name: #table_name,
            id_column: #id_column,
            columns: [#(#columns),*],
        };
    }
}

fn build_sqlx_crud_impl(config: &Config<'_>) -> TokenStream2 {
    let crate_name = &config.crate_name;
    let ident = &config.ident;
    let model_schema_ident = &config.model_schema_ident;
    let id_column_ident = &config.id_column_ident;
    let id_ty = config
        .named
        .iter()
        .find(|f| f.ident.as_ref() == Some(id_column_ident))
        .map(|f| &f.ty)
        .expect("the id type");

    let insert_binds = config
        .named
        .iter()
        .flat_map(|f| &f.ident)
        .map(|i| quote! { .bind(&self.#i) });
    let update_binds = config
        .named
        .iter()
        .flat_map(|f| &f.ident)
        .filter(|i| *i != id_column_ident)
        .map(|i| quote! { .bind(&self.#i) });

    let db_ty = config.db_ty.sqlx_db();

    quote! {
        #[automatically_derived]
        impl #crate_name::traits::Schema for #ident {
            type Id = #id_ty;

            fn table_name() -> &'static str {
                #model_schema_ident.table_name
            }

            fn id(&self) -> Self::Id {
                self.#id_column_ident
            }

            fn id_column() -> &'static str {
                #model_schema_ident.id_column
            }

            fn columns() -> &'static [&'static str] {
                &#model_schema_ident.columns
            }
        }

        #[automatically_derived]
        impl<'e> #crate_name::traits::Binds<'e, &'e ::sqlx::pool::Pool<#db_ty>> for #ident {
            fn insert_binds(
                &'e self,
                query: ::sqlx::query::QueryAs<'e, #db_ty, Self, <#db_ty as ::sqlx::database::HasArguments<'e>>::Arguments>
            ) -> ::sqlx::query::QueryAs<'e, #db_ty, Self, <#db_ty as ::sqlx::database::HasArguments<'e>>::Arguments> {
                query
                    #(#insert_binds)*
            }

            fn update_binds(
                &'e self,
                query: ::sqlx::query::QueryAs<'e, #db_ty, Self, <#db_ty as ::sqlx::database::HasArguments<'e>>::Arguments>
            ) -> ::sqlx::query::QueryAs<'e, #db_ty, Self, <#db_ty as ::sqlx::database::HasArguments<'e>>::Arguments> {
                query
                    #(#update_binds)*
                    .bind(&self.#id_column_ident)
            }
        }
    }
}

#[allow(dead_code)] // Usage in quote macros aren't flagged as used
struct Config<'a> {
    ident: &'a Ident,
    named: &'a Punctuated<Field, Comma>,
    crate_name: TokenStream2,
    db_ty: DbType,
    model_schema_ident: Ident,
    table_name: String,
    id_column_ident: Ident,
    external_id: bool,
}

impl<'a> Config<'a> {
    fn new(attrs: &[Attribute], ident: &'a Ident, named: &'a Punctuated<Field, Comma>) -> Self {
        let crate_name = std::env::var("CARGO_PKG_NAME").unwrap();
        let is_doctest = std::env::vars()
            .any(|(k, _)| k == "UNSTABLE_RUSTDOC_TEST_LINE" || k == "UNSTABLE_RUSTDOC_TEST_PATH");
        let crate_name = if !is_doctest && crate_name == "sqlx-meta" {
            quote! { crate }
        } else {
            quote! { ::sqlx_meta }
        };

        let db_ty = DbType::new(attrs);

        let model_schema_ident =
            format_ident!("{}_SCHEMA", ident.to_string().to_screaming_snake_case());

        let table_name = ident.to_string().to_table_case();

        // Search for a field with the #[id] attribute
        let id_attr = &named
            .iter()
            .find(|f| f.attrs.iter().any(|a| a.path.is_ident("id")))
            .and_then(|f| f.ident.as_ref());
        // Otherwise default to the first field as the "id" column
        let id_column_ident = id_attr
            .unwrap_or_else(|| {
                named
                    .iter()
                    .flat_map(|f| &f.ident)
                    .next()
                    .expect("the first field")
            })
            .clone();

        let external_id = attrs.iter().any(|a| a.path.is_ident("external_id"));

        Self {
            ident,
            named,
            crate_name,
            db_ty,
            model_schema_ident,
            table_name,
            id_column_ident,
            external_id,
        }
    }
}

enum DbType {
    Any,
    Mssql,
    MySql,
    Postgres,
    Sqlite,
}

#[allow(clippy::fallible_impl_from)]
impl From<&str> for DbType {
    fn from(db_type: &str) -> Self {
        match db_type {
            "Any" => Self::Any,
            "Mssql" => Self::Mssql,
            "MySql" => Self::MySql,
            "Postgres" => Self::Postgres,
            "Sqlite" => Self::Sqlite,
            _ => panic!("unknown #[database] type {db_type}"),
        }
    }
}

impl DbType {
    fn new(attrs: &[Attribute]) -> Self {
        match attrs
            .iter()
            .find(|a| a.path.is_ident("database"))
            .map(syn::Attribute::parse_meta)
        {
            Some(Ok(Meta::NameValue(MetaNameValue {
                lit: Lit::Str(s), ..
            }))) => Self::from(&*s.value()),
            _ => Self::Sqlite,
        }
    }

    fn sqlx_db(&self) -> TokenStream2 {
        match self {
            Self::Any => quote! { ::sqlx::Any },
            Self::Mssql => quote! { ::sqlx::Mssql },
            Self::MySql => quote! { ::sqlx::MySql },
            Self::Postgres => quote! { ::sqlx::Postgres },
            Self::Sqlite => quote! { ::sqlx::Sqlite },
        }
    }
}
