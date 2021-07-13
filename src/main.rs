#![no_std]
#![no_main]

use panic_abort as _;
use log::{error, info, warn};

use cortex_m_rt::entry;
use stm32f4xx_hal::{
    gpio::GpioExt,
    rcc::RccExt,
    stm32::{CorePeripherals, Peripherals, SCB},
    time::{U32Ext, MegaHertz},
};

use crate::{
    leds::Leds,
};

mod leds;

const HSE: MegaHertz = MegaHertz(8);

use rtic::cyccnt::{Instant, U32Ext as _};

const PERIOD: u32 = 1<<25;

#[rtic::app(device = stm32f4xx_hal::stm32, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {

    struct Resources {
        leds: Leds,
    }

    #[init(schedule = [blink])]
    fn init(c: init::Context) -> init::LateResources {

        let mut cp = c.core;
        cp.SCB.enable_icache();
        cp.SCB.enable_dcache(&mut cp.CPUID);
        cp.DCB.enable_trace();
        cp.DWT.enable_cycle_counter();

        let dp: stm32f4xx_hal::stm32::Peripherals = c.device;

        let _clocks = dp.RCC.constrain()
            .cfgr
            .use_hse(HSE)
            .sysclk(168.mhz())
            .hclk(168.mhz())
            .pclk1(32.mhz())
            .pclk2(64.mhz())
            .freeze();

        let gpiod = dp.GPIOD.split();

        let mut leds = Leds::new(gpiod.pd9, gpiod.pd10.into_push_pull_output(), gpiod.pd11.into_push_pull_output());

        leds.r1.on();
        leds.g3.on();
        leds.g4.off();

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
        } else {
            c.resources.leds.g3.on();
            *LED_STATE = true;
        }
        c.schedule.blink(c.scheduled + PERIOD.cycles()).unwrap();

    }

    extern "C" {
        fn EXTI0();
    }
};
