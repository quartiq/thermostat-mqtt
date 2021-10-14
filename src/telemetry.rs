///! Stabilizer Telemetry Capabilities
///!
///! # Design
///! Telemetry is reported regularly using an MQTT client. All telemetry is reported in SI units
///! using standard JSON format.
///!
///! In order to report ADC/DAC codes generated during the DSP routines, a telemetry buffer is
///! employed to track the latest codes. Converting these codes to SI units would result in
///! repetitive and unnecessary calculations within the DSP routine, slowing it down and limiting
///! sampling frequency. Instead, the raw codes are stored and the telemetry is generated as
///! required immediately before transmission. This ensures that any slower computation required
///! for unit conversion can be off-loaded to lower priority tasks.
use heapless::{String, Vec};
use minimq::QoS;
use serde::Serialize;

use crate::network_users::NetworkReference;
// use crate::hardware::{adc::AdcCode, afe::Gain, dac::DacCode};
use crate::unit_conversion::{adc_to_temp, dac_to_i, i_to_dac, pid_to_iir, temp_to_iiroffset};
use minimq::embedded_nal::IpAddr;

/// The telemetry client for reporting telemetry data over MQTT.
pub struct TelemetryClient<T: Serialize> {
    mqtt: minimq::Minimq<NetworkReference, 256>,
    telemetry_topic: String<128>,
    _telemetry: core::marker::PhantomData<T>,
}

/// The telemetry buffer is used for storing sample values during execution.
///
/// # Note
/// These values can be converted to SI units immediately before reporting to save processing time.
/// This allows for the DSP process to continually update the values without incurring significant
/// run-time overhead during conversion to SI units.
#[derive(Copy, Clone)]
pub struct TelemetryBuffer {
    pub adcs: [u32; 2],
    pub dacs: [u32; 2],
}

impl Default for TelemetryBuffer {
    fn default() -> Self {
        Self {
            adcs: [0, 0],
            dacs: [0, 0],
        }
    }
}

/// The telemetry structure is data that is ultimately reported as telemetry over MQTT.
///
/// # Note
/// This structure should be generated on-demand by the buffer when required to minimize conversion
/// overhead.
#[derive(Serialize)]
pub struct Telemetry {
    pub dacs: [f32; 2],
    pub adcs: [f64; 2],
}

impl Default for Telemetry {
    fn default() -> Self {
        Self {
            dacs: [0.0, 0.0],
            adcs: [0.0, 0.0],
        }
    }
}

impl TelemetryBuffer {
    /// Convert the telemetry buffer to finalized, SI-unit telemetry for reporting.
    ///
    /// # Args
    /// * `afe0` - The current AFE configuration for channel 0.
    /// * `afe1` - The current AFE configuration for channel 1.
    ///
    /// # Returns
    /// The finalized telemetry structure that can be serialized and reported.
    pub fn finalize(self) -> Telemetry {
        Telemetry {
            adcs: [adc_to_temp(self.adcs[0]), adc_to_temp(self.adcs[1])],
            dacs: [dac_to_i(self.dacs[0]), dac_to_i(self.dacs[1])],
        }
    }
}

impl<T: Serialize> TelemetryClient<T> {
    /// Construct a new telemetry client.
    ///
    /// # Args
    /// * `stack` - A reference to the (shared) underlying network stack.
    /// * `client_id` - The MQTT client ID of the telemetry client.
    /// * `prefix` - The device prefix to use for MQTT telemetry reporting.
    /// * `broker` - The IP address of the MQTT broker to use.
    ///
    /// # Returns
    /// A new telemetry client.
    pub fn new(stack: NetworkReference, client_id: &str, prefix: &str, broker: IpAddr) -> Self {
        let mqtt = minimq::Minimq::new(broker, client_id, stack).unwrap();

        let mut telemetry_topic: String<128> = String::from(prefix);
        telemetry_topic.push_str("/telemetry").unwrap();

        Self {
            mqtt,
            telemetry_topic,
            _telemetry: core::marker::PhantomData::default(),
        }
    }

    /// Publish telemetry over MQTT
    ///
    /// # Note
    /// Telemetry is reported in a "best-effort" fashion. Failure to transmit telemetry will cause
    /// it to be silently dropped.
    ///
    /// # Args
    /// * `telemetry` - The telemetry to report
    pub fn publish(&mut self, telemetry: &T) {
        let telemetry: Vec<u8, 256> = serde_json_core::to_vec(telemetry).unwrap();

        // log::info!("{:?}", telemetry);
        self.mqtt
            .client
            .publish(&self.telemetry_topic, &telemetry, QoS::AtMostOnce, &[])
            .ok();
    }

    /// Update the telemetry client
    ///
    /// # Note
    /// This function is provided to force the underlying MQTT state machine to process incoming
    /// and outgoing messages. Without this, the client will never connect to the broker. This
    /// should be called regularly.
    pub fn update(&mut self) {
        match self.mqtt.poll(|_client, _topic, _message, _properties| {}) {
            Err(minimq::Error::Network(smoltcp_nal::NetworkError::NoIpAddress)) => {}

            Err(error) => log::info!("Unexpected error: {:?}", error),
            _ => {}
        }
    }
}
