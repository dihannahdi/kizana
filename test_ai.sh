#!/bin/bash
# Test AI API key validity
echo '{"model":"grok-3-mini","messages":[{"role":"user","content":"test"}],"max_tokens":10,"stream":true}' > /tmp/ai_test.json
RESP=$(curl -s -w "\nHTTP_STATUS:%{http_code}" https://api.x.ai/v1/chat/completions \
  -H "Authorization: Bearer sk-61f519039ee1434b9592fe253d21998a" \
  -H "Content-Type: application/json" \
  -d @/tmp/ai_test.json)
echo "Response: $RESP"
