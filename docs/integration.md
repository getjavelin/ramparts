# Integration Patterns

This document provides examples and patterns for integrating Ramparts into various systems and workflows.

## CI/CD Pipeline Integration

### GitHub Actions Example
```yaml
# GitHub Actions example
- name: Scan MCP Server
  run: |
    ramparts server --port 3000 &
    SERVER_PID=$!
    sleep 5  # Wait for server to start
    
    # Scan your MCP server
    curl -X POST http://localhost:3000/scan \
      -H "Content-Type: application/json" \
      -d '{"url": "${{ secrets.MCP_SERVER_URL }}", "detailed": true}' \
      | jq '.result.security_issues.total_issues'
    
    kill $SERVER_PID
```

### GitLab CI Example
```yaml
scan_mcp_servers:
  stage: security
  script:
    - ramparts server --port 3000 &
    - SERVER_PID=$!
    - sleep 5
    - |
      for url in $MCP_SERVER_URLS; do
        curl -X POST http://localhost:3000/scan \
          -H "Content-Type: application/json" \
          -d "{\"url\": \"$url\", \"detailed\": true}"
      done
    - kill $SERVER_PID
  only:
    - main
    - develop
```

### Jenkins Pipeline Example
```groovy
pipeline {
    agent any
    stages {
        stage('MCP Security Scan') {
            steps {
                script {
                    sh '''
                        ramparts server --port 3000 &
                        SERVER_PID=$!
                        sleep 5
                        
                        curl -X POST http://localhost:3000/scan \
                          -H "Content-Type: application/json" \
                          -d '{"url": "${MCP_SERVER_URL}", "detailed": true}' \
                          -o scan_results.json
                        
                        kill $SERVER_PID
                        
                        # Check for critical issues
                        CRITICAL_COUNT=$(jq '.result.security_issues.critical_count' scan_results.json)
                        if [ "$CRITICAL_COUNT" -gt 0 ]; then
                            echo "Critical security issues found!"
                            exit 1
                        fi
                    '''
                }
            }
        }
    }
}
```

## Docker Deployment

### Basic Dockerfile
```dockerfile
FROM ubuntu:22.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    curl \
    jq \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy ramparts binary
COPY ramparts /usr/local/bin/ramparts
RUN chmod +x /usr/local/bin/ramparts

# Copy configuration
COPY config.yaml /app/config.yaml
WORKDIR /app

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:3000/health || exit 1

# Run server
CMD ["ramparts", "server", "--port", "3000", "--host", "0.0.0.0"]
```

### Docker Compose
```yaml
version: '3.8'

services:
  ramparts:
    build: .
    ports:
      - "3000:3000"
    environment:
      - RUST_LOG=info
    volumes:
      - ./config.yaml:/app/config.yaml:ro
      - ./rules:/app/rules:ro
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    restart: unless-stopped

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - ramparts
    restart: unless-stopped
```

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ramparts
  labels:
    app: ramparts
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ramparts
  template:
    metadata:
      labels:
        app: ramparts
    spec:
      containers:
      - name: ramparts
        image: ramparts:latest
        ports:
        - containerPort: 3000
        env:
        - name: RUST_LOG
          value: "info"
        livenessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
        resources:
          requests:
            memory: "128Mi"
            cpu: "100m"
          limits:
            memory: "512Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: ramparts-service
spec:
  selector:
    app: ramparts
  ports:
    - protocol: TCP
      port: 80
      targetPort: 3000
  type: LoadBalancer
