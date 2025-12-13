<script>
    /**
     * MetricGraph - Real-time chart component for system metrics.
     *
     * Uses Chart.js to render line/area charts with live updates.
     * Designed for 1Hz metric streams from the backend WebSocket.
     */
    import { onMount, onDestroy, untrack } from 'svelte';
    import Chart from 'chart.js/auto';

    let {
        title = "Metric",
        labels,
        datasets,
        yMin = 0,
        yMax = null,
        formatValue = (v) => v,
        type = 'line',
        stacked = false,
    } = $props();

    let canvas;
    let chart = null;

    function updateChart() {
        if (!chart) return;

        // Update labels
        chart.data.labels = [...labels];

        // Sync datasets
        while (chart.data.datasets.length > datasets.length) {
            chart.data.datasets.pop();
        }

        datasets.forEach((ds, i) => {
            const data = ds.data ? [...ds.data] : [];
            if (chart.data.datasets[i]) {
                chart.data.datasets[i].data = data;
                chart.data.datasets[i].label = ds.label;
                chart.data.datasets[i].borderColor = ds.color;
                chart.data.datasets[i].backgroundColor = ds.fillColor || `${ds.color}33`;
                chart.data.datasets[i].fill = ds.fill ?? false;
                chart.data.datasets[i].stack = ds.stack;
            } else {
                chart.data.datasets.push({
                    label: ds.label,
                    data: data,
                    borderColor: ds.color,
                    backgroundColor: ds.fillColor || `${ds.color}33`,
                    fill: ds.fill ?? false,
                    stack: ds.stack
                });
            }
        });

        chart.update('none');
    }

    onMount(() => {
        if (!canvas) return;

        chart = new Chart(canvas.getContext('2d'), {
            type: type,
            data: { labels: [], datasets: [] },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                animation: false,
                interaction: { mode: 'index', intersect: false },
                plugins: {
                    legend: {
                        display: true,
                        position: 'top',
                        labels: { boxWidth: 10, font: { size: 10 } }
                    },
                    tooltip: {
                        callbacks: {
                            label: (context) => {
                                let lbl = context.dataset.label || '';
                                if (lbl) lbl += ': ';
                                if (context.parsed.y !== null) {
                                    lbl += formatValue(context.parsed.y);
                                }
                                return lbl;
                            }
                        }
                    }
                },
                scales: {
                    x: {
                        display: true,
                        stacked: stacked,
                        grid: { display: true, color: '#e5e7eb' },
                        ticks: { maxTicksLimit: 8, maxRotation: 0 }
                    },
                    y: {
                        display: true,
                        stacked: stacked,
                        beginAtZero: true,
                        min: yMin,
                        max: yMax,
                        grid: { color: '#d1d5db' },
                        ticks: { callback(value) { return formatValue(value); } }
                    }
                },
                elements: {
                    point: { radius: 0, hitRadius: 10, hoverRadius: 4 },
                    line: { borderWidth: 2, tension: 0.3 }
                }
            }
        });

        // Initial update
        updateChart();
    });

    onDestroy(() => {
        if (chart) {
            chart.destroy();
            chart = null;
        }
    });

    // Watch for prop changes - Svelte 5 tracks these as the component re-renders
    $effect.pre(() => {
        // Read the reactive values to establish tracking
        const _labels = labels;
        const _datasets = datasets;
        const _labelsLen = labels?.length ?? 0;
        const _ds0Len = datasets?.[0]?.data?.length ?? 0;

        // Update chart with new data
        untrack(() => updateChart());
    });
</script>

<div class="bg-bg-card p-4 rounded shadow border border-border-main flex flex-col h-64">
    <h3 class="text-sm font-bold text-text-muted mb-2">{title}</h3>
    <div class="flex-1 relative min-h-0">
        <canvas bind:this={canvas}></canvas>
    </div>
</div>
