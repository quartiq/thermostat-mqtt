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
    (np.array(temp)-T_SET)*1e6, len(temp), Fs=F_S)
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)
curr_psd, freqs = psd(current, len(temp), Fs=F_S)
curr_psd = 10 * np.log10(curr_psd)

fig, axs = plt.subplots(2, 2)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
print("mid:")
print(e_rms)
e_max = max(abs((np.array(temp) - T_SET)))
text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.65, 0.95, text, transform=ax.transAxes,
        verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')
ax.set_xlim(0, max(t))

ax = axs[0][1]
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


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
ax.set_xlim(0, max(t))


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('dB A^2 / Hz')

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
    (np.array(temp)-T_SET)*1e6, len(temp), Fs=F_S)
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)
curr_psd, freqs = psd(current, len(temp), Fs=F_S)
curr_psd = 10 * np.log10(curr_psd)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
print("mid:")
print(e_rms)
e_max = max(abs((np.array(temp) - T_SET)))
text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.65, 0.95, text, transform=ax.transAxes,
        verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')
ax.set_xlim(0, max(t))

ax = axs[0][1]
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


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
ax.set_xlim(0, max(t))


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('dB A^2 / Hz')

ax.set_xlim(10**-3, max(freqs))


f = open('eval/fl_4.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/(3600 * F_S), len(temp))

# temp_spectrum, freqs = psd(
#     (np.array(temp)-T_SET), len(temp), Fs=F_S)
temp_spectrum = np.abs(
    (np.fft.fft(np.array(temp)-T_SET)*1e6)**2 * len(temp)**-1)
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)
curr_psd, freqs = psd(current, len(temp), Fs=F_S)
curr_psd = 10 * np.log10(curr_psd)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.mean((np.array(temp) - T_SET)**2)
print("high:")
print(np.sqrt(e_rms))
print(temp_spectrum.sum()/len(temp_spectrum)**2)
e_max = max(abs((np.array(temp) - T_SET)))
text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
ax.text(0.65, 0.95, text, transform=ax.transAxes,
        verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')
ax.set_xlim(0, max(t))

ax = axs[0][1]
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum[:5001])
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


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
ax.set_xlim(0, max(t))


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('dB A^2 / Hz')

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
    (np.array(temp)-T_SET)*1e6, len(temp), Fs=F_S)
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)
curr_psd, freqs = psd(current, len(temp), Fs=F_S)
curr_psd = 10 * np.log10(curr_psd)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
e_max = max(abs((np.array(temp) - T_SET)))
# text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
# ax.text(0.65, 0.95, text, transform=ax.transAxes,
#         verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')
ax.set_xlim(0, max(t))

ax = axs[0][1]
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


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
ax.set_xlim(0, max(t))


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('dB A^2 / Hz')

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
    (np.array(temp)-T_SET)*1e6, len(temp), Fs=F_S)
temp_asd = 20 * np.log10(temp_spectrum)
temp_psd = 10 * np.log10(temp_spectrum)
curr_psd, freqs = psd(current, len(temp), Fs=F_S)
curr_psd = 10 * np.log10(curr_psd)

ax = axs[0][0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
mu = np.mean(temp)
median = np.median(temp)
e_rms = np.sqrt(np.mean((np.array(temp) - T_SET)**2))
e_max = max(abs((np.array(temp) - T_SET)))
# text = f'RMS Error: {e_rms:.7f}\nMax Error: {e_max:.7f}\nMean: {mu:.7f}\nMedian: {median:.7f}'
# ax.text(0.65, 0.95, text, transform=ax.transAxes,
#         verticalalignment='top', horizontalalignment='left', bbox=box)
ax.set_xlabel('Time (hours)')
ax.set_ylabel('°C')
ax.set_xlim(0, max(t))

ax = axs[0][1]
# ax.plot(freqs, temp_psd)
ax.plot(freqs, temp_spectrum)
ax.set_title('Temperature Error PSD')
ax.grid()
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('uK^2 / Hz')
ax.set_xlim(10**-3, max(freqs))


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
ax.set_xlim(0, max(t))


ax = axs[1][1]
ax.plot(freqs, curr_psd)
ax.set_title('TEC Current PSD')
ax.grid()
ax.set_xscale('log')
ax.set_xlabel('Frequency (hertz)')
ax.set_ylabel('dB A^2 / Hz')

ax.set_xlim(10**-3, max(freqs))


plt.tight_layout()
plt.show()
