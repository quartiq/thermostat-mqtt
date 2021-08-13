///!  network management module
///!
///! # Design
///! The stabilizer network architecture supports numerous layers to permit transmission of
///! telemetry (via MQTT), configuration of run-time settings (via MQTT + Miniconf), and live data
///! streaming over raw UDP/TCP sockets. This module encompasses the main processing routines
///! related to networking operations.
pub use heapless;
pub use miniconf;
pub use serde;

// pub mod data_stream;
// pub mod messages;
// pub mod miniconf_client;
// pub mod shared;
// pub mod telemetry;

use crate::setup::{NetworkStack};
// use data_stream::{BlockGenerator, DataStream};
use minimq::embedded_nal::IpAddr;
// use network_processor::NetworkProcessor;
use crate::shared::NetworkManager;
use crate::telemetry::TelemetryClient;

use core::fmt::Write;
use heapless::String;
use miniconf::Miniconf;
use serde::Serialize;
use smoltcp_nal::embedded_nal::SocketAddr;

pub type NetworkReference = crate::shared::NetworkStackProxy<'static, NetworkStack>;

#[derive(Copy, Clone, PartialEq)]
pub enum UpdateState {
    NoChange,
    Updated,
}

#[derive(Copy, Clone, PartialEq)]
pub enum NetworkState {
    SettingsChanged,
    Updated,
    NoChange,
}

pub struct NetworkUsers<S: Default + Miniconf, T: Serialize> {
    pub miniconf: miniconf::MqttClient<S, NetworkReference>,
    stackref: NetworkReference,
    // pub processor: NetworkProcessor,
    // stream: DataStream,
    // generator: Option<FrameGenerator>,
    pub telemetry: TelemetryClient<T>,
}

impl<S, T> NetworkUsers<S, T>
where
    S: Default + Miniconf,
    T: Serialize,
{
    /// Construct default network users.
    ///
    /// # Args
    /// * `stack` - The network stack that will be used to share with all network users.
    /// * `phy` - The ethernet PHY connecting the network.
    /// * `cycle_counter` - The clock used for measuring time in the network.
    /// * `app` - The name of the application.
    /// * `mac` - The MAC address of the network.
    /// * `broker` - The IP address of the MQTT broker to use.
    ///
    /// # Returns
    /// A new struct of network users.
    pub fn new(
        stack: NetworkStack,
        // phy: EthernetPhy,
        // cycle_counter: CycleCounter,
        app: &str,
        mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
        broker: IpAddr,
    ) -> Self {
        let stack_manager =
            cortex_m::singleton!(: NetworkManager = NetworkManager::new(stack))
                .unwrap();

        // let processor = NetworkProcessor::new(
        //     stack_manager.acquire_stack(),
        //     phy,
        //     cycle_counter,
        // );

        let prefix = get_device_prefix(app, mac);

        let settings = miniconf::MqttClient::new(
            stack_manager.acquire_stack(),
            &get_client_id(app, "settings", mac),
            &prefix,
            broker,
        )
        .unwrap();

        let telemetry = TelemetryClient::new(
            stack_manager.acquire_stack(),
            &get_client_id(app, "tlm", mac),
            &prefix,
            broker,
        );

        // let (generator, stream) =
        //     data_stream::setup_streaming(stack_manager.acquire_stack());

        let stackref = stack_manager.acquire_stack();

        NetworkUsers {
            miniconf: settings,
            stackref,
            // processor,
            telemetry,
            // stream,
            // generator: Some(generator),
        }
    }


    /// Update and process all of the network users state.
    ///
    /// # Returns
    /// An indication if any of the network users indicated a state change.
    pub fn update(&mut self, now: u32) -> NetworkState {
        // // Update the MQTT clients.
        // self.telemetry.update();
        //
        // // Update the data stream.
        // if self.generator.is_none() {
        //     self.stream.process();
        // }
        //
        // Poll for incoming data.
        let poll_result = match self.stackref.lock(|stack| stack.poll(now)) {
            Ok(true) => NetworkState::Updated,
            Ok(false) =>  NetworkState::NoChange,
            Err(_) => NetworkState::Updated,
        };

        match self.miniconf.update() {
            Ok(true) => NetworkState::SettingsChanged,
            _ => poll_result,
        }
    }


}

/// Get an MQTT client ID for a client.
///
/// # Args
/// * `app` - The name of the application
/// * `client` - The unique tag of the client
/// * `mac` - The MAC address of the device.
///
/// # Returns
/// A client ID that may be used for MQTT client identification.
fn get_client_id(
    app: &str,
    client: &str,
    mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
) -> String<64> {
    let mut identifier = String::new();
    write!(&mut identifier, "{}-{}-{}", app, mac, client).unwrap();
    identifier
}

/// Get the MQTT prefix of a device.
///
/// # Args
/// * `app` - The name of the application that is executing.
/// * `mac` - The ethernet MAC address of the device.
///
/// # Returns
/// The MQTT prefix used for this device.
pub fn get_device_prefix(
    app: &str,
    mac: smoltcp_nal::smoltcp::wire::EthernetAddress,
) -> String<128> {
    // Note(unwrap): The mac address + binary name must be short enough to fit into this string. If
    // they are defined too long, this will panic and the device will fail to boot.
    let mut prefix: String<128> = String::new();
    write!(&mut prefix, "dt/sinara/{}/{}", app, mac).unwrap();

    prefix
}
