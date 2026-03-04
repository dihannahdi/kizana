# Kizana Search — Panduan Deployment VPS

## Arsitektur Sistem

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Nginx      │────▶│  SvelteKit   │     │  Rust Backend │
│  (port 80/   │     │  (port 3000) │────▶│  (port 8080)  │
│   443 SSL)   │────▶│              │     │               │
└──────────────┘     └──────────────┘     └───────┬───────┘
                                                  │
                                          ┌───────┴───────┐
                                          │   SQLite DB   │
                                          │   (20GB)      │
                                          │   + Tantivy   │
                                          │   + Redis     │
                                          └───────────────┘
```

## Prasyarat VPS

- **OS**: Ubuntu 22.04+ / Debian 12+
- **RAM**: Minimum 8 GB (Tantivy indexing memerlukan ~2GB)
- **Disk**: Minimum 100 GB (DB 20GB + index + logs)
- **CPU**: 2+ cores
- **Domain**: Sudah diarahkan ke IP VPS

---

## Langkah 1: Persiapan Server

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y nginx redis-server certbot python3-certbot-nginx \
    nodejs npm curl unzip rsync

# Install Node.js 20+ via nvm (jika versi distro terlalu lama)
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs

# Start Redis
sudo systemctl enable redis-server
sudo systemctl start redis-server
```

## Langkah 2: Buat User & Directory

```bash
# Buat user khusus
sudo useradd -r -m -s /bin/bash kizana

# Buat directory struktur
sudo mkdir -p /opt/kizana/{backend,frontend,data,tantivy_index}
sudo chown -R kizana:kizana /opt/kizana
```

## Langkah 3: Upload File dari Lokal

Dari mesin Windows lokal, jalankan:

```powershell
# === BACKEND ===
# Upload binary
scp d:\nahdi\bahtsulmasail\backend\target\release\kizana-search.exe user@VPS_IP:/tmp/
# (Binary Windows .exe TIDAK bisa dijalankan di Linux — harus cross-compile, lihat Langkah 3b)

# Upload source code untuk build di server
scp -r d:\nahdi\bahtsulmasail\backend\src user@VPS_IP:/opt/kizana/backend/src/
scp d:\nahdi\bahtsulmasail\backend\Cargo.toml user@VPS_IP:/opt/kizana/backend/
scp d:\nahdi\bahtsulmasail\backend\Cargo.lock user@VPS_IP:/opt/kizana/backend/
scp d:\nahdi\bahtsulmasail\backend\.env user@VPS_IP:/opt/kizana/backend/

# === FRONTEND ===
scp -r d:\nahdi\bahtsulmasail\frontend\src user@VPS_IP:/opt/kizana/frontend/src/
scp d:\nahdi\bahtsulmasail\frontend\package.json user@VPS_IP:/opt/kizana/frontend/
scp d:\nahdi\bahtsulmasail\frontend\svelte.config.js user@VPS_IP:/opt/kizana/frontend/
scp d:\nahdi\bahtsulmasail\frontend\vite.config.js user@VPS_IP:/opt/kizana/frontend/

# === DATABASE ===
# Upload database (20GB — gunakan rsync untuk resume)
rsync -avz --progress d:\nahdi\bahtsulmasail\kizana_all_books.sqlite user@VPS_IP:/opt/kizana/data/

# === DEPLOY CONFIGS ===
scp -r d:\nahdi\bahtsulmasail\deploy user@VPS_IP:/opt/kizana/
```

## Langkah 3b: Build Backend di Server

Karena binary Windows (.exe) tidak berjalan di Linux, build langsung di server:

```bash
# Login ke server
ssh user@VPS_IP

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build backend
cd /opt/kizana/backend
cargo build --release

# Binary akan di: /opt/kizana/backend/target/release/kizana-search
```

## Langkah 4: Konfigurasi Backend

```bash
# Edit .env file
sudo -u kizana nano /opt/kizana/backend/.env
```

Isi `.env`:
```env
DATABASE_PATH=/opt/kizana/data/kizana_all_books.sqlite
REDIS_URL=redis://127.0.0.1:6379
JWT_SECRET=ganti_dengan_secret_yang_kuat_minimal_32_karakter
AI_API_KEY=                    # Kosongkan jika belum ada
AI_API_URL=https://api.x.ai/v1/chat/completions
AI_MODEL=grok-3-mini
TANTIVY_INDEX_PATH=/opt/kizana/tantivy_index
FRONTEND_URL=https://yourdomain.com
HOST=127.0.0.1
PORT=8080
```

## Langkah 5: Build Frontend di Server

```bash
cd /opt/kizana/frontend
npm install
npm run build
```

## Langkah 6: Setup systemd Services

```bash
# Backend service
sudo cp /opt/kizana/deploy/kizana-backend.service /etc/systemd/system/
sudo nano /etc/systemd/system/kizana-backend.service
```

