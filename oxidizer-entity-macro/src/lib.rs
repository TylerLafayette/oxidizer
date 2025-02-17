use proc_macro::TokenStream;

mod attrs;
mod entity_builder;
mod field_extras;
mod props;
mod utils;

/// Entity derive macro
#[proc_macro_derive(
    Entity,
    attributes(
        primary_key,
        indexed,
        relation,
        entity,
        index,
        has_many,
        field_ignore,
        custom_type
    )
)]
pub fn entity_macro(item: TokenStream) -> TokenStream {
    entity_builder::EntityBuilder::new().build(item)
}
