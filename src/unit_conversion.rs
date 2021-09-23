// Thermostat unit and description conversion

use log::{error, info, warn};
use num_traits::float::Float;

// ADC constants
const GAIN: u32 = 0x555555;
const R_INNER: f64 = 2.0 * 5100.0;
const VREF: f64 = 3.3;
// Steinhart-Hart Parameters
const A: f64 = 0.001125308852122;
const B: f64 = 0.000234711863267;
const C: f64 = 0.000000085663516;

// PWM constants
const MAXV: f64 = 4.0 * 3.3;
const MAXI: f64 = 3.0;

// DAC constants
const R_SENSE: f64 = 0.05;
const VREF_TEC: f64 = 1.5;
const MAXCODE: f64 = 262144.0;
const VREF_DAC: f64 = 3.0;

pub fn adc_to_temp(adc: u32) -> f32 {
    // raw to V
    let data = (adc as f64) * (0.5 * 0x400000 as f64 / GAIN as f64);
    let vin = data as f64 * VREF / (0.75 * (1 << 23) as f64);

    // V to R
    let r = (R_INNER as f64) / ((VREF as f64 / vin) - 1.0);

    // R to T (°C)
    let lnr = r.ln();
    let t_inv = A + B * lnr + C * lnr * lnr * lnr;
    ((1.0 / t_inv) - 273.15) as f32
}

pub fn i_to_dac(i: f64) -> u32 {
    let v = (i * 10.0 * R_SENSE) + VREF_TEC;
    log::info!("{:?}", v);
    ((v * MAXCODE) / VREF_DAC) as u32
}

pub fn dac_to_i(val: u32) -> f32 {
    let v = VREF_DAC * (val as f64 / MAXCODE);
    ((v - VREF_TEC) / (10.0 * R_SENSE)) as f32
}

pub fn i_to_pwm(i: f64) -> f64 {
    MAXI / i
}

pub fn v_to_pwm(v: f64) -> f64 {
    MAXV / v
}

pub fn temp_to_iiroffset(temp: f64) -> f64 {
    1.0
}

pub fn pid_to_iir(adc: u32) -> f64 {
    1.0
}
