import { Buffer } from "buffer";
import ReadableStream from "readable-stream";
import { StringDecoder } from "string_decoder";
import { sha256, sha384, sha512 } from "@noble/hashes/sha2.js";
import { sha1, md5 } from "@noble/hashes/legacy.js";
import { hmac } from "@noble/hashes/hmac.js";
import { pbkdf2, pbkdf2Async } from "@noble/hashes/pbkdf2.js";
import { scrypt, scryptAsync } from "@noble/hashes/scrypt.js";
import { gzip, gunzip, gzipSync, gunzipSync, Gunzip } from "fflate/browser";
import dnsPacket from "dns-packet";
import legacyUrl from "url/url.js";
import * as acorn from "acorn";
import * as acornWalk from "acorn-walk";
import { HTTPParser } from "http-parser-js";

const stream = {
  Stream: ReadableStream.Stream,
  Readable: ReadableStream.Readable,
  Writable: ReadableStream.Writable,
  Duplex: ReadableStream.Duplex,
  Transform: ReadableStream.Transform,
  PassThrough: ReadableStream.PassThrough,
  finished: ReadableStream.finished,
  pipeline: ReadableStream.pipeline,
  promises: ReadableStream.promises,
  addAbortSignal: ReadableStream.addAbortSignal,
  compose: ReadableStream.compose,
  destroy: ReadableStream.destroy,
  from: ReadableStream.from,
  fromWeb: ReadableStream.fromWeb,
  toWeb: ReadableStream.toWeb,
  wrap: ReadableStream.wrap,
  isDisturbed: ReadableStream.isDisturbed,
  isErrored: ReadableStream.isErrored,
  isReadable: ReadableStream.isReadable,
};

const compression = { gzip, gunzip, gzipSync, gunzipSync };
Object.defineProperty(compression, "Gunzip", { value: Gunzip });

const vendor = {
  Buffer,
  stream,
  StringDecoder,
  hashes: {
    sha256,
    sha384,
    sha512,
    sha1,
    md5,
    hmac,
    pbkdf2,
    pbkdf2Async,
    scrypt,
    scryptAsync,
  },
  compression,
  dnsPacket,
  legacyUrl,
  acorn,
  acornWalk,
  HTTPParser,
};

const key = Symbol.for("niva.node-compat.runtime");
const runtime = globalThis[key] || {};
runtime.vendor = vendor;
globalThis[key] = runtime;
