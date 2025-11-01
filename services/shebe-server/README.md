# Shebe Server

**Simple RAG Service for Code Search**

The main Rust-based RAG (Retrieval-Augmented Generation)
service providing BM25 full-text search capabilities for code
repositories.

## Quick Start

```bash
# From this directory
cargo build --release
cargo run

# Run tests
cargo test

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage
```

## Architecture

See [ARCHITECTURE.md](/ARCHITECTURE.md) in the repository root
for complete system design.

## Key Features

- **BM25 Full-Text Search** via Tantivy
- **UTF-8 Safe Chunking** (character-based, never panics)
- **Session-Based Indexing** (isolated indexes)
- **REST API** (5 endpoints)
- **Production Ready** (Docker, logging, metrics)

## API Endpoints

- `GET /health` - Health check
- `POST /api/v1/index` - Index repository
- `POST /api/v1/search` - Execute search
- `GET /api/v1/sessions` - List sessions
- `DELETE /api/v1/sessions/{id}` - Delete session

## Configuration

Configuration via `shebe.toml` or environment variables:

```toml
[server]
host = "127.0.0.1"
port = 3000
log_level = "info"

[indexing]
chunk_size = 512
overlap = 64
max_file_size_mb = 10

[storage]
index_dir = "./data"

[search]
default_k = 10
max_k = 100
```

## Development

**Working Directory:** Always from this directory:
`/home/orodha/gitlab/rhobimd-oss/shebe/services/shebe-server/`

See [CLAUDE.md](/CLAUDE.md) for development workflows and
conventions.

## Documentation

- **Root:** `/docs/` - Project-wide documentation
- **Implementation:** `/docs/implementation-details/
  002-phase01-simple-rag-implementation.md`
- **Context:** `/docs/CONTEXT.md` - Project status tracker

## License

See LICENSE file in repository root.
