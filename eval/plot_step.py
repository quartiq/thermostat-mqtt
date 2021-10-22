#!/usr/bin/python3

import matplotlib.pyplot as plt
from matplotlib.mlab import psd
import numpy as np
import csv

T_SET = 25

plt.rcParams.update({'font.size': 7})
box = dict(boxstyle='round', facecolor='white', alpha=1.0)


f = open('eval/turnon.csv', 'r')
data = csv.reader(f, delimiter=',', quoting=csv.QUOTE_NONNUMERIC)

temp = []
current = []

for row in data:
    temp.append(row[0])
    current.append(row[1] * 1000)

t = np.linspace(0, len(temp)/10, len(temp))

fig, axs = plt.subplots(1, 2)

ax = axs[0]
ax.plot(t, temp)
ax.set_title('Temperature')
ax.grid()
ax.set_xlabel('Time (seconds)')
ax.set_ylabel('°C')


ax = axs[1]
ax.plot(t, current)
ax.set_title('Current')
ax.grid()
ax.set_xlabel('Time (seconds)')
ax.set_ylabel('mA')

# plt.tight_layout()

plt.show()

# input()
