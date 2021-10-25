#!/usr/bin/python3

# odr:1

import matplotlib.pyplot as plt
from matplotlib.mlab import psd
import numpy as np
import csv

T_SET = 25
F_S = 10

plt.rcParams.update({'font.size': 7})
box = dict(boxstyle='round', facecolor='white', alpha=1.0)


f = open('eval/fl_2.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

temp_spectrum, freqs = psd(
    np.array(temp)*1e6, len(temp)//8, Fs=F_S, detrend='mean')
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)


fig, axs = plt.subplots(1, 1)

ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="mid ki")
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


f = open('eval/fl_3.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

temp_spectrum, freqs = psd(
    np.array(temp)*1e6, len(temp)//8, Fs=F_S, detrend='mean')
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)


ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="low ki")
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


f = open('eval/fl_4.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

temp_spectrum, freqs = psd(
    np.array(temp)*1e6, len(temp)//8, Fs=F_S, detrend='mean')
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)


ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="high ki")
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


f = open('eval/fl_ol.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

temp_spectrum, freqs = psd(
    np.array(temp)*1e6, len(temp)//8, Fs=F_S, detrend='mean')
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)


ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="free running")
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


f = open('eval/10k_ref_short.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

temp_spectrum, freqs = psd(
    np.array(temp)*1e6, len(temp)//8, Fs=F_S, detrend='mean')
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)


ax = axs
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum,  linewidth=0.8, alpha=0.75, label="10k Ref")
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
