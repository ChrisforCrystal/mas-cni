package main

import (
	"fmt"
	"net"

	"github.com/containernetworking/cni/pkg/skel"
	"github.com/containernetworking/cni/pkg/types"
	current "github.com/containernetworking/cni/pkg/types/100"
	"github.com/containernetworking/cni/pkg/version"
	"github.com/containernetworking/plugins/pkg/ns"
	bv "github.com/containernetworking/plugins/pkg/utils/buildversion"
	"github.com/masallsome/mascni/pkg/config"
	"github.com/masallsome/mascni/pkg/masipam"
	"github.com/masallsome/mascni/pkg/masnet"
	"github.com/vishvananda/netlink"
)

func main() {
	skel.PluginMainFuncs(skel.CNIFuncs{
		Add:   cmdAdd,
		Check: cmdCheck,
		Del:   cmdDel,
	}, version.All, bv.BuildString("mascni"))
}


/**
{
  "cniVersion": "1.0.0",
  "name": "mascni-net",
  "type": "mascni",         // 对应我们的二进制文件名
  "bridge": "cni0",         // 我们的 PluginConf.Bridge
  "isGateway": true,        // 我们的 PluginConf.IsGateway
  "ipMasq": true,           // 我们的 PluginConf.IPMasq
  "ipam": {                 // 我们的 PluginConf.IPAM
    "type": "host-local",   // 委托给谁
    "subnet": "10.22.0.0/16",
    "gateway": "10.22.0.1",
    "routes": [
      { "dst": "0.0.0.0/0" }
    ]
  }
}
**/
func cmdAdd(args *skel.CmdArgs) error {
	// 从 Stdin 读取 JSON，解析出 Bridge 名字 (如 "cni0") 和 IPAM 配置
	conf, err := config.LoadConf(args.StdinData)
	if err != nil {
		return err
	}

	 // 步骤 1: 获取 NetNS (网络命名空间)
    // args.Netns 是容器的网络空间路径 (如 /var/run/netns/xxx)
    // 我们拿到这个句柄，才能把网卡塞进去
	netns, err := ns.GetNS(args.Netns)
	if err != nil {
		return fmt.Errorf("failed to open netns %q: %v", args.Netns, err)
	}
	defer netns.Close()

	// 步骤 2: 确保宿主机网桥存在
    // 调用 pkg/masnet/bridge.go -> EnsureBridge
    // 如果 cni0 不存在，就创建它；如果存在，就确保它是 UP 状态
	br, err := masnet.EnsureBridge(conf.Bridge, 1500) // Default MTU 1500 for now
	if err != nil {
		return err
	}

	// 步骤 3: 建立 Veth Pair (一根管子穿两头)
    // 调用 pkg/masnet/veth.go -> SetupVeth
    // 这是最关键的一步：
    // 1. 进入容器 NetNS，创建一堆 veth (eth0 <-> vethXXX)
    // 2. 把 vethXXX 挪回宿主机
    // 3. 把宿主机端的 vethXXX 插到网桥 (cni0) 上
	hostInterface, containerInterface, err := masnet.SetupVeth(netns, args.IfName, br, 1500)
	if err != nil {
		return err
	}

	// 4. IPAM
	// 步骤 4: 分配 IP (IPAM 委托)
	// 调用 pkg/masipam/ipam.go -> ExecAdd
	// 我们自己不算 IP，而是把锅甩给 "host-local" 插件
	// 它会返回给我们一个 IP (比如 10.244.0.5)
	//
	// 入参示例:
	// - ipamType: "host-local"
	// - args.StdinData: 完整的 CNI 配置 JSON (包含 ipam 字段)
	/*
		{
		  "cniVersion": "1.0.0",
		  "name": "mascni-net",
		  "type": "mascni",
		  "ipam": {
		    "type": "host-local",    <-- ipamType 从这里读取
		    "subnet": "10.22.0.0/16",
		    "routes": [{ "dst": "0.0.0.0/0" }]
		  },
		  ...
		}
	*/
	r, err := masipam.ExecAdd(conf.IPAM.Type, args.StdinData)
	if err != nil {
		return err
	}

	// Convert result to current version
	result, err := current.NewResultFromResult(r)
	if err != nil {
		return err
	}

	// 5. Configure Container IP/Routes
	if len(result.IPs) == 0 {
		return fmt.Errorf("IPAM returned no IPs")
	}
 	// 步骤 5: 配置容器内的 IP 和路由
    // 再次进入容器 NetNS，利用 netlink 把刚才申请到的 IP 贴在 eth0 上
    // 并设置默认路由走网关
	// Apply IP to container interface
	// 5. 配置容器内网络 (进入容器 NetNS)
	err = netns.Do(func(_ ns.NetNS) error {
		// 5.1. 找到容器内的网卡 (例如 eth0)
		link, err := netlink.LinkByName(containerInterface.Name)
		if err != nil {
			return fmt.Errorf("failed to lookup %q: %v", containerInterface.Name, err)
		}

		// 5.2. 配置 IP 地址
		// 我们遍历 IPAM 插件返回的 IP 列表 (IPv4/IPv6)，逐个绑到网卡上。
		// 相当于执行: ip addr add 10.244.0.5/24 dev eth0
		for _, ipc := range result.IPs {
			addr := &netlink.Addr{IPNet: &ipc.Address}
			if err := netlink.AddrAdd(link, addr); err != nil {
				return fmt.Errorf("failed to add IP %v to %q: %v", ipc.Address, containerInterface.Name, err)
			}
		}

		// 5.3. 配置路由
		// 我们遍历 IPAM 返回的路由表，逐个添加。
		// 最重要的是默认路由 (0.0.0.0/0 via 10.22.0.1)，让容器能访问外网。
		// 相当于执行: ip route add default via 10.22.0.1
		for _, r := range result.Routes {
			route := netlink.Route{
				Dst: &r.Dst,
				Gw:  r.GW,
			}
			if err := netlink.RouteAdd(&route); err != nil {
				// 注意：如果路由已存在 (幂等性)，netlink 会报错。
				// 在生产级代码中，我们应该检查错误类型是否为 "FileExists" 并忽略它。
				// 这里为了简化 MVP 逻辑，暂时忽略所有错误 (或者可以打印日志)
			}
		}

		// 5.4. 启动网卡
		// 相当于执行: ip link set eth0 up
		if err := netlink.LinkSetUp(link); err != nil {
			return fmt.Errorf("failed to set %q up: %v", containerInterface.Name, err)
		}

		return nil
	})
	if err != nil {
		return err
	}

	// 6. 如果配置了 IsGateway=true，我们需要把网关 IP 配置到宿主机网桥上
	if conf.IsGateway {
		for _, ipc := range result.IPs {
			// 通常网关 IP 在 IPAM 结果的 ips 列表中，但没有 Interface 索引（或者不重要）
			// 关键是看 Gateway 字段。所有的 Pod 实际上都指向同一个网关。
			// 我们只需要取第一个有效的网关 IP 配置到网桥上即可。
			if ipc.Gateway == nil {
				continue
			}

			gwIP := ipc.Gateway
			mask := ipc.Address.Mask // 使用和 Pod 相同的掩码

			// 构造网关的 IPNet 对象 (例如 10.99.0.1/16)
			gwAddr := &netlink.Addr{IPNet: &net.IPNet{
				IP:   gwIP,
				Mask: mask,
			}}

			// 查找宿主机网桥及其对应的 Netlink Link 对象
			// 注意：这里需要直接用 netlink 操作，而不是 net.Interface
			brLink, err := netlink.LinkByName(br.Name)
			if err != nil {
				return fmt.Errorf("failed to lookup bridge %q for gateway config: %v", br.Name, err)
			}

			// 尝试添加地址 (幂等操作: 如果已存在会报错，除非用 AddrReplace)
			// 为了简化，我们先 Add，如果报错 exists 则忽略
			if err := netlink.AddrAdd(brLink, gwAddr); err != nil {
				// 忽略 "file exists"
				// 更好的做法是检查 errno == EEXIST
				// 但在这个 MVP 里简单打印一下或者直接忽略
				// fmt.Printf("DEBUG: adding gw failed (likely exists): %v\n", err)
			}

			// 只配置第一个找到的网关即可 (通常只有一个 IPv4)
			break
		}
	}

	// 7. Populate result fields
	result.CNIVersion = current.ImplementedSpecVersion
	result.Interfaces = []*current.Interface{
		{
			Name: br.Name,
			Mac:  br.HardwareAddr.String(),
		},
		hostInterface,
		containerInterface,
	}

	// Link IPs to interfaces
	for _, ipc := range result.IPs {
		// Map to container interface (index 2 in our list)
		ipc.Interface = current.Int(2)
	}
    // 步骤 6: 告诉 Kubelet "完事了"
    // 打印 JSON 结果到 Stdout
	return types.PrintResult(result, conf.CNIVersion)
}

func cmdDel(args *skel.CmdArgs) error {
	conf, err := config.LoadConf(args.StdinData)
	if err != nil {
		return err
	}

	// 1. IPAM Release (Always try this even if netns is gone)
	if err := masipam.ExecDel(conf.IPAM.Type, args.StdinData); err != nil {
		return err
	}

	// 2. Cleanup Network (Veth)
	if args.Netns != "" {
		if err := masnet.TeardownVeth(args.Netns, args.IfName); err != nil {
			return err
		}
	}

	return nil
}

func cmdCheck(args *skel.CmdArgs) error {
	conf, err := config.LoadConf(args.StdinData)
	if err != nil {
		return err
	}

	// 1. IPAM Check
	if err := masipam.ExecCheck(conf.IPAM.Type, args.StdinData); err != nil {
		return err
	}

	// 2. Network Check (Optional for MVP: check bridge exists)
	if _, err := masnet.EnsureBridge(conf.Bridge, 1500); err != nil {
		return fmt.Errorf("bridge %q missing or invalid: %v", conf.Bridge, err)
	}

	return nil
}
