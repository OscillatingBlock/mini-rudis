# Mini-Redis Rust Implementation

A lightweight, asynchronous Redis clone built from scratch using **Rust** and **Tokio**. This project implements a subset of the Redis [RESP (REdis Serialization Protocol)](https://www.google.com/search?q=https://redis.io/docs/reference/protocol-spec/) and supports concurrent client connections.

## Features

* **Asynchronous I/O**: Built on top of `tokio` for high-performance, non-blocking networking.
* **Shared Global State**: Uses `Arc<Mutex<HashMap>>` to allow multiple clients to interact with the same data store in real-time.
* **Recursive RESP Parsing**: Supports nested Arrays, Bulk Strings, Simple Strings, Integers, and Nulls using boxed recursion for memory safety.
* **Command Dispatcher**: Case-insensitive command processing (e.g., `SET`, `set`, and `SeT` all work).

## Supported Commands

| Command | Description |
| --- | --- |
| `GET` | Retrieve the value of a string key. |
| `SET` | Set a string key to a string value. |
| `HGET` | Get the value of a hash field. |
| `HSET` | Set the value of a hash field (creates the hash if it doesn't exist). |
| `PING` | Returns `PONG` to test connection liveliness. |

## Architectural Overview

The project is divided into several modular components:

* **`Connection`**: Handles low-level byte reading and writing to the TCP stream, including RESP frame serialization/deserialization.
* **`Store`**: The thread-safe engine managing the underlying `HashMap`.
* **`Handler`**: Logic for extracting arguments from Frames and executing the appropriate store operations.
* **`Server`**: The main loop that accepts TCP listeners and spawns a new Tokio task for every connection.

## Getting Started

### Prerequisites

* [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
* `redis-cli` (optional, for testing)

### Installation

1. Clone the repository:
```bash
git clone git@github.com:OscillatingBlock/mini-rudis.git
cd mini-rudis

```


2. Run the server:
```bash
cargo run
```



### Testing with redis-cli

In a separate terminal, connect using the standard Redis client:

```bash
redis-cli -p 6379
127.0.0.1:6379> SET user:1 "Rustacean"
OK
127.0.0.1:6379> HSET profile:1 name "Aayush"
OK
127.0.0.1:6379> HGET profile:1 name
"Aayush"

```

## 🧠 What I Learned

* Parsing binary-safe protocols (RESP).
* Handling **Async Recursion** in Rust using `Box::pin`.
* Implementing the **Guard Pattern** with `let else` for cleaner, flatter code.
* Managing thread-safe state with `Arc` and `Mutex`.

---
