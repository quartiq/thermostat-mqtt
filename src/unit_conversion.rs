// Thermostat unit and description conversion

use core::f32;
use log::{error, info, warn};
use num_traits::float::Float;

// ADC constants
const GAIN: u32 = 0x555555;
const R_INNER: f32 = 2.0 * 5100.0;

// Steinhart-Hart Parameters
const ZEROK: f32 = 273.15;
const B: f32 = 3988.0;
const T_N_INV: f32 = 1.0 / (25.0 + ZEROK); // T_n = 25°C
const R_N: f32 = 10000.0;

// PWM constants
const MAXV: f32 = 4.0 * 3.3;
const MAXI: f32 = 3.0;

// DAC constants
const R_SENSE: f32 = 0.05;
const VREF_TEC: f32 = 1.5;
const MAXCODE: f32 = 262144.0;
const VREF_DAC: f32 = 3.0;

// IIR constants
const SCALE: u32 = 1 << 23;

pub fn adc_to_temp(adc: u32) -> f32 {
    // raw to R
    let data = (adc as f32) * (0.5 * 0x400000 as f32 / GAIN as f32);
    let vin = data as f32 / (0.75 * SCALE as f32);
    let r = (R_INNER as f32) / ((1.0 / vin) - 1.0);

    // R to T (°C) (https://www.ametherm.com/thermistor/ntc-thermistors-steinhart-and-hart-equation)
    let t_inv = T_N_INV + (1.0 / B) * (r / R_N).ln();
    ((1.0 / t_inv) - ZEROK) as f32
}

pub fn i_to_dac(i: f32) -> u32 {
    let v = (i * 10.0 * R_SENSE) + VREF_TEC;
    log::info!("{:?}", v);
    ((v * MAXCODE) / VREF_DAC) as u32
}

pub fn dac_to_i(val: u32) -> f32 {
    let v = VREF_DAC * (val as f32 / MAXCODE);
    ((v - VREF_TEC) / (10.0 * R_SENSE)) as f32
}

pub fn i_to_pwm(i: f32) -> f32 {
    MAXI / i
}

pub fn v_to_pwm(v: f32) -> f32 {
    MAXV / v
}

pub fn temp_to_iiroffset(temp: f32) -> f32 {
    // T (°C) to R (https://www.ametherm.com/thermistor/ntc-thermistors-steinhart-and-hart-equation)
    let t_inv = 1.0 / (temp + ZEROK);
    let r = R_N * (B * (t_inv - T_N_INV)).exp();

    // R to raw
    let v = r / (R_INNER + r);
    let data = 0.75 * SCALE as f32 * v;
    (-data * GAIN as f32) / (0.5 * 0x400000 as f32)
}

pub fn pid_to_iir(pid: [f32; 3]) -> [f32; 5] {
    //PID
    if (pid[1] > f32::EPSILON) & (pid[2] > f32::EPSILON) {
        [
            pid[0] + pid[1] + pid[2],
            -(pid[0] + 2.0 * pid[2]),
            pid[2],
            1.0,
            0.0,
        ]
    }
    // PI
    else if pid[1] > f32::EPSILON {
        [pid[0] + pid[1], -pid[0], 0.0, 1.0, 0.0]
    }
    // PD
    else if pid[2] > f32::EPSILON {
        [pid[2] + pid[0], -pid[2], 0.0, 0.0, 0.0]
    }
    // P
    else {
        [pid[0], 0.0, 0.0, 0.0, 0.0]
    }
}
