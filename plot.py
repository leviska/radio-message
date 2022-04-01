#!/usr/bin/python

import pandas as pd
import seaborn as sns

from matplotlib import pyplot as plt

df = pd.read_csv('log')

sns.set_theme(style='whitegrid')

for column in ['received', 'transmitted', 'avg_time']:
    sns.boxplot(x='sent', y=column, hue='algorithm', data=df) \
       .get_figure().savefig('out/{}.png'.format(column))
    plt.clf()
    for algo in ['dsdv', 'gossip']:
        sns.boxplot(x='sent', y=column, data=df[df.algorithm == algo]) \
           .get_figure().savefig('out/{}.{}.png'.format(column, algo))
        plt.clf()
