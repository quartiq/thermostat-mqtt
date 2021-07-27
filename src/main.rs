#![no_std]
#![no_main]



use panic_halt as _;
use log::{error, info, warn};

use crate::{
    leds::Leds,
};

mod leds;
// mod shared;
// mod mod;

mod setup;

use rtic::cyccnt::{Instant, U32Ext as _};

const PERIOD: u32 = 1<<25;


#[rtic::app(device = stm32_eth::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        leds: Leds,
    }

    #[init(schedule = [blink])]
    fn init(c: init::Context) -> init::LateResources {

        let (mut leds, mut network_devices) = setup::setup(c.core, c.device);

        c.schedule.blink(c.start + PERIOD.cycles()).unwrap();


        init::LateResources {
            leds: leds
        }
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

    extern "C" {
        fn EXTI0();
    }
};
