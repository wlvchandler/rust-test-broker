sudo sysctl -w vm.nr_hugepages=1024
sudo cpupower frequency-set -g performance

# isolate CPU cores (todo - add to kernel boot parameters)
# isolcpus=1,2 nohz_full=1,2 rcu_nocbs=1,2
