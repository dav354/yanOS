<script>
    import { onMount, onDestroy } from 'svelte';
    import Chart from 'chart.js/auto';

    let { 
        title = "Metric", 
        data = [], // Array of objects or values
        labels = [], // Array of time labels
        datasets = [], // Array of dataset configs { label, data, color, fill }
        yMin = 0,
        yMax = null,
        formatValue = (v) => v,
        type = 'line'
    } = $props();
    let { stacked = false } = $props();

    let canvas;
    let chart;

    function initChart() {
        if (!canvas) return;
        
        const ctx = canvas.getContext('2d');
        
        // Common options for Proxmox-like look
        const commonOptions = {
            responsive: true,
            maintainAspectRatio: false,
            animation: false, // Performance for live updates
            interaction: {
                mode: 'index',
                intersect: false,
            },
            plugins: {
                legend: {
                    display: true,
                    position: 'top',
                    labels: {
                        boxWidth: 10,
                        font: { size: 10 }
                    }
                },
                tooltip: {
                    callbacks: {
                        label: (context) => {
                            let label = context.dataset.label || '';
                            if (label) {
                                label += ': ';
                            }
                            if (context.parsed.y !== null) {
                                label += formatValue(context.parsed.y);
                            }
                            return label;
                        }
                    }
                }
            },
            scales: {
                x: {
                    display: true,
                    stacked,
                    grid: {
                        display: true,
                        color: '#e5e7eb'
                    },
                    ticks: {
                        maxTicksLimit: 8,
                        maxRotation: 0
                    }
                },
                y: {
                    display: true,
                    stacked,
                    beginAtZero: true,
                    min: yMin,
                    max: yMax,
                    grid: {
                        color: '#d1d5db'
                    },
                    ticks: {
                        callback(value) {
                            return formatValue(value);
                        }
                    }
                }
            },
            elements: {
                point: {
                    radius: 0, // Hide points for clean look
                    hitRadius: 10,
                    hoverRadius: 4
                },
                line: {
                    borderWidth: 2,
                    tension: 0.3 // Slight curve
                }
            }
        };

        chart = new Chart(ctx, {
            type: type,
            data: {
                labels: labels,
                datasets: datasets.map(ds => ({
                    label: ds.label,
                    data: ds.data,
                    borderColor: ds.color,
                    backgroundColor: ds.fillColor || `${ds.color}33`, // add alpha
                    fill: ds.fill ?? false,
                    stack: ds.stack
                }))
            },
            options: commonOptions
        });
    }

    $effect(() => {
        // Reactive update
        if (chart) {
            chart.data.labels = labels;
            chart.options.scales.x.stacked = stacked;
            chart.options.scales.y.stacked = stacked;
            if (chart.data.datasets.length !== datasets.length) {
                chart.data.datasets = datasets.map(ds => ({
                    label: ds.label,
                    data: ds.data,
                    borderColor: ds.color,
                    backgroundColor: ds.fillColor || `${ds.color}33`,
                    fill: ds.fill ?? false,
                    stack: ds.stack
                }));
            } else {
                chart.data.datasets.forEach((ds, i) => {
                    if (datasets[i]) {
                        ds.data = datasets[i].data;
                    }
                });
            }
            chart.update('none'); // 'none' mode for performance
        }
    });

    onMount(() => {
        initChart();
    });

    onDestroy(() => {
        if (chart) chart.destroy();
    });
</script>

<div class="bg-bg-card p-4 rounded shadow border border-border-main flex flex-col h-64">
    <h3 class="text-sm font-bold text-text-muted mb-2">{title}</h3>
    <div class="flex-1 relative min-h-0">
        <canvas bind:this={canvas}></canvas>
    </div>
</div>
