use gadget_common::gadget::network::gossip;
use gadget_common::gadget::network::gossip::{
    GossipHandler, GossipHandlerController, NetworkGossipEngineBuilder,
};
use gadget_common::prelude::{DebugLogger, ECDSAKeyStore, KeystoreBackend};
use libp2p_gadget::config::FullNetworkConfiguration;
use libp2p_gadget::{NetworkService, NotificationService};
use std::sync::Arc;

pub fn configurator(
    full_network_config: &mut FullNetworkConfiguration,
) -> Vec<Box<dyn NotificationService>> {
    let (set_config, notification_service_0) = gossip::set_config(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME.into(),
    );
    full_network_config.add_notification_protocol(set_config);

    let (set_config, notification_service_1) = gossip::set_config(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_SIGNING_PROTOCOL_NAME.into(),
    );
    full_network_config.add_notification_protocol(set_config);

    let (set_config, notification_service_2) = gossip::set_config(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME.into(),
    );
    full_network_config.add_notification_protocol(set_config);

    let (set_config, notification_service_3) = gossip::set_config(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME.into(),
    );
    full_network_config.add_notification_protocol(set_config);

    vec![
        notification_service_0,
        notification_service_1,
        notification_service_2,
        notification_service_3,
    ]
}

pub fn gossip_builder<KBE: KeystoreBackend>(
    keystore: ECDSAKeyStore<KBE>,
    network: Arc<NetworkService>,
    logger: DebugLogger,
) -> color_eyre::Result<(Vec<GossipHandler<KBE>>, Vec<GossipHandlerController>)> {
    let (network_keygen_handler, network_keygen_controller) = NetworkGossipEngineBuilder::new(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYGEN_PROTOCOL_NAME.into(),
        keystore.clone(),
    )
    .build(network.clone(), None, logger.clone())?;

    let (network_signing_handler, network_signing_controller) = NetworkGossipEngineBuilder::new(
        dfns_cggmp21_protocol::constants::DFNS_CGGMP21_SIGNING_PROTOCOL_NAME.into(),
        keystore.clone(),
    )
    .build(network.clone(), None, logger.clone())?;

    let (network_key_refresh_handler, network_key_refresh_controller) =
        NetworkGossipEngineBuilder::new(
            dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYREFRESH_PROTOCOL_NAME.into(),
            keystore.clone(),
        )
        .build(network.clone(), None, logger.clone())?;

    let (network_key_rotate_handler, network_key_rotate_controller) =
        NetworkGossipEngineBuilder::new(
            dfns_cggmp21_protocol::constants::DFNS_CGGMP21_KEYROTATE_PROTOCOL_NAME.into(),
            keystore.clone(),
        )
        .build(network.clone(), None, logger.clone())?;

    let gossip_handlers = vec![
        network_keygen_handler,
        network_signing_handler,
        network_key_refresh_handler,
        network_key_rotate_handler,
    ];
    let gossip_controllers = vec![
        network_keygen_controller,
        network_signing_controller,
        network_key_refresh_controller,
        network_key_rotate_controller,
    ];

    Ok((gossip_handlers, gossip_controllers))
}
