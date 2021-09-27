#![no_std]
#![no_main]

use cortex_m::asm::delay;
use log::{error, info, warn};
use panic_halt as _;

mod network_users;
mod telemetry;
mod unit_conversion;
use network_users::{NetworkState, NetworkUsers, UpdateState};
use telemetry::Telemetry;
use unit_conversion::{adc_to_temp, dac_to_i, i_to_dac, pid_to_iir, temp_to_iiroffset};

mod adc;
mod dac;
mod leds;
mod setup;

use adc::Adc;
use dac::{Dacs, Pwms};
use idsp::iir;
use leds::Leds;

use stm32_eth;

use stm32_eth::stm32::Peripherals;

use rtic::cyccnt::{Instant, U32Ext as _};

pub mod shared;

pub use miniconf::{Miniconf, MiniconfAtomic};
pub use num_traits;
pub use serde::Deserialize;

// The number of cascaded IIR biquads per channel. Select 1 or 2!
const IIR_CASCADE_LENGTH: usize = 1;
const LED_PERIOD: u32 = 1 << 25;
const CYC_PER_S: u32 = 168_000_000; // clock is 168MHz
const SCALE: f32 = 8388608.0;

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct PidSettings {
    pub pid: [f32; 3],
    pub target: f32,
    pub min: f32,
    pub max: f32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct AdcFilterSettings {
    pub odr: u32,
    pub order: u32,
    pub enhfilt: u32,
    pub enhfilten: u32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct PwmSettings {
    pub max_i_pos: f32,
    pub max_i_neg: f32,
    pub max_v: f32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct Settings {
    telemetry_period: f32,
    led: bool,
    dacs: [f32; 2],
    pidsettings: [PidSettings; 2],
    engage_iir: [bool; 2],
    adcsettings: AdcFilterSettings,
    pwmsettings: [PwmSettings; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_period: 1.0,
            led: false,
            dacs: [0.0, 0.0],
            engage_iir: [false, false],
            adcsettings: AdcFilterSettings {
                odr: 0b10000,   // 10Hz output data rate
                order: 0,       // Sinc5+Sinc1 filter
                enhfilt: 0b110, // 16.67 SPS, 92 dB rejection, 60 ms settling
                enhfilten: 1,   // enable postfilter
            },
            pwmsettings: [
                PwmSettings {
                    max_i_pos: 0.5,
                    max_i_neg: 0.5,
                    max_v: 0.5,
                },
                PwmSettings {
                    max_i_pos: 0.5,
                    max_i_neg: 0.5,
                    max_v: 0.5,
                },
            ],
            pidsettings: [
                PidSettings {
                    pid: [1.0, 0., 0.],
                    target: 22.0,
                    min: -SCALE,
                    max: SCALE,
                },
                PidSettings {
                    pid: [1.0, 0., 0.],
                    target: 22.0,
                    min: -SCALE,
                    max: SCALE,
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
        iirs: [[iir::IIR; IIR_CASCADE_LENGTH]; 2],
        #[init([[[0.; 5]; IIR_CASCADE_LENGTH]; 2])]
        iir_state: [[iir::Vec5; IIR_CASCADE_LENGTH]; 2],
        network: NetworkUsers<Settings, Telemetry>,
        settings: Settings,
        telemetry: Telemetry,
    }

    // #[init(schedule = [blink, poll_eth])]
    #[init(schedule = [blink, poll_eth, process, tele])]
    fn init(c: init::Context) -> init::LateResources {
        let mut thermostat = setup::setup(c.core, c.device);

        log::info!("setup done");

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
        c.schedule.poll_eth(c.start + 168000.cycles()).unwrap();
        c.schedule.tele(c.start + CYC_PER_S.cycles()).unwrap();

        // apply default settings
        thermostat.dacs.set(i_to_dac(settings.dacs[1]), 1);
        thermostat.dacs.set(i_to_dac(settings.dacs[0]), 0);
        thermostat.adc.set_filters(settings.adcsettings);
        thermostat
            .pwms
            .set_all(settings.pwmsettings[0], settings.pwmsettings[1]);

        log::info!("init done");
        init::LateResources {
            leds: thermostat.leds,
            adc: thermostat.adc,
            dacs: thermostat.dacs,
            pwms: thermostat.pwms,
            iirs: [[iir::IIR::new(1., -SCALE, SCALE); IIR_CASCADE_LENGTH]; 2],
            network,
            settings,
            telemetry: Telemetry::default(),
        }
    }

    #[task(priority=1, resources=[dacs, iir_state, iirs, telemetry, settings], schedule = [process])]
    fn process(c: process::Context, adcdata: [u32; 2]) {
        info!("adcdata:\t {:?}\t {:?}", adcdata[0], adcdata[1]);
        let dacs = c.resources.dacs;
        let iir_state = c.resources.iir_state;
        let iirs = c.resources.iirs;
        let telemetry = c.resources.telemetry;
        let settings = c.resources.settings;

        let mut yf: [f32; 2] = [0., 0.];

        for ch in 0..adcdata.len() {
            let y = iirs[ch]
                .iter()
                .zip(iir_state[ch].iter_mut())
                .fold(adcdata[ch] as f32, |yi, (iir_ch, state)| {
                    iir_ch.update(state, yi, false)
                });
            yf[ch] = y;
        }

        // convert to 18 bit fullscale output from 24 bit fullscale float equivalent. TODO rounding
        let yo0 = (yf[0] + SCALE) as u32 >> 6;
        let yo1 = (yf[1] + SCALE) as u32 >> 6;
        info!("yos:\t {:?}  {:?}", yo0, yo1);
        info!("yfs:\t {:?}  {:?}", yf[0], yf[1]);
        info!("y offset:\t {:?}", iirs[0][0].y_offset);

        if settings.engage_iir[0] {
            dacs.set(yo0, 0);
        }
        if settings.engage_iir[1] {
            dacs.set(yo1, 1);
        }

        // TODO: move this to the tele process
        // Wie geht das??: telemetry.dac = yf.iter().map(|x| i_to_dac(*x as f32) as f32).collect();
        telemetry.dac[0] = dac_to_i(dacs.val[0]);
        telemetry.dac[1] = dac_to_i(dacs.val[1]);
        telemetry.adc = [adc_to_temp(adcdata[0]), adc_to_temp(adcdata[1])];

        info!("dacdata:\t {:?}", dacs.val);
    }

    #[task(priority = 1, resources=[network, settings, dacs, adc, pwms, iirs])]
    fn settings_update(c: settings_update::Context) {
        log::info!("updating settings");
        let settings = c.resources.network.miniconf.settings();

        *c.resources.settings = *settings;

        c.resources
            .adc
            .set_filters(c.resources.settings.adcsettings);

        c.resources.pwms.set_all(
            c.resources.settings.pwmsettings[0],
            c.resources.settings.pwmsettings[1],
        );

        // c.resources.iirs[0][0].ba = [1.0, 0., 0., 0., 0.];
        c.resources.iirs[0][0].ba = pid_to_iir(c.resources.settings.pidsettings[0].pid);
        c.resources.iirs[0][0].set_x_offset(temp_to_iiroffset(
            c.resources.settings.pidsettings[0].target,
        ));

        // c.resources.iirs[0][0].y_offset =
        //     temp_to_iiroffset(c.resources.settings.pidsettings[0].target);

        info!(
            "target raw:\t {:?}",
            temp_to_iiroffset(c.resources.settings.pidsettings[0].target,)
        );
        // info!(
        //     "target:\t {:?}",
        //     adc_to_temp(c.resources.iirs[0][0].get_x_offset().unwrap() as u32)
        // );
        info!("y offset:\t {:?}", c.resources.iirs[0][0].y_offset);
        info!(
            "iir:\t {:?}",
            pid_to_iir(c.resources.settings.pidsettings[0].pid)
        );

        if !c.resources.settings.engage_iir[0] {
            c.resources
                .dacs
                .set(i_to_dac(c.resources.settings.dacs[0]), 0);
        }
        if !c.resources.settings.engage_iir[1] {
            c.resources
                .dacs
                .set(i_to_dac(c.resources.settings.dacs[1]), 1);
        }
    }

    #[task(priority = 1, resources = [network], schedule = [poll_eth],  spawn=[settings_update])]
    fn poll_eth(c: poll_eth::Context) {
        static mut NOW: u32 = 0;
        // log::info!("poll eth");

        match c.resources.network.update(*NOW) {
            NetworkState::SettingsChanged => c.spawn.settings_update().unwrap(),
            NetworkState::Updated => {}
            NetworkState::NoChange => {}
        }
        *NOW = *NOW + 1;
        c.schedule.poll_eth(c.scheduled + 168000.cycles()).unwrap();
    }

    #[idle(resources=[adc], spawn=[process])]
    fn idle(mut c: idle::Context) -> ! {
        let (mut adcdata0, mut adcdata1) = (0, 0);
        loop {
            let statreg = c.resources.adc.lock(|adc| adc.get_status_reg());
            if statreg != 0xff {
                let (adcdata, ch) = c.resources.adc.lock(|adc| adc.read_data());
                match ch {
                    0 => {
                        adcdata1 = adcdata;
                    } // ADC ch1 is thermostat ch0 for unknown reasons
                    _ => {
                        adcdata0 = adcdata;
                        c.spawn.process([adcdata0, adcdata1]).unwrap();
                    }
                }
            }
        }
    }

    #[task(priority = 1, resources = [network, telemetry, settings], schedule = [tele])]
    fn tele(c: tele::Context) {
        c.resources.network.telemetry.update();
        c.resources
            .network
            .telemetry
            .publish(&c.resources.telemetry);

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