```

## Health Monitoring

### Basic Health Check Script
```bash
#!/bin/bash
# Health check script
response=$(curl -s http://localhost:3000/health)
if echo "$response" | jq -e '.status == "healthy"' > /dev/null; then
  echo "Ramparts server is healthy"
  exit 0
else
  echo "Ramparts server is unhealthy"
  exit 1
fi
```

### Advanced Monitoring with Alerts
```bash
#!/bin/bash
# Enhanced health monitoring with Slack notifications

SLACK_WEBHOOK_URL="YOUR_SLACK_WEBHOOK_URL"
RAMPARTS_URL="http://localhost:3000"

check_health() {
    local response=$(curl -s -w "%{http_code}" "$RAMPARTS_URL/health")
    local body=$(echo "$response" | head -n -1)
    local status_code=$(echo "$response" | tail -n 1)
    
    if [ "$status_code" -eq 200 ]; then
        if echo "$body" | jq -e '.status == "healthy"' > /dev/null; then
            return 0
        fi
    fi
    return 1
}

send_alert() {
    local message="$1"
    curl -X POST -H 'Content-type: application/json' \
        --data "{\"text\":\"🚨 Ramparts Alert: $message\"}" \
        "$SLACK_WEBHOOK_URL"
}

# Check health
if ! check_health; then
    send_alert "Ramparts server is unhealthy or unresponsive"
    exit 1
else
    echo "Ramparts server is healthy"
fi
```

### Prometheus Metrics Integration
```bash
#!/bin/bash
# Export metrics for Prometheus

METRICS_FILE="/tmp/ramparts_metrics.prom"
RAMPARTS_URL="http://localhost:3000"

# Get health status
health_response=$(curl -s "$RAMPARTS_URL/health")
if echo "$health_response" | jq -e '.status == "healthy"' > /dev/null; then
    health_status=1
else
    health_status=0
fi

# Write metrics
cat > "$METRICS_FILE" << EOF
# HELP ramparts_health_status Health status of Ramparts server (1 = healthy, 0 = unhealthy)
# TYPE ramparts_health_status gauge
ramparts_health_status $health_status

# HELP ramparts_version_info Version information
# TYPE ramparts_version_info gauge
ramparts_version_info{version="$(echo "$health_response" | jq -r '.version')"} 1
EOF

echo "Metrics exported to $METRICS_FILE"
```

## Load Balancer Configuration

### Nginx Configuration
```nginx
# Nginx configuration for load balancing
upstream ramparts {
    server 127.0.0.1:3000;
    server 127.0.0.1:3001;
    server 127.0.0.1:3002;
}

server {
    listen 80;
    server_name ramparts.example.com;
    
    # Health check endpoint
    location /health {
        proxy_pass http://ramparts;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_connect_timeout 5s;
        proxy_send_timeout 10s;
        proxy_read_timeout 10s;
    }
    
    # API endpoints
    location / {
        proxy_pass http://ramparts;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_connect_timeout 10s;
        proxy_send_timeout 60s;
        proxy_read_timeout 300s;  # Allow longer for scans
    }
    
    # CORS headers
    add_header Access-Control-Allow-Origin *;
    add_header Access-Control-Allow-Methods "GET, POST, OPTIONS";
    add_header Access-Control-Allow-Headers *;
}
```

### HAProxy Configuration
```
global
    daemon
    log stdout local0

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    log global
    option httplog

frontend ramparts_frontend
    bind *:80
    default_backend ramparts_servers

backend ramparts_servers
    balance roundrobin
    option httpchk GET /health
    http-check expect status 200
    server ramparts1 127.0.0.1:3000 check
    server ramparts2 127.0.0.1:3001 check
    server ramparts3 127.0.0.1:3002 check
```

## Database Integration

### Storing Scan Results
```python
import requests
import sqlite3
import json
from datetime import datetime

class RampartsScanLogger:
    def __init__(self, db_path="ramparts_scans.db"):
        self.db_path = db_path
        self.init_db()
    
    def init_db(self):
        conn = sqlite3.connect(self.db_path)
        cursor = conn.cursor()
        cursor.execute('''
            CREATE TABLE IF NOT EXISTS scan_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                timestamp DATETIME NOT NULL,
                status TEXT NOT NULL,
                response_time_ms INTEGER,
                total_tools INTEGER,
                security_issues_count INTEGER,
                yara_matches INTEGER,
                result_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        ''')
        conn.commit()
        conn.close()
    
    def scan_and_store(self, mcp_url, ramparts_url="http://localhost:3000"):
        # Perform scan
        response = requests.post(f"{ramparts_url}/scan", json={
            "url": mcp_url,
            "detailed": True,
            "format": "json"
        })
        
        if response.status_code == 200:
            data = response.json()
            if data['success']:
                result = data['result']
                
                # Store in database
                conn = sqlite3.connect(self.db_path)
                cursor = conn.cursor()
                cursor.execute('''
                    INSERT INTO scan_results 
                    (url, timestamp, status, response_time_ms, total_tools, 
                     security_issues_count, yara_matches, result_json)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ''', (
                    result['url'],
                    result['timestamp'],
                    str(result['status']),
                    result['response_time_ms'],
                    len(result['tools']),
                    result['security_issues']['total_issues'] if result['security_issues'] else 0,
                    len(result['yara_results']),
                    json.dumps(result)
                ))
                conn.commit()
                conn.close()
                return True
        
        return False

# Usage
logger = RampartsScanLogger()
logger.scan_and_store("https://api.example.com/mcp/")
```

## Scheduled Scanning

### Cron Job Example
```bash
# Add to crontab: crontab -e
# Scan MCP servers every 6 hours
0 */6 * * * /path/to/scan-mcp-servers.sh

# scan-mcp-servers.sh
#!/bin/bash

RAMPARTS_URL="http://localhost:3000"
MCP_SERVERS=(
    "https://api.server1.com/mcp/"
    "https://api.server2.com/mcp/"
    "stdio:///usr/local/bin/my-mcp-server"
)

LOG_FILE="/var/log/ramparts-scheduled.log"
DATE=$(date '+%Y-%m-%d %H:%M:%S')

echo "[$DATE] Starting scheduled MCP scan" >> "$LOG_FILE"

for server in "${MCP_SERVERS[@]}"; do
    echo "[$DATE] Scanning: $server" >> "$LOG_FILE"
    
    response=$(curl -s -X POST "$RAMPARTS_URL/scan" \
        -H "Content-Type: application/json" \
        -d "{\"url\": \"$server\", \"detailed\": true}")
    
    if echo "$response" | jq -e '.success' > /dev/null; then
        issues=$(echo "$response" | jq '.result.security_issues.total_issues // 0')
        echo "[$DATE] $server: $issues security issues found" >> "$LOG_FILE"
        
        # Alert if critical issues found
        critical=$(echo "$response" | jq '.result.security_issues.critical_count // 0')
        if [ "$critical" -gt 0 ]; then
            echo "[$DATE] ALERT: $critical critical issues in $server" >> "$LOG_FILE"
            # Send notification (email, Slack, etc.)
        fi
    else
        echo "[$DATE] ERROR: Failed to scan $server" >> "$LOG_FILE"
    fi
done

echo "[$DATE] Scheduled scan completed" >> "$LOG_FILE"
```

### systemd Timer
```ini
# /etc/systemd/system/ramparts-scan.service
[Unit]
Description=Ramparts MCP Security Scan
After=network.target

[Service]
Type=oneshot
ExecStart=/usr/local/bin/scan-mcp-servers.sh
User=ramparts
Group=ramparts

# /etc/systemd/system/ramparts-scan.timer
[Unit]
Description=Run Ramparts MCP scan every 6 hours
Requires=ramparts-scan.service

[Timer]
OnCalendar=*-*-* 00,06,12,18:00:00
Persistent=true

[Install]
WantedBy=timers.target
```

Enable the timer:
```bash
sudo systemctl enable ramparts-scan.timer
sudo systemctl start ramparts-scan.timer
```

## Webhook Integration

### Webhook Notification Example
```python
import requests
import json

def scan_with_webhook(mcp_url, webhook_url):
    # Perform scan
    response = requests.post("http://localhost:3000/scan", json={
        "url": mcp_url,
        "detailed": True
    })
    
    if response.status_code == 200:
        data = response.json()
        
        # Send webhook notification
        webhook_data = {
            "event": "scan_completed",
            "mcp_url": mcp_url,
            "success": data['success'],
            "timestamp": data['timestamp']
        }
        
        if data['success']:
            result = data['result']
            webhook_data.update({
                "total_tools": len(result['tools']),
                "security_issues": result['security_issues']['total_issues'] if result['security_issues'] else 0,
                "yara_matches": len(result['yara_results'])
            })
        
        requests.post(webhook_url, json=webhook_data)

# Usage
scan_with_webhook(
    "https://api.example.com/mcp/",
    "https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK"
)
```