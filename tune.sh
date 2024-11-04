#!/bin/bash
#sudo sysctl -w vm.nr_hugepages=1024
#sudo cpupower frequency-set -g performance
# isolate CPU cores (todo - add to kernel boot parameters)
# isolcpus=1,2 nohz_full=1,2 rcu_nocbs=1,2

# Must run as root
if [ "$EUID" -ne 0 ]; then
    echo "need to be root"
    exit 1
fi


# set cpu gov to perf mode
for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    echo performance > $cpu 2>/dev/null || true
done

# Disable CPU power saving
echo 0 > /sys/module/intel_idle/parameters/max_cstate 2>/dev/null || true
echo 1 > /proc/sys/kernel/timer_migration 2>/dev/null || true

# disable transparent huge pages
echo never > /sys/kernel/mm/transparent_hugepage/enabled
echo never > /sys/kernel/mm/transparent_hugepage/defrag

sysctl -w vm.swappiness=0
sysctl -w kernel.numa_balancing=0

#network
sysctl -w net.core.rmem_max=2097152
sysctl -w net.core.wmem_max=2097152
sysctl -w net.ipv4.tcp_rmem='4096 87380 2097152'
sysctl -w net.ipv4.tcp_wmem='4096 87380 2097152'
sysctl -w net.ipv4.tcp_no_delay=1
sysctl -w net.core.netdev_budget=2000

# irq
systemctl stop irqbalance 2>/dev/null || true
systemctl disable irqbalance 2>/dev/null || true

# kernel
sysctl -w kernel.sched_min_granularity_ns=10000000
sysctl -w kernel.sched_wakeup_granularity_ns=15000000
sysctl -w kernel.sched_migration_cost_ns=5000000

# wsl2 :)
if grep -qi microsoft /proc/version; then
    echo "=== WSL2-Specific Settings ==="
    echo "Detected WSL2 environment..."
    echo "Optimizing WSL2 memory..."
    sysctl -w vm.max_map_count=262144
fi

echo "=== Current CPU Settings ==="
cat /sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq 2>/dev/null || echo "Cannot read CPU frequency"

echo
echo "system tuned for low latency"
echo
echo "other stuff to try"
echo "- vmstat 1        (memory/cpu)"
echo "- mpstat -P ALL 1 (per cpu)"
echo "- iostat -x 1     (io)"
