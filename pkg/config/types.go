package config

import (
	"encoding/json"
	"fmt"

	"github.com/containernetworking/cni/pkg/types"
)

// PluginConf represents the standard CNI configuration plus plugin-specific fields
type PluginConf struct {
	types.NetConf

	// Plugin-specific fields
	Bridge    string `json:"bridge"`
	IsGateway bool   `json:"isGateway"`
	IPMasq    bool   `json:"ipMasq"`

	// We can add more fields here if needed
	RuntimeConfig struct {
		IPRanges []struct {
			Subnet string `json:"subnet"`
		} `json:"ipRanges,omitempty"`
	} `json:"runtimeConfig,omitempty"`
}

// LoadConf parses the standard CNI configuration from bytes
func LoadConf(bytes []byte) (*PluginConf, error) {
	n := &PluginConf{
		Bridge:    "cni0", // Default bridge name
		IsGateway: true,   // Default to acting as a gateway
	}
	if err := json.Unmarshal(bytes, n); err != nil {
		return nil, fmt.Errorf("failed to load netconf: %v", err)
	}
	return n, nil
}
