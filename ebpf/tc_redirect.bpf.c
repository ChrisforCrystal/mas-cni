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

SEC("tc_ingress")
int tc_redirect(struct __sk_buff *skb) {
    void *data_end = (void *)(long)skb->data_end;
    void *data = (void *)(long)skb->data;
    struct ethhdr *eth = data;

    // 1. Check packet length valid including Ethernet header
    if (data + sizeof(*eth) > data_end)
        return TC_ACT_OK;

    // 2. Only handle IPv4 (ETH_P_IP = 0x0800)
    if (eth->h_proto != bpf_htons(ETH_P_IP))
        return TC_ACT_OK;

    // 3. Parse IP Header
    struct iphdr *ip = data + sizeof(*eth);
    if ((void *)(ip + 1) > data_end)
        return TC_ACT_OK;

    // 4. Lookup Destination IP in our routing map
    __u32 dest_ip = ip->daddr;
    __u32 *ifindex = bpf_map_lookup_elem(&routes, &dest_ip);

    if (ifindex) {
        // 5. Found a route! Redirect to target interface.
        // 0 flags = egress of target interface
        return bpf_redirect(*ifindex, 0);
    }

    // No route found, let kernel stack handle it (maybe it's for the host)
    return TC_ACT_OK;
}

char LICENSE[] SEC("license") = "GPL";
