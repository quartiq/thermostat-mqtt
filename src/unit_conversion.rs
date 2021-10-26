// Thermostat unit and data description conversion
// SingularitySurfer 2021

use core::f32;
use num_traits::float::Float;

// ADC constants
const GAIN: u32 = 0x555555; // default ADC gain from datasheet
const R_INNER: f32 = 2.0 * 5100.0; // ratiometric resistor setup. 5.1k high and low side.

// Steinhart-Hart Parameters
const ZEROK: f32 = 273.15; // 0°C in °K
const B: f32 = 3988.0; // NTC beta value
const T_N_INV: f32 = 1.0 / (25.0 + ZEROK); // T_n = 25°C
const R_N: f32 = 10000.0; // TEC resistance at 25°C

// PWM constants
const MAXV: f32 = 5.0; // maximum voltage configurable for TEC driver
const MAXI: f32 = 3.0; // maximum current configurable for TEC driver

// DAC constants
const R_SENSE: f32 = 0.05; // TEC current sense resistor
pub const VREF_TEC: f32 = 1.5; // TEC driver reference voltage
const MAXCODE: f32 = (1 << 18) as _; // maximum DAC dataword
const VREF_OS: f32 = 0.025; // Device specific offset voltage for zero current at half dac scale
pub const VREF_DAC: f32 = 3.0 + VREF_OS; // DAC reference voltage target plus offset
const DATAWIDTH_GAIN: f32 = 0.015625; // 2**-6 LSB to LSB gain from 24 to 18 bit

// IIR constants
const SCALE: f32 = (1 << 23) as _; // half the ADC maximum dataword

pub fn adc_to_temp(adc: u32) -> f32 {
    // raw to R
    let data = (adc as f32) * (0.5 * 0x400000 as f32 / GAIN as f32);
    let vin = data as f32 / (0.75 * SCALE);
    let r = (R_INNER as f32) / ((1.0 / vin) - 1.0);

    // R to T (°C)
    let t_inv = T_N_INV + (1.0 / B) * (r / R_N).ln();
    (1.0 / t_inv) - ZEROK
}

pub fn i_to_dac(i: f32) -> u32 {
    let v = (i * 10.0 * R_SENSE) + VREF_TEC;
    ((v * MAXCODE) / VREF_DAC) as u32
}

pub fn dac_to_i(val: u32) -> f32 {
    let v = VREF_DAC * (val as f32 / MAXCODE);
    (v - VREF_TEC) / (10.0 * R_SENSE)
}

pub fn i_to_pwm(i: f32) -> f32 {
    i / MAXI
}

pub fn v_to_pwm(v: f32) -> f32 {
    v / MAXV
}

pub fn temp_to_iiroffset(temp: f32) -> f32 {
    // T (°C) to R
    let t_inv = 1.0 / (temp + ZEROK);
    let r = R_N * (B * (t_inv - T_N_INV)).exp();

    // R to raw
    let v = r / (R_INNER + r);
    let data = 0.75 * SCALE * v;
    (-data * GAIN as f32) / (0.5 * 0x400000 as f32)
}

pub fn pid_to_iir(pid: [f32; 3]) -> [f32; 5] {
    let kp = pid[0] * DATAWIDTH_GAIN;
    let ki = pid[1] * DATAWIDTH_GAIN;
    let kd = pid[2] * DATAWIDTH_GAIN;
    //PID
    if (ki > f32::EPSILON) & (kd > f32::EPSILON) {
        [kp + ki + kd, -(kp + 2.0 * kd), kd, 1.0, 0.0]
    }
    // PI
    else if ki > f32::EPSILON {
        [kp + ki, -kp, 0.0, 1.0, 0.0]
    }
    // PD
    else if kd > f32::EPSILON {
        [kd + kp, -kd, 0.0, 0.0, 0.0]
    }
    // P
    else {
        [kp, 0.0, 0.0, 0.0, 0.0]
    }
}
