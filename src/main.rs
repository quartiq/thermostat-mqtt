#![no_std]
#![no_main]

use log::info;
use panic_halt as _;

mod adc;
mod dac;
mod leds;
mod network_users;
mod setup;
mod shared;
mod telemetry;
mod unit_conversion;

use adc::Adc;
use dac::{Dacs, Pwms};
use idsp::iir;
use leds::Leds;

use miniconf::Miniconf;
use network_users::{NetworkState, NetworkUsers};
use rtic::cyccnt::U32Ext as _;
use serde::Deserialize;
use stm32_eth;
use stm32_eth::stm32::Peripherals;
use telemetry::{Telemetry, TelemetryBuffer};
use unit_conversion::{i_to_dac, pid_to_iir, temp_to_iiroffset, MAXI, VREF_DAC, VREF_TEC};

const IIR_CASCADE_LENGTH: usize = 1; // Number of concatenated IIRs. Settings only support one right now.
const CYC_PER_S: u32 = 168_000_000; // 168MHz main clock
const LED_PERIOD: u32 = CYC_PER_S / 2; // LED blinking period
const ETH_P_PERIOD: u32 = CYC_PER_S / 1000; // Ethernet polling period
const OUTSCALE: f32 = 131072.0 * VREF_TEC / (VREF_DAC / 2.0); // Output scale. Zero current is slightly off center.

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct PidSettings {
    pub pid: [f32; 3],
    pub target: f32,
    pub max_i_neg: f32,
    pub max_i_pos: f32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct AdcFilterSettings {
    pub odr: u32,
    pub order: u32,
    pub enhfilt: u32,
    pub enhfilten: u32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct Settings {
    telemetry_period: f32,
    led: bool,
    dacs: [f32; 2],
    pidsettings: [PidSettings; 2],
    engage_iir: [bool; 2],
    adcsettings: AdcFilterSettings,
    max_v_tec: [f32; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_period: 1.0,
            led: false,
            dacs: [0.0, 0.0],
            engage_iir: [false, false],
            adcsettings: AdcFilterSettings {
                odr: 0b10101,   // 10Hz output data rate
                order: 0,       // Sinc5+Sinc1 filter
                enhfilt: 0b110, // 16.67 SPS, 92 dB rejection, 60 ms settling
                enhfilten: 1,   // enable postfilter
            },
            max_v_tec: [1.0, 1.0],
            pidsettings: [
                PidSettings {
                    pid: [1.0, 0., 0.],
                    target: 25.0,
                    max_i_neg: 0.1,
                    max_i_pos: 0.1,
                },
                PidSettings {
                    pid: [1.0, 0., 0.],
                    target: 25.0,
                    max_i_neg: 0.1,
                    max_i_pos: 0.1,
                },
            ],
        }
    }
}

#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        leds: Leds,
        adc: Adc,
        dacs: Dacs,
        pwms: Pwms,
        iirs: [[iir::IIR<f64>; IIR_CASCADE_LENGTH]; 2],
        #[init([[[0.; 5]; IIR_CASCADE_LENGTH]; 2])]
        iir_state: [[iir::Vec5<f64>; IIR_CASCADE_LENGTH]; 2],
        network: NetworkUsers<Settings, Telemetry>,
        settings: Settings,
        telemetry: TelemetryBuffer,
    }

    #[init(schedule = [blink, poll_eth, process, tele], spawn = [settings_update])]
    fn init(c: init::Context) -> init::LateResources {
        let thermostat = setup::setup(c.core, c.device);

        let network = NetworkUsers::new(
            thermostat.network_devices.stack,
            env!("CARGO_BIN_NAME"),
            thermostat.network_devices.mac_address,
            option_env!("BROKER")
                .unwrap_or("10.42.0.1")
                .parse()
                .unwrap(),
        );

        log::info!("Network users done");

        let settings = Settings::default();

        c.schedule.blink(c.start + LED_PERIOD.cycles()).unwrap();
        c.schedule
            .poll_eth(c.start + ETH_P_PERIOD.cycles())
            .unwrap();
        c.schedule.tele(c.start + CYC_PER_S.cycles()).unwrap();

        // apply default settings
        c.spawn.settings_update().unwrap();
        log::info!("---Init Done");
        init::LateResources {
            leds: thermostat.leds,
            adc: thermostat.adc,
            dacs: thermostat.dacs,
            pwms: thermostat.pwms,
            iirs: [[iir::IIR::new(1., 0.0, 0.0); IIR_CASCADE_LENGTH]; 2],
            network,
            settings,
            telemetry: TelemetryBuffer::default(),
        }
    }

    #[task(priority=1, resources=[dacs, iir_state, iirs, telemetry, settings])]
    fn process(c: process::Context, adcdata: [u32; 2]) {
        info!("adcdata:\t ch0: {:?}\t ch1: {:?}", adcdata[0], adcdata[1]);
        let dacs = c.resources.dacs;
        let iir_state = c.resources.iir_state;
        let iirs = c.resources.iirs;
        let telemetry = c.resources.telemetry;
        let settings = c.resources.settings;

        for ch in 0..adcdata.len() {
            let y = iirs[ch]
                .iter()
                .zip(iir_state[ch].iter_mut())
                .fold(adcdata[ch] as f64, |yi, (iir_ch, state)| {
                    iir_ch.update(state, yi, false)
                });
            if settings.engage_iir[ch] {
                dacs.set((y + OUTSCALE as f64) as u32, ch as u8);
            }
        }
        telemetry.adcs = adcdata;
        telemetry.dacs = dacs.val;
    }

    #[task(priority = 1, resources=[network, settings, dacs, adc, pwms, iirs])]
    fn settings_update(c: settings_update::Context) {
        log::info!("updating settings");
        let settings = c.resources.network.miniconf.settings();

        *c.resources.settings = *settings;

        c.resources.adc.set_filters(settings.adcsettings);

        c.resources.pwms.set_all(
            // set currents to 5% of max higher in order to avoid railing the driver before the filter.
            settings.pidsettings[0].max_i_neg + (0.05 * MAXI),
            settings.pidsettings[0].max_i_pos + (0.05 * MAXI),
            settings.max_v_tec[0],
            settings.pidsettings[1].max_i_neg + (0.05 * MAXI),
            settings.pidsettings[1].max_i_pos + (0.05 * MAXI),
            settings.max_v_tec[1],
        );

        for (i, iir) in c.resources.iirs.iter_mut().enumerate() {
            iir[0]
                .ba
                .iter_mut()
                .zip(pid_to_iir(settings.pidsettings[i].pid).iter())
                .map(|(d, x)| *d = *x as f64)
                .last();
            iir[0].set_x_offset(temp_to_iiroffset(settings.pidsettings[i].target) as f64); // set output offset to input target
            iir[0].y_min = (i_to_dac(-settings.pidsettings[i].max_i_neg) as f32 - OUTSCALE) as f64;
            iir[0].y_max = (i_to_dac(settings.pidsettings[i].max_i_pos) as f32 - OUTSCALE) as f64;
        }

        for (i, eng) in settings.engage_iir.iter().enumerate() {
            if !*eng {
                c.resources.dacs.set(i_to_dac(settings.dacs[i]), i as u8);
                // disable channel if set to zero current and iir not engaged
                if settings.dacs[i] == 0.0 {
                    c.resources.dacs.dis_ch(i as u8);
                } else {
                    c.resources.dacs.en_ch(i as u8);
                }
            } else {
                c.resources.dacs.en_ch(i as u8);
            }
        }
    }

    #[task(priority = 1, resources = [network], schedule = [poll_eth],  spawn=[settings_update])]
    fn poll_eth(c: poll_eth::Context) {
        static mut NOW: u32 = 0;

        match c.resources.network.update(*NOW) {
            NetworkState::SettingsChanged => c.spawn.settings_update().unwrap(),
            NetworkState::Updated => {}
            NetworkState::NoChange => {}
        }
        *NOW = *NOW + 1;
        c.schedule
            .poll_eth(c.scheduled + ETH_P_PERIOD.cycles())
            .unwrap();
    }

    #[idle(resources=[adc], spawn=[process])]
    fn idle(mut c: idle::Context) -> ! {
        let mut adcdata1 = 0; // initialize to zero in case ch0 comes first
        loop {
            let statreg = c.resources.adc.lock(|adc| adc.get_status_reg());
            if statreg != 0xff {
                let (adcdata, ch) = c.resources.adc.lock(|adc| adc.read_data());
                match ch {
                    0 => {
                        adcdata1 = adcdata;
                    }
                    _ => {
                        // ADC ch1 is Thermostat ch0
                        let adcdata0 = adcdata;
                        c.spawn.process([adcdata0, adcdata1]).unwrap();
                    }
                }
            }
        }
    }

    #[task(priority = 1, resources = [network, telemetry, settings], schedule = [tele])]
    fn tele(c: tele::Context) {
        c.resources
            .network
            .telemetry
            .publish(&c.resources.telemetry.finalize());

        c.schedule
            .tele(
                c.scheduled
                    + ((c.resources.settings.telemetry_period * CYC_PER_S as f32) as u32).cycles(),
            )
            .unwrap();
    }

    #[task(priority = 1, resources = [leds], schedule = [blink])]
    fn blink(c: blink::Context) {
        static mut LED_STATE: bool = false;
        if *LED_STATE {
            c.resources.leds.g3.off();
            *LED_STATE = false;
        } else {
            c.resources.leds.g3.on();
            *LED_STATE = true;
        }

        c.schedule.blink(c.scheduled + LED_PERIOD.cycles()).unwrap();
    }

    #[task(binds = ETH, priority = 1)]
    fn eth(_: eth::Context) {
        let p = unsafe { Peripherals::steal() };
        stm32_eth::eth_interrupt_handler(&p.ETHERNET_DMA);
    }

    extern "C" {
        fn EXTI0();
    }
};
