# Data Model: Initial CNI MVP

## Entities

### `PluginConf` (Go Struct)

The internal representation of the standard Input JSON.

**Source**: `pkg/config/types.go`

| Field        | Type   | Required | Description                                   |
| ------------ | ------ | -------- | --------------------------------------------- |
| `name`       | string | Yes      | Network name                                  |
| `type`       | string | Yes      | Plugin type ("mascni")                        |
| `cniVersion` | string | Yes      | CNI Spec version                              |
| `bridge`     | string | No       | Name of the bridge (default: "cni0")          |
| `isGateway`  | bool   | No       | Whether the bridge acts as GW (default: true) |
| `ipMasq`     | bool   | No       | Enable IP Masquerading (default: false)       |
| `ipam`       | JSON   | Yes      | Configuration for IPAM delegate               |

### `RuntimeArgs` (Env Vars)

Key runtime arguments passed by the runtime.

| Key               | Description                                  |
| ----------------- | -------------------------------------------- |
| `CNI_COMMAND`     | "ADD", "DEL", "CHECK", "VERSION"             |
| `CNI_CONTAINERID` | Unique ID of the container                   |
| `CNI_NETNS`       | Path to container namespace                  |
| `CNI_IFNAME`      | Interface name inside container (e.g., eth0) |
| `CNI_PATH`        | Path to search for IPAM plugins              |
