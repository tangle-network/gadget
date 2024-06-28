#[cfg(test)]
mod tests {
    use shell_sdk::prelude::tangle_primitives::jobs::v2::{
        FieldType, JobDefinition, JobMetadata, JobResultVerifier, ServiceBlueprint, ServiceMetadata,
    };

    #[tokio::test]
    async fn main() {
        let stub_blueprint = ServiceBlueprint {
            metadata: ServiceMetadata::default(),
            jobs: vec![JobDefinition {
                metadata: JobMetadata {
                    name: "keygen".try_into().unwrap(),
                    ..Default::default()
                },
                params: vec![FieldType::Uint8].try_into().unwrap(),
                result: vec![FieldType::Bytes].try_into().unwrap(),
                verifier: JobResultVerifier::None,
            }]
            .try_into()
            .unwrap(),
            registration_hook: Default::default(),
            registration_params: Default::default(),
            request_hook: Default::default(),
            request_params: Default::default(),
            gadget: Default::default(),
        };
    }
}
