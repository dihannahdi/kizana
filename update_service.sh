#!/bin/bash
# Add DeepSeek API URL and model after the AI_API_KEY line
sed -i '/^Environment=AI_API_KEY=/a Environment=AI_API_URL=https://api.deepseek.com/v1/chat/completions\nEnvironment=AI_MODEL=deepseek-chat' /etc/systemd/system/kizana-backend.service

systemctl daemon-reload
echo "Updated service file:"
grep 'Environment=' /etc/systemd/system/kizana-backend.service
