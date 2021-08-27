// Thermostat ADC driver 
// (AD7172 https://www.analog.com/media/en/technical-documentation/data-sheets/AD7172-2.pdf)
// SingularitySurfer 2021

use core::fmt;
use log::{error, info, warn};
use byteorder::{BigEndian, ByteOrder};

use stm32_eth::hal::{
    gpio::{gpiob::*, Alternate, GpioExt, Output, PushPull, AF5},
    hal::{blocking::spi::Transfer, digital::v2::OutputPin},
    rcc::Clocks,
    spi,
    spi::Spi,
    stm32::SPI2,
    time::MegaHertz,
};

/// SPI Mode 3
pub const SPI_MODE: spi::Mode = spi::Mode {
    polarity: spi::Polarity::IdleHigh,
    phase: spi::Phase::CaptureOnSecondTransition,
};

pub const SPI_CLOCK: MegaHertz = MegaHertz(2);


// ADC Register Adresses
const ID:u32 = 0x7;
const ADCMODE:u32 = 0x1;
const IFMODE:u32 = 0x2;
const DATA:u32 = 0x44;


pub type AdcSpi = Spi<
    SPI2,
    (
        PB10<Alternate<AF5>>,
        PB14<Alternate<AF5>>,
        PB15<Alternate<AF5>>,
    ),
    >;
    
    pub struct Adc_pins {
        pub sck: PB10<Alternate<AF5>>,
        pub miso: PB14<Alternate<AF5>>,
        pub mosi: PB15<Alternate<AF5>>,
        pub sync: PB12<Output<PushPull>>,
    }
    
    pub struct Adc {
        spi: AdcSpi,
        sync: PB12<Output<PushPull>>,
    }

    impl Adc {
        pub fn new(clocks: Clocks, spi2: SPI2, mut pins: Adc_pins) -> Self {
            pins.sync.set_high();
            let spi = Spi::spi2(
                spi2,
                (pins.sck, pins.miso, pins.mosi),
                SPI_MODE,
                SPI_CLOCK.into(),
                clocks,
            );
            let mut adc = Adc {
                spi,
                sync: pins.sync,
            };
        
        adc.reset();
        
        let before = adc.read_reg(0x02, 2);
        // adc.write_reg(0x02, 2, before|0x80);    // set continuous read bit

        info!("filt con: {:#X}", adc.read_reg(0x28, 2));

        let before = adc.read_reg(0x28, 2);
        adc.write_reg(0x28, 2, (before&0xffe0)|0x16);    // set data rate to 1/16th

        info!("filt con: {:#X}", adc.read_reg(0x28, 2));

        adc.print_continuous_conversion();


        loop{
            let mut a = adc.get_status_reg();
            info!("din: {:#X}", a);
            if(a != 0xff){
                info!("dinnnnnnnnnnnnnnnnnnnnn: {:#X}", a);
            }
            info!("din: {:#X}", adc.get_status_reg());
            info!("din: {:#X}", adc.read_reg(0x44, 3));

        }


        


        
        adc
    }

    pub fn reset(&mut self) {
        let mut buf = [0xFFu8; 8];
        self.sync.set_low();
        let result = self.spi.transfer(&mut buf);
        self.sync.set_high();
        match result {
            Err(e) => {
                warn!("ADC reset failed! {:?}", e)
            }
            Ok(_) => {
                info!("ADC reset succeeded")
            }
        };
    }

    fn print_continuous_conversion(&mut self){
        loop{
            let mut statreg = 0xff;
            while (statreg==0xff){
                statreg = self.get_status_reg();
            }
            info!("statreg: {:#X}", self.get_status_reg());
            info!("data: {:#X}", self.read_reg(0x44, 3));
        }
    }

    fn read_reg(&mut self, addr: u8, size: u8) -> u32 {
        let mut addr_buf = [addr|0x40];
        let _ = self.sync.set_low();
        let _ = self.spi.transfer(&mut addr_buf);
        let data = match size{
            1 => {let mut buf = [0];
                let raw = self.spi.transfer(&mut buf);
                raw.unwrap()[0].clone() as u32
            }
            2 => {let mut buf = [0,0];
                let raw = self.spi.transfer(&mut buf);
                BigEndian::read_u16(raw.unwrap()) as u32
            }
            3 => {let mut buf = [0,0,0];
                let raw = self.spi.transfer(&mut buf);
                BigEndian::read_u24(raw.unwrap()) as u32
            }
            4 => {let mut buf = [0,0,0,0];
                let raw = self.spi.transfer(&mut buf);
                BigEndian::read_u32(raw.unwrap()) as u32
            }
            _ => 0
        };
        let _ = self.sync.set_high();
        return data
    }

    fn write_reg(&mut self, addr: u8, size: u8, data: u32) {
        let mut addr_buf = [addr];
        let _ = self.sync.set_low();
        let _ = self.spi.transfer(&mut addr_buf);
        match size{
            1 => {let mut buf = [data as u8];
                let _ = self.spi.transfer(&mut buf);
            }
            2 => {let mut buf = [0,0];
                BigEndian::write_u16(&mut buf, data as u16);
                let _ = self.spi.transfer(&mut buf);
            }
            3 => {let mut buf = [0,0,0];
                BigEndian::write_u24(&mut buf, data as u32);
                let _ = self.spi.transfer(&mut buf);
            }
            4 => {let mut buf = [0,0,0,0];
                BigEndian::write_u32(&mut buf, data as u32);
                let _ = self.spi.transfer(&mut buf);
            }
            _ => {}
        };
    }

    fn get_status_reg(&mut self) -> u8 {
        let mut addr_buf = [0];
        let _ = self.sync.set_low();
        let _ = self.spi.transfer(&mut addr_buf);
        let _ = self.sync.set_high();
        addr_buf[0]

    }
}
