#!/bin/bash

echo "Before adding primary address"
sudo ip netns exec client ip link set mptcp-client-12 up
echo "After adding primary address"
echo "Before adding mptcp principal endpoint"
sudo ip netns exec client ip mptcp endpoint add 10.0.0.2 dev mptcp-client-12 id 5 subflow
echo "After adding mptcp principal endpoint"

echo "Before adding default route"
sleep 0.1
sudo ip netns exec client ip route add default dev mptcp-client-12
echo "After adding default route"