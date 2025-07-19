#!/bin/bash
# This topo contains a single network namespace with two interfaces to enable MPTCP, used for testing.

# Clean stuff.
ip netns del client &> /dev/null
ip link del mptcp-client-11 &> /dev/null
ip link del mptcp-client-21 &> /dev/null

# Create netns.
ip netns add client

# Create the two links.
ip link add mptcp-client-11 type veth peer name mptcp-client-12
ip link set mptcp-client-11 up
ip link set mptcp-client-12 netns client up

ip link add mptcp-client-21 type veth peer name mptcp-client-22
ip link set mptcp-client-21 up
ip link set mptcp-client-22 netns client up

# Add IP addresses.
ip addr add 10.0.0.1/24 dev mptcp-client-11
ip netns exec client ip addr add 10.0.0.2/24 dev mptcp-client-12

ip addr add 10.1.0.1/24 dev mptcp-client-21
ip netns exec client ip addr add 10.1.0.2/24 dev mptcp-client-22

# Add default route.
ip netns exec client ip route add default dev mptcp-client-12

# Add latency on the first path.
tc qdisc add dev mptcp-client-11 root netem delay 2ms rate 10000mbit
ip netns exec client tc qdisc add dev mptcp-client-12 root netem delay 2ms rate 10000mbit

# Add latency and bandwidth limit on the second path.
tc qdisc add dev mptcp-client-21 root netem delay 2ms rate 1mbit
ip netns exec client tc qdisc add dev mptcp-client-22 root netem delay 2ms rate 1mbit