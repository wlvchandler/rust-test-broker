#!/bin/bash

if [ "$EUID" -ne 0 ]; then
    echo "need to be root"
    exit 1
fi

echo "Reverting system tuning..."

for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    echo powersave > $cpu 2>/dev/null || true
done

echo always > /sys/kernel/mm/transparent_hugepage/enabled
echo always > /sys/kernel/mm/transparent_hugepage/defrag
sysctl -w vm.swappiness=60
sysctl -w kernel.numa_balancing=1

sysctl -w net.core.rmem_max=212992
sysctl -w net.core.wmem_max=212992
sysctl -w net.ipv4.tcp_rmem='4096 87380 6291456'
sysctl -w net.ipv4.tcp_wmem='4096 16384 4194304'
sysctl -w net.ipv4.tcp_no_delay=0
sysctl -w net.core.netdev_budget=300

systemctl enable irqbalance 2>/dev/null || true
systemctl start irqbalance 2>/dev/null || true

sysctl -w kernel.sched_min_granularity_ns=2250000
sysctl -w kernel.sched_wakeup_granularity_ns=3000000
sysctl -w kernel.sched_migration_cost_ns=500000

echo
echo "done"
