use quote::quote;
use syn::DeriveInput;

use crate::cfg::FieldInfo;

/// Generate the `ServicesContext` implementation for the given struct.
pub fn generate_context_impl(
    DeriveInput {
        ident: name,
        generics,
        ..
    }: DeriveInput,
    config_field: FieldInfo,
) -> proc_macro2::TokenStream {
    let field_access = match config_field {
        FieldInfo::Named(ident) => quote! { self.#ident },
        FieldInfo::Unnamed(index) => quote! { self.#index },
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics gadget_sdk::ctx::ServicesContext for #name #ty_generics #where_clause {
            type Config = gadget_sdk::ext::subxt::PolkadotConfig;
            /// Get the current blueprint information from the context.
            fn current_blueprint(
                &self,
                client: &gadget_sdk::ext::subxt::OnlineClient<Self::Config>,
            ) -> impl core::future::Future<
                Output = Result<
                    gadget_sdk::ext::tangle_subxt::tangle_testnet_runtime::api::runtime_types::tangle_primitives::services::ServiceBlueprint,
                    gadget_sdk::ext::subxt::Error
                >
            > {
                use gadget_sdk::ext::tangle_subxt::tangle_testnet_runtime::api;
                use gadget_sdk::ext::subxt;

                async move {
                    let blueprint_id = #field_access.blueprint_id;
                    let blueprint = api::storage().services().blueprints(blueprint_id);
                    let storage = client.storage().at_latest().await?;
                    let result = storage.fetch(&blueprint).await?;
                    match result {
                        Some((_, blueprint)) => Ok(blueprint),
                        None => Err(subxt::Error::Other(format!(
                            "Blueprint with id {blueprint_id} not found"
                        ))),
                    }
                }
            }

            /// Query the current blueprint owner from the context.
            fn current_blueprint_owner(
                &self,
                client: &gadget_sdk::ext::subxt::OnlineClient<Self::Config>,
            ) -> impl core::future::Future<Output = Result<gadget_sdk::ext::subxt::utils::AccountId32, gadget_sdk::ext::subxt::Error>> {
                use gadget_sdk::ext::tangle_subxt::tangle_testnet_runtime::api;
                use gadget_sdk::ext::subxt;
                async move {
                    let blueprint_id = #field_access.blueprint_id;
                    let blueprint = api::storage().services().blueprints(blueprint_id);
                    let storage = client.storage().at_latest().await?;
                    let result = storage.fetch(&blueprint).await?;
                    match result {
                        Some((account_id, _)) => Ok(account_id),
                        None => Err(subxt::Error::Other(format!(
                            "Blueprint with id {blueprint_id} not found"
                        ))),
                    }
                }
            }

            /// Get the current service operators from the context.
            /// This function will return a list of service operators that are selected to run this service
            /// instance.
            fn current_service_operators(
                &self,
                client: &gadget_sdk::ext::subxt::OnlineClient<Self::Config>,
            ) -> impl core::future::Future<
                Output = Result<
                    Vec<(
                        gadget_sdk::ext::subxt::utils::AccountId32,
                        gadget_sdk::ext::tangle_subxt::tangle_testnet_runtime::api::runtime_types::sp_arithmetic::per_things::Percent,
                    )>,
                    gadget_sdk::ext::subxt::Error
                >
            > {
                use gadget_sdk::ext::tangle_subxt::tangle_testnet_runtime::api;
                use gadget_sdk::ext::subxt;

                async move {
                    let service_instance_id = match #field_access.service_id {
                        Some(service_id) => service_id,
                        None => return Err(subxt::Error::Other("Service instance id is not set".to_string())),
                    };
                    let service_instance = api::storage().services().instances(service_instance_id);
                    let storage = client.storage().at_latest().await?;
                    let result = storage.fetch(&service_instance).await?;
                    match result {
                        Some(instance) => Ok(instance.operators.0),
                        None => Err(subxt::Error::Other(format!(
                            "Service instance {service_instance_id} is not created, yet"
                        ))),
                    }
                }
            }
        }
    }
}
