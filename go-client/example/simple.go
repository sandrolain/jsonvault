package main

import (
	"log"

	client "github.com/sandrolain/rust-json-db-client"
)

func main() {
	// Simple example showing basic usage
	c, err := client.NewClient("127.0.0.1:8080")
	if err != nil {
		log.Fatal("Failed to connect:", err)
	}
	defer c.Close()

	// Ping the server
	if err := c.Ping(); err != nil {
		log.Fatal("Ping failed:", err)
	}
	log.Println("Connected successfully!")

	// Set a simple value
	user := map[string]interface{}{
		"name": "John Doe",
		"age":  25,
	}
	if err := c.Set("user:1", user); err != nil {
		log.Fatal("Set failed:", err)
	}
	log.Println("User created")

	// Get the value
	result, err := c.Get("user:1")
	if err != nil {
		log.Fatal("Get failed:", err)
	}
	log.Printf("User: %v", result)

	// Set a nested property
	if err := c.QSet("user:1", "address.city", "New York"); err != nil {
		log.Fatal("QSet failed:", err)
	}
	log.Println("Address added")

	// Query with JSONPath
	name, err := c.QGet("user:1", "$.name")
	if err != nil {
		log.Fatal("QGet failed:", err)
	}
	log.Printf("Name: %v", name)

	// Merge additional data
	updates := map[string]interface{}{
		"age":    26,
		"active": true,
	}
	if err := c.Merge("user:1", updates); err != nil {
		log.Fatal("Merge failed:", err)
	}
	log.Println("User updated")

	// Get final result
	final, err := c.Get("user:1")
	if err != nil {
		log.Fatal("Get failed:", err)
	}
	log.Printf("Final user: %v", final)

	// Clean up
	if err := c.Delete("user:1"); err != nil {
		log.Fatal("Delete failed:", err)
	}
	log.Println("User deleted")
}
