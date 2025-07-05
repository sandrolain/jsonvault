# Rust JSON Database - Go Client

A Go client library for connecting to the Rust JSON Database server.

## Features

- **Set/Get/Delete**: Basic key-value operations
- **QGet**: JSONPath queries for extracting data
- **QSet**: Set sub-properties using JSONPath
- **Merge**: Merge JSON objects
- **Ping**: Health check functionality
- **TCP Protocol**: Efficient length-prefixed JSON communication

## Installation

```bash
go get github.com/sandrolain/rust-json-db-client
```

## Usage

### Basic Example

```go
package main

import (
    "fmt"
    "log"
    
    client "github.com/sandrolain/rust-json-db-client"
)

func main() {
    // Connect to the database
    c, err := client.NewClient("127.0.0.1:8080")
    if err != nil {
        log.Fatal("Failed to connect:", err)
    }
    defer c.Close()
    
    // Set a value
    user := map[string]interface{}{
        "name": "Alice",
        "age":  30,
        "city": "New York",
    }
    if err := c.Set("user:1", user); err != nil {
        log.Fatal("Set failed:", err)
    }
    
    // Get a value
    result, err := c.Get("user:1")
    if err != nil {
        log.Fatal("Get failed:", err)
    }
    fmt.Printf("User: %v\n", result)
    
    // JSONPath query
    name, err := c.QGet("user:1", "$.name")
    if err != nil {
        log.Fatal("QGet failed:", err)
    }
    fmt.Printf("Name: %v\n", name)
    
    // Set sub-property
    if err := c.QSet("user:1", "profession", "Engineer"); err != nil {
        log.Fatal("QSet failed:", err)
    }
    
    // Merge data
    updates := map[string]interface{}{
        "age":     31,
        "country": "USA",
    }
    if err := c.Merge("user:1", updates); err != nil {
        log.Fatal("Merge failed:", err)
    }
    
    // Delete
    if err := c.Delete("user:1"); err != nil {
        log.Fatal("Delete failed:", err)
    }
}
```

## API Reference

### NewClient(address string) (*Client, error)

Creates a new client connection to the specified address.

```go
client, err := NewClient("127.0.0.1:8080")
```

### client.Set(key string, value interface{}) error

Sets a value for the given key.

```go
err := client.Set("mykey", map[string]interface{}{"name": "Alice"})
```

### client.Get(key string) (interface{}, error)

Retrieves the value for the given key.

```go
value, err := client.Get("mykey")
```

### client.QGet(key, query string) (interface{}, error)

Executes a JSONPath query on the value at the given key.

```go
name, err := client.QGet("user:1", "$.name")
age, err := client.QGet("user:1", "$.age")
```

### client.QSet(key, path string, value interface{}) error

Sets a sub-property using JSONPath.

```go
err := client.QSet("user:1", "address.city", "Boston")
err := client.QSet("user:1", "tags.0", "developer")
```

### client.Merge(key string, value interface{}) error

Merges a JSON value with the existing value at the given key.

```go
updates := map[string]interface{}{
    "age": 31,
    "city": "Boston",
}
err := client.Merge("user:1", updates)
```

### client.Delete(key string) error

Removes the value for the given key.

```go
err := client.Delete("user:1")
```

### client.Ping() error

Sends a ping to the server to check connectivity.

```go
err := client.Ping()
```

### client.Close() error

Closes the connection to the server.

```go
err := client.Close()
```

## JSONPath Examples

The client supports JSONPath queries for both reading (QGet) and writing (QSet) operations:

### Reading with QGet

```go
// Get a specific property
name, _ := client.QGet("user", "$.name")

// Get nested properties
city, _ := client.QGet("user", "$.address.city")

// Get array elements
firstTag, _ := client.QGet("user", "$.tags[0]")

// Get all array elements
allTags, _ := client.QGet("user", "$.tags[*]")

// Get multiple properties
props, _ := client.QGet("config", "$.database.*")
```

### Writing with QSet

```go
// Set a simple property
client.QSet("user", "name", "Alice")

// Set nested properties
client.QSet("user", "address.city", "Boston")
client.QSet("user", "address.zipcode", "02101")

// Set array elements
client.QSet("user", "tags.0", "developer")
client.QSet("user", "tags.1", "golang")

// Create new nested structures
client.QSet("config", "database.host", "localhost")
client.QSet("config", "database.port", 5432)
```

## Protocol

The client communicates with the Rust JSON Database server using a simple TCP protocol:

1. **Length Prefix**: 4-byte big-endian integer indicating message length
2. **JSON Payload**: Command or response serialized as JSON

### Command Format

```json
{
  "type": "Set|Get|Delete|QGet|QSet|Merge|Ping",
  "key": "string",
  "value": "any",
  "query": "string",
  "path": "string"
}
```

### Response Format

```json
{
  "type": "Ok|Error|Pong",
  "value": "any",
  "error": "string"
}
```

## Error Handling

All client methods return an error if the operation fails. Errors can occur due to:

- Network connectivity issues
- Invalid JSON data
- Server-side errors
- Invalid JSONPath expressions

Always check for errors in production code:

```go
if err := client.Set("key", value); err != nil {
    log.Printf("Failed to set value: %v", err)
    return err
}
```

## License

This Go client library is part of the Rust JSON Database project.
