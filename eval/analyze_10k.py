#!/usr/bin/python3

import matplotlib.pyplot as plt
from matplotlib.mlab import psd
import numpy as np
import csv

T_SET = 25

plt.rcParams.update({'font.size': 7})
box = dict(boxstyle='round', facecolor='white', alpha=1.0)


f = open('eval/10k_ref1.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])

t = np.linspace(0, len(temp)/3600, len(temp))

temp_psd, freqs = psd((np.array(temp) - T_SET),
                      len(temp), Fs=1, detrend='mean')
temp_psd = 10 * np.log10(temp_psd)

fig, axs = plt.subplots(2, 1)

ax = axs[0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
e_max = max(abs((np.array(temp) - T_SET)))
text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.75, 0.35, text, transform=ax.transAxes,
        verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')

ax = axs[1]
ax.plot(freqs, temp_psd)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')

plt.tight_layout()

plt.show()

# input()
