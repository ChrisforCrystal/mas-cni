# Rust CNI + eBPF 手动验证手册 (Manual Verification Guide)

这份文档汇总了从源码构建到在 Kind 集群中验证完整功能的全部步骤。不需要依赖任何本地环境（只需要 Docker）。

## 1. 准备工作

确保你处于项目根目录：

```bash
cd /Users/jiwn2/dev/masallsome/mascni
# 或者你的实际路径
```

确保 Kind 集群正在运行，并且 Control Plane 容器名为 `kind-control-plane`：

```bash
docker ps --filter "name=kind-control-plane"
```

## 2. 构建产物 (Build Artifacts)

### 2.1 编译 eBPF 对象文件 (`tc_redirect.o`)

使用 Docker 里的 Ubuntu 环境来编译 C 代码，确保能找到 Linux 内核头文件。

````bash
```bash
# ==============================================================================
# eBPF 编译命令 (Cross-Compilation Helper)
# ------------------------------------------------------------------------------
# 目的：使用 Docker 作为一个一致的编译环境，将 C 源码编译成 BPF 字节码 (.o)。
# 难点：我们需要在 MacOS (ARM64) 上操作，但需生成兼容 Linux 内核的 BPF 字节码，
#      且编译过程中需要引用 Linux 内核头文件 (linux-libc-dev)。
# ==============================================================================
docker run --rm \
    -v $(pwd):/src \
    -w /src \
    --platform linux/amd64 \
    ubuntu:latest \
    /bin/bash -c \
    "apt-get update && \
     # 1. 安装 LLVM/Clang (BPF编译器) 和 Linux 头文件
     apt-get install -y clang llvm libbpf-dev linux-libc-dev gcc-multilib && \
     \
     # 2. 修复头文件路径问题 (Hack)
     # Clang 有时找不到 <asm/types.h>，因为 Ubuntu 把它们放到了 x86_64-linux-gnu 目录下。
     # 这里我们手动建个软链，帮编译器找到头文件。
     ln -s /usr/include/x86_64-linux-gnu/asm /usr/include/asm && \
     \
     # 3. 执行编译
     # -O2:         必须开启优化 (BPF Verifier 要求)。
     # -target bpf: 生成 BPF 架构的字节码。
     clang -O2 -target bpf -c ebpf/tc_redirect.bpf.c -o ebpf/tc_redirect.o"
````

````

**产物检查**: `ls -l ebpf/tc_redirect.o`

### 2.2 编译 Rust CNI 二进制 (`mascni`)

使用官方 Rust 镜像构建 Linux (glibc) 兼容的可执行文件。

```bash
docker run --rm -v $(pwd):/usr/src/mascni -w /usr/src/mascni rust:1.83 cargo build --release
````

**产物检查**: `ls -l target/release/mascni`
_(注意：产物在 `target/release/` 下，不是 `target/x86_64-unknown-linux-gnu/`)_

## 3. 部署到 Kind (Deploy)

将构建好的文件拷贝到 Kind 节点的 CNI 目录。

```bash
# 1. 拷贝二进制
docker cp target/release/mascni kind-control-plane:/opt/cni/bin/mascni

# 2. 拷贝 eBPF 对象
docker cp ebpf/tc_redirect.o kind-control-plane:/opt/cni/bin/tc_redirect.o

# 3. 赋予执行权限
docker exec kind-control-plane chmod +x /opt/cni/bin/mascni
# 3. 赋予执行权限
docker exec kind-control-plane chmod +x /opt/cni/bin/mascni

# 4. 拷贝配置文件 (关键！)
# Kubelet 会监控 /etc/cni/net.d/ 目录，并加载字典序第一的配置文件。
docker cp 10-mascni-new.conf kind-control-plane:/etc/cni/net.d/10-mascni-new.conf
```

## 4. 安装运行时依赖 (Install Runtime Deps)

Rust CNI 依赖 `bpftool` 来操作 eBPF Map。Kind 节点（基于 Debian）默认可能没有。

```bash
docker exec kind-control-plane apt-get update
docker exec kind-control-plane apt-get install -y bpftool
```

_(如果网络不好，可能需要多试几次，或者手动进容器装)_

## 5. 验证 (Verify)

### 5.1 重建测试 Pod

```bash
# 删除旧的
kubectl delete pod mascni-pod --ignore-not-found=true

# 部署新的
kubectl apply -f test-pod.yaml

# 等待启动
kubectl get pod mascni-pod -o wide -w
```

### 5.2 连通性测试 (Ping)

等到 Pod 状态变成 `Running`，执行 Ping 测试（Ping 网关）：

```bash
kubectl exec mascni-pod -- ping -c 2 10.88.0.1
```

**预期输出**:

```text
64 bytes from 10.88.0.1: seq=0 ttl=64 time=0.xxx ms
64 bytes from 10.88.0.1: seq=1 ttl=64 time=0.xxx ms
```

### 5.3 深度检查 (可选)

检查 eBPF 程序是否挂载，以及路由表是否正确。

**检查宿主机路由 (确认 Route Fallback 生效)**:

```bash
# 应该能看到类似：10.99.0.x dev vethxxxx scope link
docker exec kind-control-plane ip route show | grep 10.99.0
```

**检查 eBPF 挂载**:

```bash
# 先找到对应的 veth 名字 (宿主机端)
docker exec kind-control-plane ip addr | grep veth

# 查看 tc filter (替换为你查到的 veth 名字)
docker exec kind-control-plane tc filter show dev <veth_name> ingress
```
