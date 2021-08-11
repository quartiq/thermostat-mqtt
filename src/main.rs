#![no_std]
#![no_main]



use panic_halt as _;
use log::{error, info, warn};

use crate::{
    leds::Leds,
};

mod leds;
// mod network_users;
// use network_users::NetworkUsers;

mod setup;

use stm32_eth;

use stm32_eth::{
    stm32::{Peripherals},
};

use rtic::cyccnt::{Instant, U32Ext as _};

use smoltcp_nal::smoltcp;

// pub mod messages;
// pub mod miniconf_client;
// pub mod shared;
// pub mod configuration;
//
// pub use miniconf;

const PERIOD: u32 = 1<<25;




#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        leds: Leds,
        network: setup::NetworkDevices,
    }

    #[init(schedule = [blink, poll_eth])]
    fn init(c: init::Context) -> init::LateResources {

        let (mut leds, mut network_devices) = setup::setup(c.core, c.device);

        log::info!("setup done");
        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();
        c.schedule.poll_eth(c.start + 168000.cycles()).unwrap();


        init::LateResources {
            leds: leds,
            network: network_devices,
        }
    }


    #[task(resources = [network], schedule = [poll_eth])]
    fn poll_eth(c: poll_eth::Context) {
        static mut NOW: u32 = 0;
        let updated = c.resources.network.stack.poll(*NOW);
        log::info!("{:?}", *NOW);
        log::info!("{:?}", updated);
        *NOW = *NOW + 1;
        c.schedule.poll_eth(c.scheduled + 168000.cycles()).unwrap();
    }


    #[task(resources = [leds], schedule = [blink])]
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
