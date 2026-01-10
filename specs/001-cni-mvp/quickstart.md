# Quickstart: mascni (Generic CNI Plugin)

## Prerequisites

- Go 1.23+
- Linux environment (or VM) with root access for network testing
- `cnitool` (from `github.com/containernetworking/cni`)
- `host-local` (from `github.com/containernetworking/plugins`)

## Build

```bash
# Initialize module (first time only)
go mod init github.com/masallsome/mascni
go mod tidy

# Build
go build -o bin/mascni cmd/mascni/main.go
```

## Testing (Manual)

1. **Install Dependencies**:

   ```bash
   mkdir -p /opt/cni/bin
   cp bin/mascni /opt/cni/bin/
   # Ensure host-local is also in /opt/cni/bin/
   ```

2. **Run cnitool**:
   ```bash
   sudo CNI_PATH=/opt/cni/bin cnitool add mascni-net /var/run/netns/testing
   sudo CNI_PATH=/opt/cni/bin cnitool check mascni-net /var/run/netns/testing
   sudo CNI_PATH=/opt/cni/bin cnitool del mascni-net /var/run/netns/testing
   ```

## Configuration

Place this in `/etc/cni/net.d/10-mascni.conf` or invoke via environment:

```json
{
  "cniVersion": "1.0.0",
  "name": "mascni-net",
  "type": "mascni",
  "bridge": "cni0",
  "isGateway": true,
  "ipam": {
    "type": "host-local",
    "subnet": "10.22.0.0/16"
  }
}
```
