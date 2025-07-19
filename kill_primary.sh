#!/bin/bash

echo "Before killing principal interface"
sudo ip netns exec client ip link set mptcp-client-12 down
echo "After killing principal interface"
echo "Before removing principal mptcp endpoint"
sudo ip netns exec client ip mptcp endpoint del id 5
echo "After removing principal mptcp endpoint"