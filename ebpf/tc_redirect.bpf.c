#include <linux/bpf.h>
#include <linux/pkt_cls.h>
#include <linux/if_ether.h>
#include <linux/ip.h>
#include <linux/in.h>
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_endian.h>

// BPF Map definition: Destination IP (u32) -> Interface Index (u32)
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 1024);
    __type(key, __u32);   // Destination IP
    __type(value, __u32); // Target Interface Index
} routes SEC(".maps");

// NEW: ARP Table (IP -> MAC)
// 为了解决 L2 问题，我们必须知道目标 Pod 的真实 MAC 地址。
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, 1024);
    __type(key, __u32);             // Destination IP
    __type(value, unsigned char[6]);// Target MAC (6 bytes)
} arp_table SEC(".maps");

SEC("tc_ingress")
int tc_redirect(struct __sk_buff *skb) {
    // 0. 数据指针准备
    // eBPF 中访问数据包必须使用 data 和 data_end 指针，并且必须做边界检查（Verifier 要求）。
    void *data_end = (void *)(long)skb->data_end;
    void *data = (void *)(long)skb->data;
    struct ethhdr *eth = data;

    // 1. Check packet length valid including Ethernet header
    // 边界检查：确保包长至少包含一个以太网头。否则是非法包，放行（或丢弃）。
    if (data + sizeof(*eth) > data_end)
        return TC_ACT_OK;

    // 2. Only handle IPv4 (ETH_P_IP = 0x0800)
    // 协议过滤：我们只处理 IPv4 包。
    // 如果是 ARP (ETH_P_ARP) 或 IPv6，直接 return TC_ACT_OK 让内核协议栈去处理。
    if (eth->h_proto != bpf_htons(ETH_P_IP))
        return TC_ACT_OK;

    // 3. Parse IP Header
    // 跨过以太网头，指向 IP 头。再次做边界检查。
    struct iphdr *ip = data + sizeof(*eth);
    if ((void *)(ip + 1) > data_end)
        return TC_ACT_OK;

    // 4. Lookup Destination IP in our routing map
    // 核心逻辑：
    // 拿到 IP 头里的【目的 IP 地址】(daddr)。
    // 去我们的全局 Map ("routes") 里查一下：这个 IP 有没有对应的网卡？
    __u32 dest_ip = ip->daddr;
    __u32 *ifindex = bpf_map_lookup_elem(&routes, &dest_ip);

    // 去 ARP 表查MAC
    unsigned char *target_mac = bpf_map_lookup_elem(&arp_table, &dest_ip);

    if (ifindex && target_mac) {
        // 5. Found a route & MAC! Redirect with L2 Rewrite.
        
        // [MAC Spoofing / L2 Rewrite]
        // 我们不仅要改方向(redirect)，还要改“信封”上的收件人名字。
        // 将以太网头的 h_dest 修改为目标 Pod 的真实 MAC。
        // 这样目标 Pod 收到包后，一看 MAC 是自己的，才会接受。
        
        // 注意：这里需要考虑 BPF Helper 对内存操作的限制。
        // 我们直接使用指针赋值通常是可以的，因为 eth 已经被验证过在 data_end 之前。
        // 为了稳妥，我们使用 __builtin_memcpy。
        __builtin_memcpy(eth->h_dest, target_mac, ETH_ALEN);
        
        // 优化：最好把源 MAC (h_source) 也改成网关的 MAC，假装是网关发来的。
        // 但 MVP 阶段如果不改源 MAC，通常也能通（只要目标不校验源 MAC）。
        // 暂时只改 Dest MAC。

        // 查到了！(Hit)
        // bpf_redirect(*ifindex, 0) 的意思是：
        // "别走原本的路线了，立刻、马上把这个包扔给 ifindex 这个网卡！"
        // flag=0 表示 redirect 到目标网卡的 Egress（即发出去）。
        //
        // 这就是【Pod-to-Pod 直通】的关键：直接跳过了宿主机 IP 协议栈。
        return bpf_redirect(*ifindex, 0);
    }

    // No route found, let kernel stack handle it (maybe it's for the host)
    // 没查到 (Miss)。
    // 可能是去外网的包，或者是发给宿主机自己的包。
    // 返回 TC_ACT_OK，这就相当于"放行"，让包继续走宿主机的标准 iptables/路由流程。
    return TC_ACT_OK;
}

char LICENSE[] SEC("license") = "GPL";
