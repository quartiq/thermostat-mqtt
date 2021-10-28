// Thermostat DAC/TEC driver
//
// This file contains all of the drivers to convert an 18 bit word to an analog current.
// On Thermostat this used the ad5680 DAC and the MAX1968 PWM TEC driver. The (analog voltage)
// max output voltages/current settings are driven by PWMs of the STM32.
// SingularitySurfer 2021

use cortex_m::asm::delay;

use stm32_eth::hal::{
    gpio::{gpioc::*, gpioe::*, gpiof::*, Alternate, Output, PushPull, AF5},
    hal::{blocking::spi::Transfer, digital::v2::OutputPin, PwmPin},
    pwm::{self, PwmChannels},
    rcc::Clocks,
    spi,
    spi::{NoMiso, Spi},
    stm32::{SPI4, SPI5, TIM1, TIM3},
    time::{MegaHertz, U32Ext},
};

use crate::unit_conversion::{i_to_dac, i_to_pwm, v_to_pwm};

/// SPI Mode 1
pub const SPI_MODE: spi::Mode = spi::Mode {
    polarity: spi::Polarity::IdleLow,
    phase: spi::Phase::CaptureOnSecondTransition,
};

pub const SPI_CLOCK: MegaHertz = MegaHertz(30); // DAC SPI clock speed
pub const MAX_VALUE: u32 = 0x3FFFF; // Maximum DAC output value
pub const F_PWM: u32 = 20; // PWM freq in kHz

pub type Dac0Spi = Spi<SPI4, (PE2<Alternate<AF5>>, NoMiso, PE6<Alternate<AF5>>)>;

pub struct Dac0Pins {
    pub sck: PE2<Alternate<AF5>>,
    pub mosi: PE6<Alternate<AF5>>,
    pub sync: PE4<Output<PushPull>>,
}

pub type Dac1Spi = Spi<SPI5, (PF7<Alternate<AF5>>, NoMiso, PF9<Alternate<AF5>>)>;

pub struct Dac1Pins {
    pub sck: PF7<Alternate<AF5>>,
    pub mosi: PF9<Alternate<AF5>>,
    pub sync: PF6<Output<PushPull>>,
}

pub struct Pwms {
    pub max_v0: PwmChannels<TIM3, pwm::C1>,
    pub max_v1: PwmChannels<TIM3, pwm::C2>,
    pub max_i_pos0: PwmChannels<TIM1, pwm::C1>,
    pub max_i_pos1: PwmChannels<TIM1, pwm::C2>,
    pub max_i_neg0: PwmChannels<TIM1, pwm::C3>,
    pub max_i_neg1: PwmChannels<TIM1, pwm::C4>,
    pub shdn0: PE10<Output<PushPull>>,
    pub shdn1: PE15<Output<PushPull>>,
}

impl Pwms {
    pub fn new<M1, M2, M3, M4, M5, M6>(
        clocks: Clocks,
        tim1: TIM1,
        tim3: TIM3,
        max_v0: PC6<M1>,
        max_v1: PC7<M2>,
        max_i_pos0: PE9<M3>,
        max_i_pos1: PE11<M4>,
        max_i_neg0: PE13<M5>,
        max_i_neg1: PE14<M6>,
        shdn0: PE10<Output<PushPull>>,
        shdn1: PE15<Output<PushPull>>,
    ) -> Pwms {
        fn init_pwm_pin<P: PwmPin<Duty = u16>>(pin: &mut P) {
            pin.set_duty(0);
            pin.enable();
        }
        let channels = (max_v0.into_alternate_af2(), max_v1.into_alternate_af2());
        let (mut max_v0, mut max_v1) = pwm::tim3(tim3, channels, clocks, F_PWM.khz());
        init_pwm_pin(&mut max_v0);
        init_pwm_pin(&mut max_v1);

        let channels = (
            max_i_pos0.into_alternate_af1(),
            max_i_pos1.into_alternate_af1(),
            max_i_neg0.into_alternate_af1(),
            max_i_neg1.into_alternate_af1(),
        );
        let (mut max_i_pos0, mut max_i_pos1, mut max_i_neg0, mut max_i_neg1) =
            pwm::tim1(tim1, channels, clocks, F_PWM.khz());
        init_pwm_pin(&mut max_i_pos0);
        init_pwm_pin(&mut max_i_neg0);
        init_pwm_pin(&mut max_i_pos1);
        init_pwm_pin(&mut max_i_neg1);

        Pwms {
            max_v0,
            max_v1,
            max_i_pos0,
            max_i_pos1,
            max_i_neg0,
            max_i_neg1,
            shdn0,
            shdn1,
        }
    }

