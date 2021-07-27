#![no_std]
#![no_main]



use panic_halt as _;
use log::{error, info, warn};

use crate::{
    leds::Leds,
};

mod leds;
mod cycle_counter;
// mod network_users;
// use network_users::NetworkUsers;

mod setup;

use stm32_eth;

use stm32_eth::{
    stm32::{Peripherals},
};

use rtic::cyccnt::{Instant, U32Ext as _};

use smoltcp_nal::smoltcp;

const PERIOD: u32 = 1<<25;


#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        leds: Leds,
        network: setup::NetworkDevices,
    }

    #[init(schedule = [blink])]
    fn init(c: init::Context) -> init::LateResources {

        let (mut leds, mut network_devices) = setup::setup(c.core, c.device);


        // let mut network = NetworkUsers::new(
        //     network_devices.stack,
        //     env!("CARGO_BIN_NAME"),
        //     network_devices.mac_address,
        // );

        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();

        init::LateResources {
            leds: leds,
            network: network_devices,
        }
    }

    // #[idle(resources=[network])]
    // fn idle(mut c: idle::Context) -> ! {
    //     loop {
    //         match c.resources.network.lock(|net| net.update()) {
    //             NetworkState::SettingsChanged => {
    //                 //c.spawn.settings_update().unwrap()
    //             }
    //             NetworkState::Updated => {}
    //             NetworkState::NoChange => cortex_m::asm::wfi(),
    //         }
    //     }
    // }

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
