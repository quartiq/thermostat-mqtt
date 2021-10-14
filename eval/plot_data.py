#!/usr/bin/python3

import matplotlib.pyplot as plt
import csv

f = open('adcvals.csv', 'w')
data = csv.reader(f, delimiter=',')
