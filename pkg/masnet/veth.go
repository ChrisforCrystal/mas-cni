package masnet

import (
	"fmt"
	"net"

	current "github.com/containernetworking/cni/pkg/types/100"
	"github.com/containernetworking/plugins/pkg/ip"
	"github.com/containernetworking/plugins/pkg/ns"
	"github.com/vishvananda/netlink"
)

// SetupVeth creates the veth pair.
// Note: It MUST be run inside the container's network namespace (via ns.Do).
// SetupVeth 创建 Veth Pair 并连接容器与网桥。
// 注意：此函数使用了 netns.Do，会切换到容器的网络命名空间执行操作。
func SetupVeth(netns ns.NetNS, ifName string, bridge *net.Interface, mtu int) (*current.Interface, *current.Interface, error) {
	hostInterface := &current.Interface{}
	containerInterface := &current.Interface{}

	// 1. 进入容器网络命名空间 (相当于 ip netns exec <container_id> ...)
	err := netns.Do(func(hostNS ns.NetNS) error {
		// 1.1 在容器内创建 Veth Pair
		// ip.SetupVeth 是 CNI 插件库提供的辅助函数：
		// - 创建一对 Veth (ifName 和 随机名)
		// - 将 ifName (如 eth0) 留在当前命名空间 (容器内)
		// - 将另一端 (随机名，如 veth123) 移动到 hostNS (宿主机命名空间)
		hostVeth, containerVeth, err := ip.SetupVeth(ifName, mtu, "", hostNS)
		if err != nil {
			return fmt.Errorf("failed to setup veth: %v", err)
		}

		// 1.2 记录接口信息 (名字、MAC地址) 用于返回给 CNI 调用者
		hostInterface.Name = hostVeth.Name
		hostInterface.Mac = hostVeth.HardwareAddr.String()

		containerInterface.Name = containerVeth.Name
		containerInterface.Mac = containerVeth.HardwareAddr.String()
		containerInterface.Sandbox = netns.Path()

		return nil
	})
	if err != nil {
		return nil, nil, err
	}

	// 此时代码执行流已回到宿主机命名空间。

	// 2. 找到宿主机那一端的 Veth 网卡
	// 因为它刚才在容器里被创建并扔回了宿主机，我们需要根据名字找到它。
	vethLink, err := netlink.LinkByName(hostInterface.Name)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to lookup host veth %q: %v", hostInterface.Name, err)
	}

	// 3. 将宿主机端的 Veth 插到网桥上 (相当于 ip link set vethXXX master cni0)
	if err := netlink.LinkSetMaster(vethLink, &netlink.Bridge{LinkAttrs: netlink.LinkAttrs{Index: bridge.Index}}); err != nil {
		return nil, nil, fmt.Errorf("failed to attach veth %q to bridge: %v", hostInterface.Name, err)
	}

	return hostInterface, containerInterface, nil
}
