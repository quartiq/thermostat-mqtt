#!/usr/bin/python3

# odr:1

import matplotlib.pyplot as plt
from matplotlib.mlab import psd
import numpy as np
import csv

T_SET = 25
F_S = 10
REPS = 20

plt.rcParams.update({'font.size': 7})
box = dict(boxstyle='round', facecolor='white', alpha=1.0)
fig, axs = plt.subplots(1, 1)

temp_spectrum_sum = np.zeros(5001)


for rep in range(REPS):
    f = open(f'eval/repeated/fl_ol{rep}.csv', 'r')
    data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

    temp = []
    current = []

    for row in data:
        temp.append(row[0])
        current.append(row[1] * 1000)

    temp_spectrum, freqs = psd(
        np.array(temp)*1e6, len(temp), Fs=F_S//8, detrend='mean')

    # ax = axs
    # # ax.plot(freqs, temp_psd)
    # ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="mid ki")
    # ax.set_title('Temperature Error PSD')
    # ax.grid()
    # ax.set_xscale('log')
    # ax.set_yscale('log')
    # ax.set_xlabel('Frequency (hertz)')
    # ax.set_ylabel('uK^2 / Hz')
    # ax.set_xlim(10**-3, max(freqs))

    temp_spectrum_sum = temp_spectrum_sum + temp_spectrum


temp_spectrum = temp_spectrum_sum / REPS
ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="averaged")
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


plt.legend()
plt.tight_layout()
plt.show()
