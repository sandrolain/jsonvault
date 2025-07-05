package main

import (
	"encoding/json"
	"fmt"
	"log"

	client "github.com/sandrolain/rust-json-db-client"
)

func main() {
	// Connect to the database server
	c, err := client.NewClient("127.0.0.1:8080")
	if err != nil {
		log.Fatal("Failed to connect:", err)
	}
	defer c.Close()

	fmt.Println("🔌 Connected to JSON Database")

	// Test Ping
	fmt.Println("\n🏓 Testing Ping...")
	if err := c.Ping(); err != nil {
		log.Fatal("Ping failed:", err)
	}
	fmt.Println("✅ Ping successful")

	// Test Set
	fmt.Println("\n📝 Testing Set...")
	user := map[string]interface{}{
		"name": "Alice Smith",
		"age":  28,
		"city": "New York",
	}
	if err := c.Set("user:alice", user); err != nil {
		log.Fatal("Set failed:", err)
	}
	fmt.Println("✅ Set successful")

	// Test Get
	fmt.Println("\n📖 Testing Get...")
	result, err := c.Get("user:alice")
	if err != nil {
		log.Fatal("Get failed:", err)
	}
	if result != nil {
		resultJSON, _ := json.MarshalIndent(result, "", "  ")
		fmt.Printf("Result: %s\n", string(resultJSON))
	} else {
		fmt.Println("No result found")
	}

	// Test QGet (JSONPath query)
	fmt.Println("\n🔍 Testing QGet (JSONPath query)...")
	nameResult, err := c.QGet("user:alice", "$.name")
	if err != nil {
		log.Fatal("QGet failed:", err)
	}
	fmt.Printf("Name: %v\n", nameResult)

	// Test QSet (set sub-property)
	fmt.Println("\n🎯 Testing QSet (set sub-property)...")
	if err := c.QSet("user:alice", "profession", "Software Engineer"); err != nil {
		log.Fatal("QSet failed:", err)
	}
	fmt.Println("✅ QSet successful")

	// Get after QSet
	fmt.Println("\n📖 Testing Get after QSet...")
	result, err = c.Get("user:alice")
	if err != nil {
		log.Fatal("Get failed:", err)
	}
	if result != nil {
		resultJSON, _ := json.MarshalIndent(result, "", "  ")
		fmt.Printf("Result after QSet: %s\n", string(resultJSON))
	}

	// Test Merge
	fmt.Println("\n🔀 Testing Merge...")
	updateData := map[string]interface{}{
		"age":     29,
		"country": "USA",
	}
	if err := c.Merge("user:alice", updateData); err != nil {
		log.Fatal("Merge failed:", err)
	}
	fmt.Println("✅ Merge successful")

	// Get after Merge
	fmt.Println("\n📖 Testing Get after Merge...")
	result, err = c.Get("user:alice")
	if err != nil {
		log.Fatal("Get failed:", err)
	}
	if result != nil {
		resultJSON, _ := json.MarshalIndent(result, "", "  ")
		fmt.Printf("Final result: %s\n", string(resultJSON))
	}

	// Test complex nested operations
	fmt.Println("\n🏗️ Testing complex nested operations...")
	config := map[string]interface{}{
		"database": map[string]interface{}{
			"host": "localhost",
			"port": 5432,
		},
		"features": []string{"auth", "logging"},
	}
	if err := c.Set("app:config", config); err != nil {
		log.Fatal("Set config failed:", err)
	}

	// Set nested property with QSet
	if err := c.QSet("app:config", "database.timeout", 30); err != nil {
		log.Fatal("QSet timeout failed:", err)
	}

	// Query nested property with QGet
	hostResult, err := c.QGet("app:config", "$.database.host")
	if err != nil {
		log.Fatal("QGet host failed:", err)
	}
	fmt.Printf("Database host: %v\n", hostResult)

	// Get final config
	configResult, err := c.Get("app:config")
	if err != nil {
		log.Fatal("Get config failed:", err)
	}
	if configResult != nil {
		configJSON, _ := json.MarshalIndent(configResult, "", "  ")
		fmt.Printf("Final config: %s\n", string(configJSON))
	}

	// Test Delete
	fmt.Println("\n🗑️ Testing Delete...")
	if err := c.Delete("user:alice"); err != nil {
		log.Fatal("Delete failed:", err)
	}
	fmt.Println("✅ Delete successful")

	// Verify deletion
	fmt.Println("\n📖 Verifying deletion...")
	result, err = c.Get("user:alice")
	if err != nil {
		log.Fatal("Get after delete failed:", err)
	}
	if result == nil {
		fmt.Println("✅ User successfully deleted")
	} else {
		fmt.Printf("⚠️ User still exists: %v\n", result)
	}

	fmt.Println("\n🎉 All tests completed successfully!")
}
