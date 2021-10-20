#![no_std]
#![no_main]

use cortex_m::asm::delay;
use log::{error, info, warn};
use panic_halt as _;

mod network_users;
mod telemetry;
mod unit_conversion;
use network_users::{NetworkState, NetworkUsers, UpdateState};
use telemetry::{Telemetry, TelemetryBuffer};
use unit_conversion::{
    adc_to_temp, dac_to_i, i_to_dac, pid_to_iir, temp_to_iiroffset, VREF_DAC, VREF_TEC,
};

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

const IIR_CASCADE_LENGTH: usize = 1;
const LED_PERIOD: u32 = 1 << 25;
const CYC_PER_S: u32 = 168_000_000; // clock is 168MHz
const SCALE: f32 = 8388608.0;
const OUTSCALE: f32 = 131072.0 * VREF_TEC / (VREF_DAC / 2.0); // zero current is slightly off center

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

    // #[init(schedule = [blink, poll_eth])]
    #[init(schedule = [blink, poll_eth, process, tele], spawn = [settings_update])]
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
        c.spawn.settings_update().unwrap();
        log::info!("init done");
        init::LateResources {
            leds: thermostat.leds,
            adc: thermostat.adc,
            dacs: thermostat.dacs,
            pwms: thermostat.pwms,
            iirs: [[iir::IIR::new(1., (-SCALE).into(), SCALE.into()); IIR_CASCADE_LENGTH]; 2],
            network,
            settings,
            telemetry: TelemetryBuffer::default(),
        }
    }

    #[task(priority=1, resources=[dacs, iir_state, iirs, telemetry, settings])]
    fn process(c: process::Context, adcdata: [u32; 2]) {
        info!("adcdata:\t {:?}\t {:?}", adcdata[0], adcdata[1]);
        let dacs = c.resources.dacs;
        let iir_state = c.resources.iir_state;
        let iirs = c.resources.iirs;
        let telemetry = c.resources.telemetry;
        let settings = c.resources.settings;

        let mut y: [i32; 2] = [0, 0];

        for ch in 0..adcdata.len() {
            y[ch] = iirs[ch]
                .iter()
                .zip(iir_state[ch].iter_mut())
                .fold(adcdata[ch] as f64, |yi, (iir_ch, state)| {
                    iir_ch.update(state, yi, false)
                }) as i32;
            if settings.engage_iir[ch] {
                dacs.set((y[ch] + OUTSCALE as i32) as u32, ch as u8);
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
            settings.pidsettings[0].max_i_neg * 1.1, // set to 10% higher than iir limits
            settings.pidsettings[0].max_i_pos * 1.1, // set to 10% higher than iir limits
            settings.max_v_tec[0],
            settings.pidsettings[1].max_i_neg * 1.1, // set to 10% higher than iir limits
            settings.pidsettings[1].max_i_pos * 1.1, // set to 10% higher than iir limits
            settings.max_v_tec[1],
        );

        for (i, iir) in c.resources.iirs.iter_mut().enumerate() {
            iir[0]
                .ba
                .iter_mut()
                .zip(pid_to_iir(settings.pidsettings[i].pid).iter())
                .map(|(d, x)| *d = *x as f64)
                .last();
            iir[0].set_x_offset(temp_to_iiroffset(settings.pidsettings[i].target) as f64); // set input offset to target
                                                                                           // iir[0].y_offset = iir[0].y_offset; // add output offset to half range
            iir[0].y_min = (i_to_dac(-settings.pidsettings[i].max_i_neg) as f32 - OUTSCALE) as f64;
            iir[0].y_max = (i_to_dac(settings.pidsettings[i].max_i_pos) as f32 - OUTSCALE) as f64;
            info!("y_min:\t {:?}  y_max:\t {:?}", iir[0].y_min, iir[0].y_max);
        }

        log::info!("ba: {:?}", c.resources.iirs[0][0].y_offset);
        // log::info!("x_offset: {:?}", c.resources.iirs[0][0].x_offset);

        for (i, eng) in settings.engage_iir.iter().enumerate() {
            if !*eng {
                c.resources.dacs.set(i_to_dac(settings.dacs[i]), i as u8);
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
                    }
                    _ => {
                        // ADC ch1 is thermostat ch0
                        adcdata0 = adcdata;
                        c.spawn.process([adcdata0, adcdata1]).unwrap();
                    }
                }
            }
        }
    }

    #[task(priority = 1, resources = [network, telemetry, settings], schedule = [tele])]
    fn tele(c: tele::Context) {
        // Wie geht das??: telemetry.dac.iter_mut().zip(yf.iter()).map(|&mut d, &x| *d = i_to_dac(x as f32) as f32).last();
        c.resources.network.telemetry.update();
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
