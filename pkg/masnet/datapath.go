package masnet

import (
	"net"

	current "github.com/containernetworking/cni/pkg/types/100"
)

// Datapath defines the interface for underlying network operations.
// This allows us to swap implementations (e.g., Linux Bridge vs eBPF) later.
type Datapath interface {
	// EnsureBridge creates the bridge if it doesn't exist and returns the bridge object
	EnsureBridge(bridgeName string, mtu int) (net.Interface, error)

	// SetupVeth creates a veth pair, attaching one end to the bridge and moving the other to the container NS
	SetupVeth(netnsPath string, ifName string, bridgeName string, mtu int) (*current.Interface, *current.Interface, error)
}
