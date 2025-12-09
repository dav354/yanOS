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

# Deploy and run frontend on VM
dev-frontend-remote:
    rsync -avz --exclude 'node_modules' --exclude '.svelte-kit' --exclude '.git' frontend/ root@192.168.122.143:~/zos-frontend/
    ssh root@192.168.122.143 "cd ~/zos-frontend && npm install && npm run dev -- --host 0.0.0.0"

# Clean all build artifacts
clean:
    cd backend && cargo clean
    rm -rf frontend/node_modules
    rm -rf frontend/.svelte-kit
