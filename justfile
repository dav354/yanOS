# Default task
default: all

# Build everything
all: build-backend build-frontend

# Build the Rust backend
build-backend:
    cd backend && cargo build

# Build the Svelte frontend
build-frontend:
    cd frontend && npm install && npm run build

# Run the backend
run-backend:
    cd backend && cargo run

# Run the frontend dev server
run-frontend:
    cd frontend && npm install && npm run dev

# Clean all build artifacts
clean:
    cd backend && cargo clean
    rm -rf frontend/node_modules
    rm -rf frontend/.svelte-kit
