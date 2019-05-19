#!/usr/bin/python

from scipy import stats
import matplotlib.pyplot as plt
import numpy as np
import seaborn as sns
import sys

data = np.loadtxt(sys.argv[1])

print('samples', data.size)
print('quartiles', [np.percentile(data, 25), np.percentile(data, 50), np.percentile(data, 75)])
print('extremes', [np.amin(data), np.amax(data)])
print('std', round(np.std(data), 2))

# remove outliers (using Z-score) for the KDE plot
z = np.abs(stats.zscore(data))
plt.figure(dpi=96*2)
sns.kdeplot(data[(z < 3)])
plt.savefig('kde.png')