Sesuaikan isi service:
```ini
[Unit]
Description=Kizana Search Backend
After=network.target redis-server.service

[Service]
Type=simple
User=kizana
Group=kizana
WorkingDirectory=/opt/kizana/backend
ExecStart=/opt/kizana/backend/target/release/kizana-search
Environment=RUST_LOG=info
Restart=always
RestartSec=5
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

```bash
# Frontend service
sudo cp /opt/kizana/deploy/kizana-frontend.service /etc/systemd/system/
sudo nano /etc/systemd/system/kizana-frontend.service
```

```ini
[Unit]
Description=Kizana Search Frontend
After=network.target

[Service]
Type=simple
User=kizana
Group=kizana
WorkingDirectory=/opt/kizana/frontend
ExecStart=/usr/bin/node build/index.js
Environment=PORT=3000
Environment=ORIGIN=https://yourdomain.com
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
# Enable dan start
sudo systemctl daemon-reload
sudo systemctl enable kizana-backend kizana-frontend
sudo systemctl start kizana-backend
# Tunggu backend selesai indexing (cek log), lalu:
sudo systemctl start kizana-frontend
```

## Langkah 7: Konfigurasi Nginx

```bash
sudo nano /etc/nginx/sites-available/kizana
```

```nginx
server {
    listen 80;
    server_name yourdomain.com;
    
    # Redirect ke HTTPS (setelah SSL setup)
    # return 301 https://$host$request_uri;

    # Gzip
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml;
    gzip_min_length 256;

    # API Backend
    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_read_timeout 300s;
        proxy_connect_timeout 10s;
        
        # CORS
        add_header Access-Control-Allow-Origin $http_origin always;
        add_header Access-Control-Allow-Methods "GET, POST, OPTIONS" always;
        add_header Access-Control-Allow-Headers "Authorization, Content-Type" always;
        
        if ($request_method = OPTIONS) {
            return 204;
        }
    }

    # Frontend SvelteKit
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Client max body size
    client_max_body_size 10M;
}
```

```bash
# Aktifkan site
sudo ln -s /etc/nginx/sites-available/kizana /etc/nginx/sites-enabled/
sudo rm -f /etc/nginx/sites-enabled/default
sudo nginx -t
sudo systemctl reload nginx
```

## Langkah 8: SSL Certificate (Let's Encrypt)

```bash
sudo certbot --nginx -d yourdomain.com
# Ikuti instruksi, pilih redirect HTTP ke HTTPS
sudo systemctl reload nginx
```

## Langkah 9: Firewall

```bash
sudo ufw allow ssh
sudo ufw allow 'Nginx Full'
sudo ufw enable
```

---

## Monitoring & Maintenance

### Cek Status
```bash
sudo systemctl status kizana-backend
sudo systemctl status kizana-frontend
sudo systemctl status nginx
sudo systemctl status redis-server
```

### Lihat Log
```bash
# Backend logs
sudo journalctl -u kizana-backend -f

# Frontend logs
sudo journalctl -u kizana-frontend -f

# Nginx logs
sudo tail -f /var/log/nginx/error.log
```

### Health Check
```bash
curl http://localhost:8080/api/health
curl http://localhost:8080/api/status
```

### Restart Services
```bash
sudo systemctl restart kizana-backend
sudo systemctl restart kizana-frontend
```

---

## Troubleshooting

### Backend tidak start
1. Cek `.env` — pastikan `DATABASE_PATH` benar
2. Cek permission: `ls -la /opt/kizana/data/`
3. Cek log: `journalctl -u kizana-backend -n 50`

### Index building lambat
- Indexing ~7872 buku pertama kali memerlukan waktu 30-60 menit
- RAM usage bisa mencapai 2GB selama proses
- Backend tetap melayani request saat indexing berlangsung

### Redis error
- Redis opsional — backend berjalan normal tanpa Redis
- Cek: `redis-cli ping` → harus jawab `PONG`

### Disk penuh
- Database: ~20 GB
- Tantivy index: ~2-5 GB
- Pastikan disk minimal 100 GB

### Permission error
```bash
sudo chown -R kizana:kizana /opt/kizana
chmod 644 /opt/kizana/data/kizana_all_books.sqlite
```

---

## Alternatif: Deploy dengan Docker

```bash
# Build image
docker build -t kizana-search .

# Run container
docker run -d \
  --name kizana \
  -p 80:80 \
  -v /path/to/kizana_all_books.sqlite:/app/data/kizana_all_books.sqlite \
  -v kizana-index:/app/tantivy_index \
  -e JWT_SECRET=your_secret_here \
  -e DATABASE_PATH=/app/data/kizana_all_books.sqlite \
  kizana-search
```

---

## Struktur File di Server

```
/opt/kizana/
├── backend/
│   ├── src/                    # Source code Rust
│   ├── target/release/
│   │   └── kizana-search       # Binary
│   ├── Cargo.toml
│   └── .env                    # Konfigurasi
├── frontend/
│   ├── src/                    # Source code Svelte
│   ├── build/                  # Production build
│   ├── package.json
│   └── node_modules/
├── data/
│   └── kizana_all_books.sqlite # Database 20GB
├── tantivy_index/              # Search index (auto-generated)
└── deploy/
    ├── nginx.conf
    ├── kizana-backend.service
    ├── kizana-frontend.service
    └── start.sh
```
