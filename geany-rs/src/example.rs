//! Geany-Rs Example File
//! This file demonstrates syntax highlighting features

use std::collections::HashMap;

/// Represents a user in the system
struct User {
    id: u64,
    name: String,
    email: String,
    active: bool,
}

/// User management system
impl User {
    /// Create a new user
    fn new(id: u64, name: &str, email: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            email: email.to_string(),
            active: true,
        }
    }

    /// Check if user is active
    fn is_active(&self) -> bool {
        self.active
    }

    /// Deactivate the user account
    fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Main application entry point
fn main() {
    println!("Welcome to Geany-Rs!");
    
    // Create some users
    let mut users: HashMap<u64, User> = HashMap::new();
    
    users.insert(1, User::new(1, "Alice", "alice@example.com"));
    users.insert(2, User::new(2, "Bob", "bob@example.com"));
    
    // Process users
    for (id, user) in &users {
        if user.is_active() {
            println!("User {}: {} <{}>", id, user.name, user.email);
        }
    }
    
    // Example of pattern matching
    let result = divide(10, 2);
    match result {
        Ok(value) => println!("Division result: {}", value),
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Divide two numbers safely
fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Cannot divide by zero".to_string())
    } else {
        Ok(a / b)
    }
}

// Constants
const MAX_USERS: usize = 1000;
const VERSION: &str = "1.0.0";
