import test from 'node:test';
import assert from 'node:assert/strict';
import * as native from 'node:crypto';
import crypto from '../src/crypto.js';
import { Buffer } from '../src/buffer.js';

test('secret KeyObjects copy key material, export consistently and work with HMAC', () => {
  const input=Buffer.from([0,1,2,255]);
  const key=crypto.createSecretKey(input), expected=native.createSecretKey(input);
  input.fill(0);
  assert.equal(key.type,'secret');
  assert.equal(key.symmetricKeySize,expected.symmetricKeySize);
  assert.deepEqual([...key.export()],[...expected.export()]);
  assert.deepEqual(key.export({format:'jwk'}),expected.export({format:'jwk'}));
  key.export().fill(255);
  assert.equal(key.equals(crypto.createSecretKey(Buffer.from([0,1,2,255]))),true);
  assert.equal(key.equals(crypto.createSecretKey(Buffer.from([0,1,2,254]))),false);
  assert.equal(crypto.createHmac('sha256',key).update('payload').digest('hex'),native.createHmac('sha256',expected).update('payload').digest('hex'));
  assert.equal(crypto.createSecretKey(Buffer.alloc(0)).equals(crypto.createSecretKey(Buffer.alloc(0))),true);
  assert.throws(()=>key.equals({}),{code:'ERR_INVALID_ARG_TYPE'});
  assert.throws(()=>new crypto.KeyObject(),{code:'ERR_ILLEGAL_CONSTRUCTOR'});
});
