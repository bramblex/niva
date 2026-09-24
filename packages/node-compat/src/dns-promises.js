import "./runtime/bridge.js";
import "./runtime/dns.js";

const promises = globalThis[Symbol.for("niva.node-compat.runtime")].dns.promises;
export const lookup = promises.lookup;
export const resolve = promises.resolve;
export const resolve4 = promises.resolve4;
export const resolve6 = promises.resolve6;
export const resolveCname = promises.resolveCname;
export const resolveMx = promises.resolveMx;
export const resolveTxt = promises.resolveTxt;
export const resolveNs = promises.resolveNs;
export const resolveSrv = promises.resolveSrv;
export const resolveSoa = promises.resolveSoa;
export const resolvePtr = promises.resolvePtr;
export const Resolver = promises.Resolver;
export default promises;
