use syn::{Data, Error, Fields, Ident, Index, Result};

pub enum FieldInfo {
    Named(Ident),
    Unnamed(Index),
}

pub fn find_config_field(input_ident: &Ident, data: &Data) -> Result<FieldInfo> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("config"))
                    {
                        return field.ident.clone().map(FieldInfo::Named).ok_or_else(|| {
                            Error::new_spanned(
                                field,
                                "Expected named field with #[config] attribute",
                            )
                        });
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    "No field with #[config] attribute found, please add #[config] to the field that holds the `gadget_sdk::config::GadgetConfiguration`",
                ))
            }
            Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident("config"))
                    {
                        return Ok(FieldInfo::Unnamed(Index::from(i)));
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    "No field with #[config] attribute found, please add #[config] to the field that holds the `gadget_sdk::config::GadgetConfiguration`",
                ))
            }
            Fields::Unit => Err(Error::new_spanned(
                input_ident,
                "Context Extensions traits cannot be derived for unit structs",
            )),
        },
        _ => Err(Error::new_spanned(
            input_ident,
            "Context Extensions traits can only be derived for structs",
        )),
    }
}
