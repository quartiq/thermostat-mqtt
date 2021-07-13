#!/usr/bin/env bash

set -e

cargo flash --elf target/thumbv7em-none-eabihf/release/mqtt-thermostat --chip STM32F427ZIT


