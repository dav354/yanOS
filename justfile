# Config
vmConnection := "root@192.168.122.143"
remoteDir := "/opt/zos"

# Derived paths (Must be defined at top level for 'just')
backendDir := remoteDir + "/backend"
frontendDir := remoteDir + "/ui"

# Build the Rust backend
build-backend:
    cd backend && cargo build

# Build the Svelte frontend
build-frontend:
    cd frontend && npm ci && npm run build

# Deploy the backend on VM
deploy-backend:
    ssh {{ vmConnection }} "mkdir -p {{ backendDir }}"
    rsync -avz --delete --exclude 'target' backend/ {{ vmConnection }}:{{ backendDir }}
    ssh -t {{ vmConnection }} "cd {{ backendDir }} && LD_LIBRARY_PATH=/opt/ooce/llvm-21/lib:\$LD_LIBRARY_PATH /opt/ooce/bin/cargo run --color=always --profile dev"

# Deploy frontend on VM
deploy-frontend: build-frontend
    ssh {{ vmConnection }} "mkdir -p {{ frontendDir }}"
    rsync -avz --delete frontend/build/ {{ vmConnection }}:{{ frontendDir }}

# Full Deploy
deploy: deploy-frontend deploy-backend

# Clean all build artifacts
clean:
    cd backend && cargo clean
    rm -rf frontend/node_modules
    rm -rf frontend/.svelte-kit
    rm -rf frontend/build
