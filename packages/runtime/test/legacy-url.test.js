import "./setup-runtime.js";
import test from 'node:test';
import assert from 'node:assert/strict';
import * as nodeUrl from 'node:url';
import url from '../dist/source/url.js';

test('legacy URL objects preserve HTTP routing fields, escapes and relative resolution', () => {
  for (const input of ['http://example.test:8123/foo?bar#fragment', 'https://[::1]:8443/a%20b?x=1', 'https://example.test/a/b', '/a/b?x=1']) {
    const actual=url.parse(input), expected=nodeUrl.parse(input);
    for (const name of ['protocol','host','hostname','port','auth','pathname','search','hash','path','href']) assert.equal(actual[name],expected[name],name);
    assert.equal(url.format(actual),nodeUrl.format(expected));
  }
  for (const [base,relative] of [['/one/two/three','four'],['https://example.test/a/b','../c?x=2'],['https://example.test/a','//another.test/b']]) {
    assert.equal(url.resolve(base,relative),nodeUrl.resolve(base,relative));
  }
});
