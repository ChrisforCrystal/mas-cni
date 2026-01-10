# mascni

An educational CNI plugin implementing a minimal Bridge CNI in Go.

## Features

- **ADD**: Creates a bridge on the host and connects containers via veth pairs.
- **DEL**: Cleans up network interfaces.
- **CHECK**: Verifies IPAM and Bridge health.
- **VERSION**: Reports CNI version 1.0.0 support.
- **IPAM**: Delegates to `host-local`.

## Build

```bash
GOOS=linux go build -o bin/mascni cmd/mascni/main.go
```

## Configuration

Example `/etc/cni/net.d/10-mascni.conf`:

```json
{
  "cniVersion": "1.0.0",
  "name": "mascni-net",
  "type": "mascni",
  "bridge": "cni0",
  "ipam": {
    "type": "host-local",
    "subnet": "10.22.0.0/16"
  }
}
```

## Testing

Use `cnitool`:

```bash
cnitool add mascni-net /var/run/netns/testing
```
