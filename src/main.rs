#![no_std]
#![no_main]

use cortex_m::asm::delay;
use log::{error, info, warn};
use panic_halt as _;

mod network_users;
mod telemetry;
use network_users::{NetworkState, NetworkUsers, UpdateState};
use telemetry::Telemetry;

mod adc;
mod dac;
mod iir;
mod leds;
mod setup;

use adc::Adc;
use dac::{Dacs, Pwms};
use iir::Iirs;
use leds::Leds;

use stm32_eth;

use stm32_eth::stm32::Peripherals;

use rtic::cyccnt::{Instant, U32Ext as _};

// use smoltcp_nal::smoltcp;

// pub mod messages;
// pub mod miniconf_client;
pub mod shared;
// pub mod configuration;

pub use miniconf::{Miniconf, MiniconfAtomic};
pub use serde::Deserialize;

const PERIOD: u32 = 1 << 25;
const CYC_PER_S: u32 = 168_000_000;   // clock is 168MHz

#[derive(Copy, Clone, Debug, Deserialize, MiniconfAtomic)]
pub struct Iirsettings {
    pub ba: [f64; 5],
    pub target: f64,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct Adcsettings {
    pub data_rate_setting: u32,
    pub filter_setting: u32,
}

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct Settings {
    telemetry_period: f64,
    led: bool,
    dacs: [u32; 2],
    engage_iir: [bool; 2],
    iirs: [Iirsettings; 2],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telemetry_period: 1.0,
            led: false,
            dacs: [1 << 17, 1 << 17],
            engage_iir: [false, false],
            iirs: [
                Iirsettings {
                    ba: [1.0, 0.0, 0.0, 0.0, 0.0],
                    target: 8300000.0,
                },
                Iirsettings {
                    ba: [1.0, 0.0, 0.0, 0.0, 0.0],
                    target: 8300000.0,
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
        iirs: Iirs,
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

        let iirs = Iirs::new();

        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();
        c.schedule.poll_eth(c.start + 168000.cycles()).unwrap();
        c.schedule.tele(c.start + CYC_PER_S.cycles()).unwrap();

        thermostat.dacs.set(settings.dacs[0], 0);
        thermostat.dacs.set(settings.dacs[1], 1);

        log::info!("init done");
        init::LateResources {
            leds: thermostat.leds,
            adc: thermostat.adc,
            dacs: thermostat.dacs,
            pwms: thermostat.pwms,
            iirs,
            network,
            settings,
            telemetry: Telemetry::default(),
        }
    }

    #[task(priority=1, resources=[dacs, iirs, telemetry, settings], schedule = [process])]
    fn process(c: process::Context, adcdata0: u32, adcdata1: u32) {
        let dacs = c.resources.dacs;
        let iirs = c.resources.iirs;
        let telemetry = c.resources.telemetry;
        let settings = c.resources.settings;

        let yf0 = iirs.iir0.tick(adcdata0 as f64);
        let yf1 = iirs.iir1.tick(adcdata1 as f64);

        // convert to 18 bit fullscale output from 24 bit fullscale float equivalent
        let yo0 = ((yf0 + 8388608.0) as u32) >> 6;
        let yo1 = ((yf1 + 8388608.0) as u32) >> 6;

        if settings.engage_iir[0] {
            dacs.set(yo0, 0);
        }
        if settings.engage_iir[1] {
            dacs.set(yo1, 1);
        }
        telemetry.dac[0] = dacs.val0;
        telemetry.dac[1] = dacs.val1;
        telemetry.adc = [adcdata0, adcdata1];
    }

    #[task(priority = 1, resources=[network, settings, iirs, dacs])]
    fn settings_update(c: settings_update::Context) {
        log::info!("updating settings");
        let settings = c.resources.network.miniconf.settings();

        // c.resources.settings.lock(|current| *current = *settings);
        *c.resources.settings = *settings;

        c.resources.iirs.iir0.ba = c.resources.settings.iirs[0].ba;
        c.resources.iirs.iir0.target = c.resources.settings.iirs[0].target;
        c.resources.iirs.iir1.ba = c.resources.settings.iirs[1].ba;
        c.resources.iirs.iir1.target = c.resources.settings.iirs[1].target;

        if !c.resources.settings.engage_iir[0] {
            c.resources.dacs.set(c.resources.settings.dacs[0], 0);
        }
        if !c.resources.settings.engage_iir[1] {
            c.resources.dacs.set(c.resources.settings.dacs[1], 1);
            log::info!("set dac1: {:?}", c.resources.settings.dacs[1]);
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
        // log::info!("poll eth done");
    }

    #[idle(resources=[adc], spawn=[process])]
    fn idle(c: idle::Context) -> ! {
        let (mut adcdata0, mut adcdata1) = (0, 0);
        loop {
            let statreg = c.resources.adc.get_status_reg();
            if statreg != 0xff {
                let (adcdata, ch) = c.resources.adc.read_data();
                match ch {
                    0 => {
                        adcdata1 = adcdata;
                    } // ADC ch1 is thermostat ch0 for unknown reasons
                    _ => {
                        adcdata0 = adcdata;
                        c.spawn.process(adcdata0, adcdata1).unwrap();
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
        
        c.schedule.tele(c.scheduled + ((c.resources.settings.telemetry_period * CYC_PER_S as f64) as u32).cycles()).unwrap();
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

        c.schedule.blink(c.scheduled + PERIOD.cycles()).unwrap();
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
