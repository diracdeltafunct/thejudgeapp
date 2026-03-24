# judge-api

REST API for serving Judge app database updates. Intended to replace direct Scryfall calls from the client.

## Running locally

```bash
cargo run -p judge-api
# or with custom port
PORT=8080 cargo run -p judge-api
```

The server binds to `0.0.0.0:3000` by default. Override with the `PORT` environment variable.

## Endpoints

| Method | Path      | Description                              |
|--------|-----------|------------------------------------------|
| GET    | `/`       | Hello / version                          |
| GET    | `/health` | Health check                             |
| GET    | `/cards`  | Full compiled card database (JSON array) |

## Environment variables

| Variable     | Default | Description                                             |
|--------------|---------|---------------------------------------------------------|
| `PORT`       | `3000`  | Port to listen on                                       |
| `CARDS_FILE` | —       | Path to the compiled `judge-cards.json` file. If unset, `/cards` returns 404. |

## Generating the cards file

Use the `compile_cards` tool to compile a Scryfall all-cards dump into the compact format:

```bash
cargo run -p judge-api --bin compile_cards --release -- \
  /path/to/all-cards-YYYYMMDD.json \
  /path/to/judge-cards.json
```

This produces a single JSON array of ~33k oracle cards, each with the full list of set printings. Upload `judge-cards.json` to the server and point `CARDS_FILE` at it.

## Deployment on Digital Ocean

### 1. Create a Droplet

- **Image**: Ubuntu 24.04 LTS
- **Size**: Basic, 1 GB RAM / 1 vCPU is sufficient to start
- **Region**: Choose one close to your users
- Add your SSH key during creation

### 2. Install Rust on the Droplet

```bash
ssh root@<your-droplet-ip>
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### 3. Copy and build

```bash
# On your local machine — copy the repo
scp -r /path/to/thejudgeapp root@<your-droplet-ip>:/opt/thejudgeapp

# On the droplet
cd /opt/thejudgeapp
cargo build --release -p judge-api
```

The binary ends up at `target/release/judge-api`.

### 4. Run as a systemd service

Create `/etc/systemd/system/judge-api.service`:

```ini
[Unit]
Description=Judge API
After=network.target

[Service]
ExecStart=/opt/thejudgeapp/target/release/judge-api
Restart=always
RestartSec=5
Environment=PORT=3000
Environment=RUST_LOG=judge_api=info,tower_http=info
Environment=CARDS_FILE=/opt/thejudgeapp/judge-cards.json
WorkingDirectory=/opt/thejudgeapp

[Install]
WantedBy=multi-user.target
```

Enable and start it:

```bash
systemctl daemon-reload
systemctl enable judge-api
systemctl start judge-api
systemctl status judge-api
```

### 5. Open the firewall

```bash
ufw allow 3000/tcp
ufw enable
```

Or put Nginx in front and proxy to port 3000 (recommended for TLS):

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Then use Certbot for a free TLS certificate:

```bash
apt install certbot python3-certbot-nginx
certbot --nginx -d your-domain.com
```

### 6. Verify

```bash
curl http://your-droplet-ip:3000/health
# ok

curl http://your-droplet-ip:3000/
# {"message":"Hello from the Judge API","version":"0.1.0"}
```
