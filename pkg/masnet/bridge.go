package masnet

import (
	"fmt"
	"net"

	"github.com/vishvananda/netlink"
)

// EnsureBridge 保证网桥存在并处于 UP 状态。
// 如果网桥已存在，通过。如果不存在，创建它。
func EnsureBridge(bridgeName string, mtu int) (*net.Interface, error) {
	// 1. 尝试查找网桥，看是否已经存在
	br, err := netlink.LinkByName(bridgeName)
	if err == nil {
		// 1.1 网桥已存在
		// 确保它是 UP 状态 (ip link set cni0 up)，否则数据包通不了
		if err := netlink.LinkSetUp(br); err != nil {
			return nil, fmt.Errorf("failed to set bridge %q up: %v", bridgeName, err)
		}
		// 返回 Go 标准库的 net.Interface 结构，供上层通用逻辑使用
		return &net.Interface{
			Index:        br.Attrs().Index,
			MTU:          br.Attrs().MTU,
			Name:         br.Attrs().Name,
			HardwareAddr: br.Attrs().HardwareAddr,
			Flags:        br.Attrs().Flags,
		}, nil
	}

	// 2. 如果网桥不存在 (LinkNotFoundError)，则创建它
	if _, ok := err.(netlink.LinkNotFoundError); ok {
		br := &netlink.Bridge{
			LinkAttrs: netlink.LinkAttrs{
				Name: bridgeName,
				MTU:  mtu,
			},
		}

		// 2.1 调用 Netlink 创建设备 (相当于 ip link add name cni0 type bridge)
		if err := netlink.LinkAdd(br); err != nil {
			return nil, fmt.Errorf("failed to create bridge %q: %v", bridgeName, err)
		}

		// 2.2 启动网桥 (相当于 ip link set cni0 up)
		if err := netlink.LinkSetUp(br); err != nil {
			return nil, fmt.Errorf("failed to set bridge %q up: %v", bridgeName, err)
		}

		// 2.3 重新获取网桥信息
		// 为什么要重新获取？因为创建时结构体里只有 Name/MTU，
		// 内核分配的 Index 和生成的 MAC 地址需要重新查出来。
		l, err := netlink.LinkByName(bridgeName)
		if err != nil {
			return nil, fmt.Errorf("failed to fetch newly created bridge %q: %v", bridgeName, err)
		}

		return &net.Interface{
			Index:        l.Attrs().Index,
			MTU:          l.Attrs().MTU,
			Name:         l.Attrs().Name,
			HardwareAddr: l.Attrs().HardwareAddr,
			Flags:        l.Attrs().Flags,
		}, nil
	}

	// 其他不可预知的错误
	return nil, fmt.Errorf("failed to check bridge existence: %v", err)
}
