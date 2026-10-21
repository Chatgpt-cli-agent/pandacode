import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

// Static regression tests: they do not claim a live OpenClaw Gateway handshake.
const bridge = readFileSync(resolve(import.meta.dirname, '..', 'panda-gateway.mjs'), 'utf8');
test('uses official Gateway client, not a custom WebSocket handshake', () => {
  assert.match(bridge, /import \{ GatewayClient, mergeChatStreamMessage \} from '@openclaw\/gateway-client'/);
  assert.doesNotMatch(bridge, /new WebSocket\(/);
});
test('requires isolated Panda session identity', () => {
  assert.match(bridge, /startsWith\('agent:panda:'\)/);
});
test('uses official chat reducer and run idempotency', () => {
  assert.match(bridge, /mergeChatStreamMessage\(lastMessage, p\)/);
  assert.match(bridge, /idempotencyKey:key/);
});
test('supports authenticated Gateway shutdown and abort', () => {
  assert.match(bridge, /chat\.abort/);
  assert.match(bridge, /stopAndWait\(\)/);
});
