package masipam

import (
	"context"

	"github.com/containernetworking/cni/pkg/invoke"
)

// ExecCheck calls the IPAM plugin to check IP allocation
func ExecCheck(ipamType string, netconf []byte) error {
	// 委托执行 CHECK
	return invoke.DelegateCheck(context.Background(), ipamType, netconf, nil)
}
