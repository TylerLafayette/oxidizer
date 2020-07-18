use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2};
use quote::{quote};
use syn::{parse_macro_input, DeriveInput};

use super::props::*;

pub struct EntityBuilder {
}

impl EntityBuilder {
    pub fn new() -> Self {
        EntityBuilder{}
    }

    fn build_save_fn(&self, props: &Props) -> TokenStream2 {
        let table_name = props.get_table_name();
        let fields_plain_names = props.get_fields_plain_names();
        let fields_plain_numbered = props.get_fields_plain_numbered();
        let fields_plain_numbered_next_index = props.get_fields_plain_numbered_next_index();

        let primary_key = props.get_primary_key_field().unwrap();
        let primary_key_ident = &primary_key.ident;
        let primary_key_type = &primary_key.ty;

        quote! {
            async fn save(&mut self, db: &dboom::db::DB) -> dboom::db::DBResult<bool> {
                let mut creating = false;
                let primary_key_default: #primary_key_type = Default::default();
                let _result = match self.#primary_key_ident {
                    v if self.#primary_key_ident == primary_key_default => {
                        creating = true;
                        let rows = db.query(
                            concat!(
                                "INSERT INTO ",
                                #table_name,
                                " (",
                                stringify!(#(#fields_plain_names),*),
                                ") values(",
                                #(#fields_plain_numbered),*,
                                ") RETURNING ",
                                stringify!(#primary_key_ident),
                                ";"
                            ),
                            &[#( &self.#fields_plain_names),*]
                        ).await?;
                        let first_row = rows.first().ok_or(dboom::db::Error::Other)?;
                        self.#primary_key_ident = first_row.get::<&str, #primary_key_type>(stringify!(#primary_key_ident));
                        1
                    },
                    id => {
                        db.execute(
                            concat!(
                                "UPDATE ",
                                #table_name,
                                " SET ",
                                #(stringify!(#fields_plain_names =), #fields_plain_numbered),*,
                                " WHERE ",
                                stringify!(#primary_key_ident),
                                "= $",
                                #fields_plain_numbered_next_index
                            ),
                            &[#( &self.#fields_plain_names),*, &self.#primary_key_ident],
                        ).await?
                    }
                };

                Ok(creating)
            }
        }
    }

    fn build_from_row_fn(&self, props: &Props) -> TokenStream2 {
        let fields_all_names = props.get_fields_all_names();
        let fields_all_types = props.get_fields_all_types();
        quote! {
            fn from_row(row: &dboom::tokio_postgres::Row) -> Self {
                let mut obj: Self = Self{
                    #(
                        #fields_all_names: row.get::<&str, #fields_all_types>(concat!(stringify!(#fields_all_names))),
                    )*
                };
                obj
            }
        }
    }

    fn build_create_migration_fn(&self, props: &Props) -> TokenStream2 {
        let table_name = props.get_table_name();
        let fields_all_names = props.get_fields_all_names();
        let fields_all_db_types = props.get_fields_all_db_types();
        quote! {
             async fn create_migration() -> dboom::db::DBResult<dboom::Migration> {
                let mut m = dboom::Migration::new();
                m.create_table(#table_name, |t| {
                    #(t.add_column(stringify!(#fields_all_names), #fields_all_db_types);)*
                });
                Ok(m)
            }
        }
    }

    fn build_find_fn(&self, props: &Props) -> TokenStream2 {
        let name = props.get_name();
        let table_name = props.get_table_name();
        quote! {
            async fn find(db: &dboom::db::DB, condition: &str, params: &'_ [&'_ (dyn dboom::db_types::ToSql + Sync)]) -> dboom::db::DBResult<Vec<#name>> {
                let query_str = format!("SELECT * FROM {} WHERE {}", #table_name, condition);
                let rows = db.query(&query_str, params).await?;
                let results: Vec<#name> = rows.iter().map(|row| Self::from_row(row)).collect();
                Ok(results)
            }
        }
    }

    fn build_first_fn(&self, props: &Props) -> TokenStream2 {
        let name = props.get_name();
        let table_name = props.get_table_name();
        quote! {
            async fn first(db: &dboom::db::DB, condition: &str, params: &'_ [&'_ (dyn dboom::db_types::ToSql + Sync)]) -> dboom::db::DBResult<Option<#name>> {
                let query_str = format!("SELECT * FROM {} WHERE {} LIMIT 1", #table_name, condition);
                let rows = db.query(&query_str, params).await?;
                let mut results: Vec<#name> = rows.iter().map(|row| Self::from_row(row)).collect();
                match results.len() {
                    0 => Ok(None),
                    _ => Ok(Some(results.remove(0))),
                }
            }
        }
    }

    fn build_delete_fn(&self, props: &Props) -> TokenStream2 {
        let primary_key_ident = &props.get_primary_key_field().unwrap().ident;
        let table_name = props.get_table_name();
        quote! {
            async fn delete(&mut self, db: &dboom::db::DB) -> dboom::db::DBResult<bool> {
                if self.#primary_key_ident == Default::default() {
                    return Ok(false);
                }

                let condition = format!("{} = $1", stringify!(#primary_key_ident));
                let query_str = format!("DELETE FROM {} WHERE {}", #table_name, condition);
                match db.execute(&query_str, &[&self.#primary_key_ident]).await? {
                    0 => Ok(false),
                    _ => {
                        self.#primary_key_ident = 0;
                        Ok(true)
                    },
                }
            }
        }
    }

    pub fn build(&self, item: TokenStream) -> TokenStream {
        let input = parse_macro_input!(item as DeriveInput);

        let props = Props::new(input);

        if let Some(ts) = props.check() {
            return ts;
        }

        eprintln!("{:#?}", props.get_fields_all_types());

        let save_fn = self.build_save_fn(&props);
        let delete_fn = self.build_delete_fn(&props);
        let from_row_fn = self.build_from_row_fn(&props);
        let create_migration_fn = self.build_create_migration_fn(&props);
        let find_fn = self.build_find_fn(&props);
        let first_fn = self.build_first_fn(&props);

        let name = props.get_name();

        let expanded = quote! {
            pub use dboom::entity::Entity;

            #[dboom::async_trait]
            impl dboom::entity::Entity for #name {
                #save_fn

                #delete_fn

                #from_row_fn

                #create_migration_fn

                #find_fn

                #first_fn
            }
        };

        // Hand the output tokens back to the compiler
        let r = TokenStream::from(expanded);

        println!("{}", r);

        r
    }
}