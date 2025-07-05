package client

import (
	"bufio"
	"encoding/binary"
	"encoding/json"
	"fmt"
	"net"
	"time"
)

// SetCommand represents a SET command
type SetCommand struct {
	Set SetData `json:"Set"`
}

type SetData struct {
	Key   string      `json:"key"`
	Value interface{} `json:"value"`
}

// GetCommand represents a GET command
type GetCommand struct {
	Get GetData `json:"Get"`
}

type GetData struct {
	Key string `json:"key"`
}

// DeleteCommand represents a DELETE command
type DeleteCommand struct {
	Delete DeleteData `json:"Delete"`
}

type DeleteData struct {
	Key string `json:"key"`
}

// QGetCommand represents a QGET command
type QGetCommand struct {
	QGet QGetData `json:"QGet"`
}

type QGetData struct {
	Key   string `json:"key"`
	Query string `json:"query"`
}

// QSetCommand represents a QSET command
type QSetCommand struct {
	QSet QSetData `json:"QSet"`
}

type QSetData struct {
	Key   string      `json:"key"`
	Path  string      `json:"path"`
	Value interface{} `json:"value"`
}

// MergeCommand represents a MERGE command
type MergeCommand struct {
	Merge MergeData `json:"Merge"`
}

type MergeData struct {
	Key   string      `json:"key"`
	Value interface{} `json:"value"`
}

// PingCommand represents a PING command
type PingCommand struct {
	Ping interface{} `json:"Ping"`
}

// Response represents a server response
type Response struct {
	Ok    interface{} `json:"Ok,omitempty"`
	Error string      `json:"Error,omitempty"`
	Pong  interface{} `json:"Pong,omitempty"`
}

// Client represents a connection to the JSON database
type Client struct {
	conn   net.Conn
	reader *bufio.Reader
}

// NewClient creates a new client connection to the specified address
func NewClient(address string) (*Client, error) {
	conn, err := net.DialTimeout("tcp", address, 10*time.Second)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to %s: %w", address, err)
	}

	return &Client{
		conn:   conn,
		reader: bufio.NewReader(conn),
	}, nil
}

// Close closes the connection to the server
func (c *Client) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// sendCommand sends a command to the server and returns the response
func (c *Client) sendCommand(cmd interface{}) (interface{}, error) {
	// Serialize command to JSON
	data, err := json.Marshal(cmd)
	if err != nil {
		return nil, fmt.Errorf("failed to marshal command: %w", err)
	}

	// Send length prefix (4 bytes, big endian)
	length := uint32(len(data))
	if err := binary.Write(c.conn, binary.BigEndian, length); err != nil {
		return nil, fmt.Errorf("failed to write length: %w", err)
	}

	// Send JSON data
	if _, err := c.conn.Write(data); err != nil {
		return nil, fmt.Errorf("failed to write data: %w", err)
	}

	// Read response length
	var respLength uint32
	if err := binary.Read(c.reader, binary.BigEndian, &respLength); err != nil {
		return nil, fmt.Errorf("failed to read response length: %w", err)
	}

	// Read response data
	respData := make([]byte, respLength)
	if _, err := c.reader.Read(respData); err != nil {
		return nil, fmt.Errorf("failed to read response data: %w", err)
	}

	// Parse response as generic interface first
	var response interface{}
	if err := json.Unmarshal(respData, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return response, nil
}

// parseResponse parses a generic response into specific types
func parseResponse(resp interface{}) (value interface{}, err error) {
	switch v := resp.(type) {
	case string:
		// Handle enum variants like "Pong"
		if v == "Pong" {
			return nil, nil // Pong successful
		}
		// Otherwise it's an error message or unknown
		return nil, fmt.Errorf("unexpected string response: %s", v)
	case map[string]interface{}:
		// Handle structured responses like {"Ok": value} or {"Error": "message"}
		if okValue, exists := v["Ok"]; exists {
			return okValue, nil
		}
		if errorMsg, exists := v["Error"]; exists {
			if errStr, ok := errorMsg.(string); ok {
				return nil, fmt.Errorf("server error: %s", errStr)
			}
			return nil, fmt.Errorf("server error: %v", errorMsg)
		}
		return nil, fmt.Errorf("unknown response format: %v", v)
	default:
		return nil, fmt.Errorf("unexpected response type: %T", resp)
	}
}

// Set sets a value for the given key
func (c *Client) Set(key string, value interface{}) error {
	cmd := SetCommand{
		Set: SetData{
			Key:   key,
			Value: value,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return err
	}

	_, err = parseResponse(resp)
	return err
}

// Get retrieves the value for the given key
func (c *Client) Get(key string) (interface{}, error) {
	cmd := GetCommand{
		Get: GetData{
			Key: key,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return nil, err
	}

	return parseResponse(resp)
}

// Delete removes the value for the given key
func (c *Client) Delete(key string) error {
	cmd := DeleteCommand{
		Delete: DeleteData{
			Key: key,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return err
	}

	_, err = parseResponse(resp)
	return err
}

// QGet executes a JSONPath query on the value at the given key
func (c *Client) QGet(key, query string) (interface{}, error) {
	cmd := QGetCommand{
		QGet: QGetData{
			Key:   key,
			Query: query,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return nil, err
	}

	return parseResponse(resp)
}

// QSet sets a sub-property using JSONPath
func (c *Client) QSet(key, path string, value interface{}) error {
	cmd := QSetCommand{
		QSet: QSetData{
			Key:   key,
			Path:  path,
			Value: value,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return err
	}

	_, err = parseResponse(resp)
	return err
}

// Merge merges a JSON value with the existing value at the given key
func (c *Client) Merge(key string, value interface{}) error {
	cmd := MergeCommand{
		Merge: MergeData{
			Key:   key,
			Value: value,
		},
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return err
	}

	_, err = parseResponse(resp)
	return err
}

// Ping sends a ping to the server
func (c *Client) Ping() error {
	cmd := PingCommand{
		Ping: nil,
	}

	resp, err := c.sendCommand(cmd)
	if err != nil {
		return err
	}

	_, err = parseResponse(resp)
	return err
}
