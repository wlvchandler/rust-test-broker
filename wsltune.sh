#!/bin/bash

if [ "$EUID" -ne 0 ]; then
    echo "need to be root"
    exit 1
fi

echo "tuning..."
echo

# Memory 
sysctl -w vm.swappiness=0

sysctl -w vm.max_map_count=262144

# Network
sysctl -w net.core.rmem_max=2097152
sysctl -w net.core.wmem_max=2097152
sysctl -w net.ipv4.tcp_rmem='4096 87380 2097152'
sysctl -w net.ipv4.tcp_wmem='4096 87380 2097152'
sysctl -w net.core.netdev_budget=2000

# allow realtime scheduling
if command -v chrt >/dev/null 2>&1; then
    sysctl -w kernel.sched_rt_runtime_us=-1 2>/dev/null || true
fi

# wsl2 config suggestions
echo "in windows, add to %UserProfile%/.wslconfig :"
echo "[wsl2]
memory=8GB
processors=4
swap=0
localhostForwarding=true"

echo "Done"
echo "consider adding .wslconfig as mentioned above"
echo "run 'wsl --shutdown' and restart if needed"
echo
#echo "To monitor performance:"
#vmstat 1 1 | head -n 3
