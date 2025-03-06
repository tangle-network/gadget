use syn::{Data, Error, Fields, Ident, Index, Result};

pub enum FieldInfo {
    Named(Ident),
    Unnamed(Index),
}

pub fn find_config_field(
    input_ident: &Ident,
    data: &Data,
    tag_name: &'static str,
    tag_ty: &'static str,
) -> Result<FieldInfo> {
    match data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => {
                for field in &fields.named {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident(tag_name))
                    {
                        return field.ident.clone().map(FieldInfo::Named).ok_or_else(|| {
                            Error::new_spanned(
                                field,
                                format!("Expected named field with #[{tag_name}] attribute"),
                            )
                        });
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    format!(
                        "No field with #[{tag_name}] attribute found, please add #[{tag_name}] to the field that holds the `{tag_ty}`"
                    ),
                ))
            }
            Fields::Unnamed(fields) => {
                for (i, field) in fields.unnamed.iter().enumerate() {
                    if field
                        .attrs
                        .iter()
                        .any(|attr| attr.path().is_ident(tag_name))
                    {
                        return Ok(FieldInfo::Unnamed(Index::from(i)));
                    }
                }
                Err(Error::new_spanned(
                    input_ident,
                    format!(
                        "No field with #[{tag_name}] attribute found, please add #[{tag_name}] to the field that holds the `{tag_ty}`"
                    ),
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
