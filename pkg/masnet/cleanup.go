package masnet

import (
	"github.com/containernetworking/plugins/pkg/ip"
	"github.com/containernetworking/plugins/pkg/ns"
)

// TeardownVeth deletes the veth pair.
// It tries to enter the container namespace and delete the interface.
// If the namespace doesn't exist, we assume it's already cleaned up.
// TeardownVeth 删除 Veth Pair。
// 它尝试进入容器的 NetNS 并删除那个接口。
// 如果 NetNS 已经不存在了，我们认为清理已经完成 (幂等性)。
func TeardownVeth(netnsPath string, ifName string) error {
	// 如果没有提供 NetNS 路径，甚至无法开始清理，直接返回成功 (幂等)
	if netnsPath == "" {
		return nil
	}

	// 尝试打开 NetNS
	netns, err := ns.GetNS(netnsPath)
	if err != nil {
		// 如果打开失败 (通常是因为 NetNS 文件已经被删除了)，这也算成功。
		// 因为我们的目标是“让它消失”，它既然已经消失了，任务就完成了。
		return nil
	}
	defer netns.Close()

	// 进入容器 NetNS 删除网卡
	return netns.Do(func(_ ns.NetNS) error {
		// 删除网卡接口
		// 这里我们忽略错误，因为如果接口不存在 (ip.DelLinkByName 返回错)，
		// 也意味着我们的目标“清除它”已经达成。
		_ = ip.DelLinkByName(ifName)
		return nil
	})
}
