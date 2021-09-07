#![no_std]
#![no_main]

use log::{error, info, warn};
use panic_halt as _;

use crate::leds::Leds;

mod leds;
mod network_users;
mod telemetry;
use network_users::{NetworkState, NetworkUsers, UpdateState};
use telemetry::Telemetry;

mod adc;
mod dac;
mod setup;

use stm32_eth;

use stm32_eth::stm32::Peripherals;

use rtic::cyccnt::{Instant, U32Ext as _};

use smoltcp_nal::smoltcp;

// pub mod messages;
// pub mod miniconf_client;
pub mod shared;
// pub mod configuration;

pub use miniconf::Miniconf;
pub use serde::Deserialize;

const PERIOD: u32 = 1 << 25;

#[derive(Copy, Clone, Debug, Deserialize, Miniconf)]
pub struct Settings {
    /// Configure the LED
    led: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { led: false }
    }
}

#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        leds: Leds,
        // network: NetworkUsers<Settings, Telemetry>,
        // settings: Settings,
        // telemetry: Telemetry,
    }

    // #[init(schedule = [blink, poll_eth])]
    #[init(schedule = [blink])]
    fn init(c: init::Context) -> init::LateResources {
        let (mut thermostat) = setup::setup(c.core, c.device);


        log::info!("setup done");

        // let mut network = NetworkUsers::new(
        //     thermostat.network_devices.stack,
        //     env!("CARGO_BIN_NAME"),
        //     thermostat.network_devices.mac_address,
        //     option_env!("BROKER")
        //         .unwrap_or("10.42.0.1")
        //         .parse()
        //         .unwrap(),
        // );

        // log::info!("Network users done");

        // let settings = Settings::default();

        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();
        // c.schedule.poll_eth(c.start + 168000.cycles()).unwrap();

        log::info!("init done");
        init::LateResources {
            leds: thermostat.leds,
            // network,
            // settings,
            // telemetry: Telemetry::default(),
        }
    }

    // #[idle(resources=[network], spawn=[settings_update])]
    // fn idle(mut c: idle::Context) -> ! {
    //     loop {
    //         match c.resources.network.lock(|net| net.update()) {
    //             NetworkState::SettingsChanged => {
    //                 c.spawn.settings_update().unwrap()
    //             }
    //             NetworkState::Updated => {}
    //             NetworkState::NoChange => cortex_m::asm::wfi(),
    //         }
    //     }
    // }

    // #[task(priority = 1, resources=[network, settings])]
    // fn settings_update(mut c: settings_update::Context) {
    //     log::info!("updating settings");
    //     let settings = c.resources.network.miniconf.settings();

    //     // c.resources.settings.lock(|current| *current = *settings);
    //     *c.resources.settings = *settings;
    // }

    // #[task(priority = 1, resources = [network], schedule = [poll_eth],  spawn=[settings_update])]
    // fn poll_eth(c: poll_eth::Context) {
    //     static mut NOW: u32 = 0;
    //     // log::info!("poll eth");

    //     match c.resources.network.update(*NOW) {
    //         NetworkState::SettingsChanged => c.spawn.settings_update().unwrap(),
    //         NetworkState::Updated => {}
    //         NetworkState::NoChange => {}
    //     }
    //     *NOW = *NOW + 1;
    //     c.schedule.poll_eth(c.scheduled + 168000.cycles()).unwrap();
    //     // log::info!("poll eth done");
    // }

    // #[task(resources = [leds, network, telemetry, settings], schedule = [blink])]
    #[task(priority = 1, resources = [leds], schedule = [blink])]
    fn blink(c: blink::Context) {
        static mut LED_STATE: bool = false;

        if *LED_STATE {
            c.resources.leds.g3.off();
            *LED_STATE = false;
            log::info!("led off");
        } else {
            c.resources.leds.g3.on();
            *LED_STATE = true;
            log::info!("led on");
        }
        // c.resources.telemetry.led = c.resources.settings.led;

        // if c.resources.telemetry.led {
        //     c.resources.leds.g4.on();
        // } else {
        //     c.resources.leds.g4.off();
        // }

        // c.resources.network.telemetry.update();

        // c.resources
        //     .network
        //     .telemetry
        //     .publish(&c.resources.telemetry);
        c.schedule.blink(c.scheduled + PERIOD.cycles()).unwrap();
    }

    // #[task(binds = ETH, priority = 1)]
    // fn eth(_: eth::Context) {
    //     let p = unsafe { Peripherals::steal() };
    //     stm32_eth::eth_interrupt_handler(&p.ETHERNET_DMA);
    // }

    extern "C" {
        fn EXTI0();
    }
};