    /// Set PWM channel to relative dutycycle.
    pub fn set(&mut self, duty: f32, ch: u8) {
        fn set<P: PwmPin<Duty = u16>>(pin: &mut P, duty: f32) {
            let duty = i_to_pwm(duty);
            let max = pin.get_max_duty();
            let value = ((duty * (max as f32)) as u16).min(max);
            pin.set_duty(value);
        }
        match ch {
            0 => set(&mut self.max_v0, duty),
            1 => set(&mut self.max_v1, duty),
            2 => set(&mut self.max_i_pos0, duty),
            3 => set(&mut self.max_i_neg0, duty),
            4 => set(&mut self.max_i_pos1, duty),
            5 => set(&mut self.max_i_neg1, duty),
            _ => unreachable!(),
        }
    }

    /// set all PWM oututs to specified min/max currents
    pub fn set_all(
        &mut self,
        min_i0: f32,
        max_i0: f32,
        max_v0: f32,
        min_i1: f32,
        max_i1: f32,
        max_v1: f32,
    ) {
        self.set(v_to_pwm(max_v0), 0);
        self.set(v_to_pwm(max_v1), 1);
        self.set(i_to_pwm(max_i0), 2);
        self.set(i_to_pwm(min_i0), 3);
        self.set(i_to_pwm(max_i1), 4);
        self.set(i_to_pwm(min_i1), 5);
    }
}

/// DAC: https://www.analog.com/media/en/technical-documentation/data-sheets/AD5680.pdf
/// Peltier Driver: https://datasheets.maximintegrated.com/en/ds/MAX1968-MAX1969.pdf
pub struct Dacs {
    spi0: Dac0Spi,
    sync0: PE4<Output<PushPull>>,
    pub val: [u32; 2],
    spi1: Dac1Spi,
    sync1: PF6<Output<PushPull>>,
}

impl Dacs {
    pub fn new(clocks: Clocks, spi4: SPI4, spi5: SPI5, pins0: Dac0Pins, pins1: Dac1Pins) -> Self {
        let spi0 = Spi::spi4(
            spi4,
            (pins0.sck, NoMiso, pins0.mosi),
            SPI_MODE,
            SPI_CLOCK.into(),
            clocks,
        );
        let spi1 = Spi::spi5(
            spi5,
            (pins1.sck, NoMiso, pins1.mosi),
            SPI_MODE,
            SPI_CLOCK.into(),
            clocks,
        );

        let mut dacs = Dacs {
            spi0,
            sync0: pins0.sync,
            val: [0, 0],
            spi1,
            sync1: pins1.sync,
        };
        dacs.sync0.set_low().unwrap();
        dacs.sync1.set_low().unwrap();

        // default to zero amps
        dacs.set(i_to_dac(0.0), 0);
        dacs.set(i_to_dac(0.0), 1);
        dacs
    }

    /// Set the DAC output to value on a channel.
    pub fn set(&mut self, value: u32, ch: u8) {
        let value = value.min(MAX_VALUE);
        // 24 bit transfer. First 6 bit and last 2 bit are low.
        let mut buf = [(value >> 14) as u8, (value >> 6) as u8, (value << 2) as u8];
        if ch == 0 {
            self.sync0.set_high().unwrap();
            // must be high for >= 33 ns
            delay(100); // 100 * 5.95ns
            self.sync0.set_low().unwrap();
            self.spi0.transfer(&mut buf).unwrap();
            self.val[0] = value;
        } else {
            self.sync1.set_high().unwrap();
            // must be high for >= 33 ns
            delay(100); // 100 * 5.95ns
            self.sync1.set_low().unwrap();
            self.spi1.transfer(&mut buf).unwrap();
            self.val[1] = value;
        }
    }
}
