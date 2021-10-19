#!/usr/bin/python3

import matplotlib.pyplot as plt
from matplotlib.mlab import psd
import numpy as np
import csv

T_SET = 25

plt.rcParams.update({'font.size': 7})
box = dict(boxstyle='round', facecolor='white', alpha=1.0)


f = open('eval/fiber_laser.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/3600, len(temp))

temp_psd, freqs = psd((np.array(temp) - T_SET), len(temp), Fs=1)
temp_psd = 10 * np.log10(temp_psd)
curr_psd, freqs = psd(current, len(temp), Fs=1)
curr_psd = 10 * np.log10(curr_psd)


fig, axs = plt.subplots(2, 2)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
e_max = max(abs((np.array(temp) - T_SET)))
text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.65, 0.95, text, transform=ax.transAxes,
        verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')

ax = axs[0][1]
ax.plot(freqs, temp_psd)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')


ax = axs[1][0]
ax.plot(t, current)
ax.set_title('TEC Current')
ax.grid()
mu = np.mean(current)
median = np.median(current)
v_rms = np.sqrt(np.mean((np.array(current))**2))
v_max = max(abs((np.array(current))))
v_min = min(abs((np.array(current))))
text = f'RMS: {v_rms:.7f}\nMax: {v_max:.7f}\nMin: {v_min:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.65, 0.05, text, transform=ax.transAxes,
        verticalalignment='bottom', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('mA')


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')


plt.tight_layout()
plt.show()