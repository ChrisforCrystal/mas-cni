package masipam

import (
	"context"
	"os"
	"path/filepath"

	"github.com/containernetworking/cni/pkg/invoke"
	"github.com/containernetworking/cni/pkg/types"
)

// ExecAdd calls the IPAM plugin to allocate an IP
// ExecAdd 调用外部 IPAM 插件 (如 host-local) 来申请 IP 地址。
// CNI 的委托机制允许我们复用现有的 IP 管理能力，而不需要自己造轮子。
func ExecAdd(ipamType string, netconf []byte) (types.Result, error) {
	// 2. 执行委托 (Delegate)
	// 本质上是 fork/exec 执行那个二进制文件，并把我们的 Stdin (配置) 和环境变传给它
	// 注意：CNI >= 0.8.0 的 DelegateAdd 第一个参数接受的是 *plugin name* (例如 "host-local")
	// 而不是绝对路径。invoke 库内部会自己再找一次。
	return invoke.DelegateAdd(context.Background(), ipamType, netconf, nil)
}

// ExecDel 调用外部 IPAM 插件释放 IP 地址。
func ExecDel(ipamType string, netconf []byte) error {
	// 委托执行 DEL
	return invoke.DelegateDel(context.Background(), ipamType, netconf, nil)
}

// locations 返回 CNI 插件的查找路径
func locations() []string {
	// 优先检查环境变量 CNI_PATH (多个路径用冒号分隔)
	// 默认路径为 /opt/cni/bin (这是所有 Kubernetes 节点的标准路径)
	paths := filepath.SplitList(os.Getenv("CNI_PATH"))
	if len(paths) == 0 {
		paths = []string{"/opt/cni/bin"}
	}
	return paths
}
