package main

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"os"

	"github.com/1password/onepassword-sdk-go"
)

// Request represents the incoming JSON structure
type Request struct {
	Method string          `json:"method"`
	Token  string          `json:"token"`
	Params json.RawMessage `json:"params"`
}

// Response represents the outgoing JSON structure
type Response struct {
	Ok     bool        `json:"ok"`
	Result interface{} `json:"result,omitempty"`
	Error  string      `json:"error,omitempty"`
}

// VaultListParams (empty, no params needed)
type VaultListParams struct{}

// ItemListParams for items.list
type ItemListParams struct {
	VaultID string `json:"vault_id"`
}

// ItemGetParams for items.get
type ItemGetParams struct {
	VaultID string `json:"vault_id"`
	ItemID  string `json:"item_id"`
}

// ItemCreateParams for items.create
type ItemCreateParams struct {
	VaultID string      `json:"vault_id"`
	Item    interface{} `json:"item"`
}

// SecretsResolveParams for secrets.resolve
type SecretsResolveParams struct {
	Reference string `json:"reference"`
}

func main() {
	// Read single line from stdin
	scanner := bufio.NewScanner(os.Stdin)
	if !scanner.Scan() {
		writeError("Failed to read from stdin")
		return
	}

	line := scanner.Text()

	// Parse request
	var req Request
	if err := json.Unmarshal([]byte(line), &req); err != nil {
		writeError(fmt.Sprintf("Invalid JSON: %v", err))
		return
	}

	// Validate token
	if req.Token == "" {
		writeError("Missing token")
		return
	}

	// Initialize 1Password client
	ctx := context.Background()
	client, err := onepassword.NewClient(
		ctx,
		onepassword.WithServiceAccountToken(req.Token),
		onepassword.WithIntegrationInfo("Tairseach", "0.1.0"),
	)
	if err != nil {
		writeError(fmt.Sprintf("Failed to initialize 1Password client: %v", err))
		return
	}

	// Route method
	switch req.Method {
	case "vaults.list":
		handleVaultsList(ctx, client)
	case "items.list":
		handleItemsList(ctx, client, req.Params)
	case "items.get":
		handleItemsGet(ctx, client, req.Params)
	case "items.create":
		handleItemsCreate(ctx, client, req.Params)
	case "secrets.resolve":
		handleSecretsResolve(ctx, client, req.Params)
	default:
		writeError(fmt.Sprintf("Unknown method: %s", req.Method))
	}
}

func handleVaultsList(ctx context.Context, client *onepassword.Client) {
	vaults, err := client.Vaults().List(ctx)
	if err != nil {
		writeError(fmt.Sprintf("Failed to list vaults: %v", err))
		return
	}
	writeSuccess(vaults)
}

func handleItemsList(ctx context.Context, client *onepassword.Client, params json.RawMessage) {
	var p ItemListParams
	if err := json.Unmarshal(params, &p); err != nil {
		writeError(fmt.Sprintf("Invalid params for items.list: %v", err))
		return
	}

	if p.VaultID == "" {
		writeError("Missing vault_id parameter")
		return
	}

	items, err := client.Items().List(ctx, p.VaultID)
	if err != nil {
		writeError(fmt.Sprintf("Failed to list items: %v", err))
		return
	}
	writeSuccess(items)
}

func handleItemsGet(ctx context.Context, client *onepassword.Client, params json.RawMessage) {
	var p ItemGetParams
	if err := json.Unmarshal(params, &p); err != nil {
		writeError(fmt.Sprintf("Invalid params for items.get: %v", err))
		return
	}

	if p.VaultID == "" || p.ItemID == "" {
		writeError("Missing vault_id or item_id parameter")
		return
	}

	item, err := client.Items().Get(ctx, p.VaultID, p.ItemID)
	if err != nil {
		writeError(fmt.Sprintf("Failed to get item: %v", err))
		return
	}
	writeSuccess(item)
}

func handleItemsCreate(ctx context.Context, client *onepassword.Client, params json.RawMessage) {
	var p ItemCreateParams
	if err := json.Unmarshal(params, &p); err != nil {
		writeError(fmt.Sprintf("Invalid params for items.create: %v", err))
		return
	}

	if p.VaultID == "" {
		writeError("Missing vault_id parameter")
		return
	}

	// Note: The SDK's actual item creation method may differ
	// This is a placeholder - adjust based on the actual SDK API
	writeError("items.create not fully implemented - SDK integration needed")
}

func handleSecretsResolve(ctx context.Context, client *onepassword.Client, params json.RawMessage) {
	var p SecretsResolveParams
	if err := json.Unmarshal(params, &p); err != nil {
		writeError(fmt.Sprintf("Invalid params for secrets.resolve: %v", err))
		return
	}

	if p.Reference == "" {
		writeError("Missing reference parameter")
		return
	}

	secret, err := client.Secrets().Resolve(ctx, p.Reference)
	if err != nil {
		writeError(fmt.Sprintf("Failed to resolve secret: %v", err))
		return
	}
	writeSuccess(map[string]string{"value": secret})
}

func writeSuccess(result interface{}) {
	resp := Response{
		Ok:     true,
		Result: result,
	}
	writeResponse(resp)
}

func writeError(message string) {
	resp := Response{
		Ok:    false,
		Error: message,
	}
	writeResponse(resp)
}

func writeResponse(resp Response) {
	data, err := json.Marshal(resp)
	if err != nil {
		// Last resort - write error directly
		fmt.Fprintf(os.Stdout, `{"ok":false,"error":"Failed to marshal response: %v"}`+"\n", err)
		return
	}
	fmt.Fprintf(os.Stdout, "%s\n", data)
}
