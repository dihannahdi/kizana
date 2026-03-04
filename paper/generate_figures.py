#!/usr/bin/env python3
"""
Generate publication-quality figures for the Kizana Q1 paper.
Run: pip install matplotlib numpy
      python generate_figures.py
"""
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

# Set publication style
plt.rcParams.update({
    'font.family': 'serif',
    'font.size': 11,
    'axes.labelsize': 12,
    'axes.titlesize': 13,
    'xtick.labelsize': 10,
    'ytick.labelsize': 10,
    'legend.fontsize': 9,
    'figure.dpi': 300,
    'savefig.dpi': 300,
    'savefig.bbox': 'tight',
    'savefig.pad_inches': 0.1,
})

# Color palette
COLORS = {
    'no_trans': '#DC2626',    # red
    'kizana': '#3B82F6',      # blue
    'optimized': '#10B981',   # green
    'gt': '#EA580C',          # orange
    'stemmer': '#8B5CF6',     # purple
    'other': '#6B7280',       # gray
}

# ============================================================
# Figure 1: Per-Language MAP Comparison (Grouped Bar Chart)
# ============================================================
def fig_per_language_map():
    fig, ax = plt.subplots(figsize=(8, 5))
    
    languages = ['Arabic', 'English', 'Indonesian', 'Mixed', 'Overall']
    systems = {
        'No Translation':  [0.833, 0.129, 0.027, 0.067, 0.139],
        'Kizana Full':     [0.838, 0.610, 0.776, 0.734, 0.740],
        'Kizana Optimized':[0.834, 0.699, 0.837, 0.733, 0.790],
        'GT + BM25':       [0.831, 0.726, 0.810, 0.845, 0.799],
    }
    colors = [COLORS['no_trans'], COLORS['kizana'], COLORS['optimized'], COLORS['gt']]
    
    x = np.arange(len(languages))
    width = 0.18
    offsets = [-1.5, -0.5, 0.5, 1.5]
    
    for i, (name, vals) in enumerate(systems.items()):
        bars = ax.bar(x + offsets[i] * width, vals, width, label=name, 
                      color=colors[i], edgecolor='white', linewidth=0.5, alpha=0.85)
        # Add value labels on top
        for bar, val in zip(bars, vals):
            if val > 0.15:
                ax.text(bar.get_x() + bar.get_width()/2., bar.get_height() + 0.01,
                        f'{val:.3f}', ha='center', va='bottom', fontsize=7, rotation=45)
    
    ax.set_ylabel('MAP')
    ax.set_xticks(x)
    ax.set_xticklabels(languages)
    ax.set_ylim(0, 1.05)
    ax.legend(loc='upper left', framealpha=0.9)
    ax.grid(axis='y', alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig1_per_language_map.png')
    fig.savefig('fig1_per_language_map.pdf')
    print('Saved fig1_per_language_map.png/pdf')
    plt.close()


# ============================================================
# Figure 2: Ablation Study - Horizontal Bar Chart
# ============================================================
def fig_ablation():
    fig, ax = plt.subplots(figsize=(8, 5))
    
    components = [
        ('$-$MultiVariant', +0.031),
        ('$-$Parent', 0.000),
        ('Raw BM25', -0.006),
        ('$-$Hierarchy', -0.006),
        ('$-$Diversity', -0.005),
        ('$-$BookPenalty', -0.004),
        ('$-$Phrases', -0.010),
        ('$+$Stemmer', -0.074),
        ('$-$Translation', -0.601),
    ]
    # Sort by delta (most positive first)
    components.sort(key=lambda x: x[1], reverse=True)
    
    names = [c[0] for c in components]
    deltas = [c[1] for c in components]
    colors_bar = [COLORS['optimized'] if d > 0 else (COLORS['no_trans'] if d < -0.05 else COLORS['other']) for d in deltas]
    
    y_pos = np.arange(len(names))
    bars = ax.barh(y_pos, deltas, color=colors_bar, edgecolor='white', linewidth=0.5, alpha=0.85)
    
    # Add value labels
    for bar, delta in zip(bars, deltas):
        x_pos = bar.get_width()
        ha = 'left' if delta >= 0 else 'right'
        offset = 0.005 if delta >= 0 else -0.005
        ax.text(x_pos + offset, bar.get_y() + bar.get_height()/2., 
                f'{delta:+.3f}', ha=ha, va='center', fontsize=9, fontweight='bold')
    
    ax.set_yticks(y_pos)
    ax.set_yticklabels(names, fontsize=10)
    ax.set_xlabel('ΔMAP from Full System')
    ax.axvline(x=0, color='black', linewidth=0.8)
    ax.set_xlim(-0.65, 0.08)
    ax.grid(axis='x', alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig2_ablation.png')
    fig.savefig('fig2_ablation.pdf')
    print('Saved fig2_ablation.png/pdf')
    plt.close()


# ============================================================
# Figure 3: Optimization Progression
# ============================================================
def fig_optimization():
    fig, ax = plt.subplots(figsize=(8, 5))
    
    configs = ['Full\nSystem', '$-$MV', '$-$MV\n$-$Div', '$-$MV\n$-$BP', '$-$MV$-$Div\n$-$BP\n(Optimized)']
    map_vals = [0.740, 0.771, 0.772, 0.789, 0.790]
    ndcg5_vals = [0.465, 0.488, 0.494, 0.505, 0.513]
    ndcg10_vals = [0.541, 0.559, 0.568, 0.591, 0.593]
    
    x = np.arange(len(configs))
    
    ax.plot(x, map_vals, 'o-', color=COLORS['kizana'], linewidth=2.5, markersize=10, label='MAP', zorder=3)
    ax.plot(x, ndcg10_vals, 's-', color=COLORS['optimized'], linewidth=2, markersize=8, label='NDCG@10', zorder=3)
    ax.plot(x, ndcg5_vals, '^-', color=COLORS['gt'], linewidth=2, markersize=8, label='NDCG@5', zorder=3)
    
    # GT+BM25 reference line
    ax.axhline(y=0.799, color=COLORS['no_trans'], linewidth=1.5, linestyle='--', alpha=0.7, label='GT+BM25 MAP')
    
    # Annotations
    for i, (m, n5, n10) in enumerate(zip(map_vals, ndcg5_vals, ndcg10_vals)):
        ax.annotate(f'{m:.3f}', (x[i], m), textcoords="offset points", xytext=(0, 12), 
                    ha='center', fontsize=8, color=COLORS['kizana'])
    
    ax.set_xticks(x)
    ax.set_xticklabels(configs, fontsize=9)
    ax.set_ylabel('Score')
    ax.set_ylim(0.35, 0.85)
    ax.legend(loc='lower right', framealpha=0.9)
    ax.grid(alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig3_optimization.png')
    fig.savefig('fig3_optimization.pdf')
    print('Saved fig3_optimization.png/pdf')
    plt.close()


# ============================================================
# Figure 4: Per-Domain Performance Radar/Bar Chart
# ============================================================
def fig_per_domain():
    fig, ax = plt.subplots(figsize=(7, 5))
    
    domains = ['Ibadah\n(n=46)', 'Muamalat\n(n=23)', 'Munakahat\n(n=17)', 'Aqidah\n(n=10)']
    map_vals = [0.730, 0.703, 0.887, 0.612]
    mrr_vals = [0.741, 0.720, 0.897, 0.596]
    ndcg5_vals = [0.469, 0.377, 0.573, 0.469]
    
    x = np.arange(len(domains))
    width = 0.25
    
    ax.bar(x - width, map_vals, width, label='MAP', color=COLORS['kizana'], alpha=0.85, edgecolor='white')
    ax.bar(x, mrr_vals, width, label='MRR', color=COLORS['optimized'], alpha=0.85, edgecolor='white')
    ax.bar(x + width, ndcg5_vals, width, label='NDCG@5', color=COLORS['gt'], alpha=0.85, edgecolor='white')
    
    # Value labels
    for i in range(len(domains)):
        ax.text(x[i] - width, map_vals[i] + 0.01, f'{map_vals[i]:.3f}', ha='center', va='bottom', fontsize=8)
        ax.text(x[i], mrr_vals[i] + 0.01, f'{mrr_vals[i]:.3f}', ha='center', va='bottom', fontsize=8)
        ax.text(x[i] + width, ndcg5_vals[i] + 0.01, f'{ndcg5_vals[i]:.3f}', ha='center', va='bottom', fontsize=8)
    
    ax.set_ylabel('Score')
    ax.set_xticks(x)
    ax.set_xticklabels(domains)
    ax.set_ylim(0, 1.0)
    ax.legend(loc='upper right', framealpha=0.9)
    ax.grid(axis='y', alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig4_per_domain.png')
    fig.savefig('fig4_per_domain.pdf')
    print('Saved fig4_per_domain.png/pdf')
    plt.close()


# ============================================================
# Figure 5: System Comparison Summary (Radar/Spider Chart)
# ============================================================
def fig_system_comparison():
    fig, ax = plt.subplots(figsize=(7, 5))
    
    metrics = ['MAP', 'MRR', 'NDCG@5', 'NDCG@10', 'P@5', 'P@10']
    
    systems = {
        'No Translation':  [0.139, 0.140, 0.123, 0.127, 0.110, 0.096],
        'Kizana Full':     [0.740, 0.749, 0.465, 0.541, 0.704, 0.692],
        'Kizana Optimized':[0.790, 0.794, 0.513, 0.593, 0.758, 0.747],
        'GT + BM25':       [0.799, 0.876, 0.580, 0.649, 0.750, 0.733],
    }
    
    x = np.arange(len(metrics))
    width = 0.2
    offsets = [-1.5, -0.5, 0.5, 1.5]
    colors = [COLORS['no_trans'], COLORS['kizana'], COLORS['optimized'], COLORS['gt']]
    
    for i, (name, vals) in enumerate(systems.items()):
        bars = ax.bar(x + offsets[i] * width, vals, width, label=name,
                      color=colors[i], edgecolor='white', linewidth=0.5, alpha=0.85)
    
    ax.set_ylabel('Score')
    ax.set_xticks(x)
    ax.set_xticklabels(metrics)
    ax.set_ylim(0, 1.0)
    ax.legend(loc='upper left', framealpha=0.9, fontsize=8)
    ax.grid(axis='y', alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig5_system_comparison.png')
    fig.savefig('fig5_system_comparison.pdf')
    print('Saved fig5_system_comparison.png/pdf')
    plt.close()


# ============================================================
# Figure 6: Language-Specific Improvement from Optimization
# ============================================================
def fig_optimization_by_lang():
    fig, ax = plt.subplots(figsize=(7, 4.5))
    
    languages = ['Arabic', 'English', 'Indonesian', 'Mixed', 'Overall']
    full_map = [0.838, 0.610, 0.776, 0.734, 0.740]
    opt_map = [0.834, 0.699, 0.837, 0.733, 0.790]
    deltas = [o - f for f, o in zip(full_map, opt_map)]
    pct = [(o - f) / f * 100 for f, o in zip(full_map, opt_map)]
    
    x = np.arange(len(languages))
    width = 0.35
    
    bars1 = ax.bar(x - width/2, full_map, width, label='Full System', color=COLORS['kizana'], alpha=0.85, edgecolor='white')
    bars2 = ax.bar(x + width/2, opt_map, width, label='Optimized', color=COLORS['optimized'], alpha=0.85, edgecolor='white')
    
    # Add delta labels
    for i in range(len(languages)):
        color = COLORS['optimized'] if deltas[i] > 0 else COLORS['no_trans']
        sign = '+' if deltas[i] > 0 else ''
        ax.text(x[i] + width/2, opt_map[i] + 0.01, f'{sign}{pct[i]:.1f}%',
                ha='center', va='bottom', fontsize=8, color=color, fontweight='bold')
    
    ax.set_ylabel('MAP')
    ax.set_xticks(x)
    ax.set_xticklabels(languages)
    ax.set_ylim(0, 1.0)
    ax.legend(framealpha=0.9)
    ax.grid(axis='y', alpha=0.3)
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    
    fig.savefig('fig6_optimization_by_lang.png')
    fig.savefig('fig6_optimization_by_lang.pdf')
    print('Saved fig6_optimization_by_lang.png/pdf')
    plt.close()


if __name__ == '__main__':
    print('Generating publication figures for Kizana Q1 paper...\n')
    fig_per_language_map()
    fig_ablation()
    fig_optimization()
    fig_per_domain()
    fig_system_comparison()
    fig_optimization_by_lang()
    print('\nAll figures generated successfully!')
    print('PNG files for preview, PDF files for LaTeX inclusion.')
