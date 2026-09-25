
const dns = globalThis.Niva.dns;
export const lookup = dns.lookup;
export const resolve = dns.resolve;
export const resolve4 = dns.resolve4;
export const resolve6 = dns.resolve6;
export const resolveCname = dns.resolveCname;
export const resolveMx = dns.resolveMx;
export const resolveTxt = dns.resolveTxt;
export const resolveNs = dns.resolveNs;
export const resolveSrv = dns.resolveSrv;
export const resolveSoa = dns.resolveSoa;
export const resolvePtr = dns.resolvePtr;
export const Resolver = dns.Resolver;
export const setServers = dns.setServers;
export const getServers = dns.getServers;
export const promises = dns.promises;
export default dns;
