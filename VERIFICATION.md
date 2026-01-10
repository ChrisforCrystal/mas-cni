# mascni 验证指南 (macOS + Kind)

本文档详细说明如何在 macOS 环境下，利用 Kind (Kubernetes in Docker) 对 CNI 插件进行全流程验证。

## 1. 编译产物 (Build)

由于我们的目标运行环境是 Linux (Kind 所在的容器)，所以必须进行交叉编译。

在项目根目录下执行：

```bash
# 1. 编译插件本体 (mascni)
GOOS=linux go build -o bin/mascni cmd/mascni/main.go

# 2. 编译测试工具 (cnitool)
# 注意：cnitool 是官方提供的 CNI 命令行测试工具
GOOS=linux go build -o bin/cnitool github.com/containernetworking/cni/cnitool
```

## 2. 准备测试环境 (Environment)

我们需要将编译好的二进制文件“注入”到 Kind 的节点容器中。

```bash
# 1. 确保 Kind 正在运行
kind get clusters
# 如果没有集群，请运行: kind create cluster

# 2. 创建测试配置文件
# 将以下内容保存为 bin/test-net.conf
echo '{
  "cniVersion": "1.0.0",
  "name": "mascni-test",
  "type": "mascni",
  "bridge": "mascni0",
  "ipam": {
    "type": "host-local",
    "subnet": "10.99.0.0/16"
  }
}' > bin/test-net.conf

# 3. 将文件复制到 Kind 节点 (kind-control-plane)
# /opt/cni/bin 是 CNI 插件的标准存放路径
docker cp bin/mascni kind-control-plane:/opt/cni/bin/
docker cp bin/cnitool kind-control-plane:/opt/cni/bin/
docker cp bin/test-net.conf kind-control-plane:/etc/cni/net.d/99-test-mascni.conf
```

## 3. 执行验证 (Verification)

验证分为三步：ADD (创建), CHECK (检查), DEL (删除)。我们将使用 `docker exec` 在 Kind 节点内部执行这些操作。

### 第一步：ADD (创建网络)

创建一个网络命名空间 `manual-test` 模拟 Pod，并把网卡插进去。

```bash
# 1. 创建测试用的网络命名空间
docker exec kind-control-plane ip netns add manual-test

# 2. 执行 ADD
docker exec kind-control-plane bash -c "export CNI_PATH=/opt/cni/bin; export NETCONFPATH=/etc/cni/net.d; /opt/cni/bin/cnitool add mascni-test /var/run/netns/manual-test"
```

✅ **预期结果**：输出一段 JSON，其中 `ips` 字段应包含分配的 IP (如 `10.99.0.2`)。

### 第二步：CHECK (健康检查)

验证插件和 IPAM 状态。

```bash
docker exec kind-control-plane bash -c "export CNI_PATH=/opt/cni/bin; export NETCONFPATH=/etc/cni/net.d; /opt/cni/bin/cnitool check mascni-test /var/run/netns/manual-test"
```

✅ **预期结果**：无任何输出，且退出码为 0。

### 第三步：DEL (清理资源)

删除网卡并释放 IP。

```bash
docker exec kind-control-plane bash -c "export CNI_PATH=/opt/cni/bin; export NETCONFPATH=/etc/cni/net.d; /opt/cni/bin/cnitool del mascni-test /var/run/netns/manual-test"
```

✅ **预期结果**：无任何输出，且退出码为 0。

## 4. 故障排查 (Troubleshooting)

如果遇到问题，可以进入容器内部查看详情：

```bash
docker exec -it kind-control-plane bash
```

常用检查命令：

- 查看网桥是否创建：`ip link show mascni0`
- 查看 IPAM 数据：`ls -R /var/lib/cni/networks/mascni-test/` (host-local 插件存储 IP 分配记录的地方)
- 查看 CNI 目录：`ls -l /opt/cni/bin/`

### 常见问题 (FAQ)

**Q: Pod 有 IP 但是 Ping 不通网关 (10.99.0.1)?**
A: 这通常是因为宿主机的网桥 (`mascni0`) 虽然创建了，但没有配置 IP 地址。
CNI 插件不仅仅要配置容器内的 IP，如果是网关模式 (`isGateway: true`)，还必须把 Gateway IP 配置到宿主机网桥上。
检查命令：`ip addr show mascni0`。如果只有 MAC 地址没有 `inet` IP，就是这个问题。
我们已经在代码中修复了这个问题 (自动检测 `isGateway` 字段并配置网桥 IP)。

## 5. 进阶验证：真实 Pod 测试 (Real Kubernetes Pod)

既然我们已经验证了插件的基本功能，现在可以将它配置为 Kind 集群的默认 CNI 插件，并启动一个真实的 Pod 来验证。

### 步驟 1: 配置 Kubelet 使用我们的插件

我们需要把测试配置放到 Kind 节点的标准 CNI 配置路径下，并重启容器运行时（或等待它自动加载，通常 Kubelet 会监视这个目录）。

```bash
# 因为 Kind 启动时自带了默认 CNI (kindnet)，我们需要先把它的配置移走或覆盖
# 注意：这可能会暂时断开现有 Pod 的网络，但对于测试集群没问题

# 1. 备份原有的 CNI 配置 (如果有)
docker exec kind-control-plane mv /etc/cni/net.d/10-kindnet.conflist /etc/cni/net.d/10-kindnet.conflist.bak

# 2. 确保我们的配置是唯一的或者优先级最高的 (文件名按字母排序)
# 我们之前已经上传了 99-test-mascni.conf，为了确保它生效，我们可以重命名一下
docker exec kind-control-plane mv /etc/cni/net.d/99-test-mascni.conf /etc/cni/net.d/00-mascni.conf
```

### 步驟 2: 创建测试 Pod

在宿主机创建一个简单的 Pod 定义文件 `test-pod.yaml`：

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: mascni-pod
spec:
  containers:
    - name: busybox
      image: busybox
      command: ["sleep", "3600"]
```

然后部署它：

```bash
kubectl apply -f test-pod.yaml
```

### 步驟 3: 验证 IP 分配

等待 Pod 启动 (Running) 后，查看它的 IP 地址。

```bash
kubectl get pod mascni-pod -o wide
```

✅ **预期结果**：
你看到的 IP 应该是 `10.99.x.x` 网段的（根据我们的配置 `subnet: 10.99.0.0/16`），而不是 Kind 默认的网段。
如果能看到分配了我们期望的 IP，说明 **mascni 已经成功接管了 K8s 的网络！**

### 步驟 4: 验证连通性

进入 Pod 内部测试网络。

```bash
kubectl exec -it mascni-pod -- ip addr
# 应该能看到 eth0 和分配的 IP

kubectl exec -it mascni-pod -- ping 10.99.0.1
# 应该能 Ping 通网关 (cni0/mascni0)
```
