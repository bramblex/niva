// Copyright Joyent, Inc. and other Node contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the
// "Software"), to deal in the Software without restriction, including
// without limitation the rights to use, copy, modify, merge, publish,
// distribute, sublicense, and/or sell copies of the Software, and to permit
// persons to whom the Software is furnished to do so, subject to the
// following conditions:
//
// The above copyright notice and this permission notice shall be included
// in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
// MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
// NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
// DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
// USE OR OTHER DEALINGS IN THE SOFTWARE.
// Adapted from Node.js v22.14.0, commit 5d2feb257bcee090e57900eb51720171a6aa92f3:
// https://github.com/nodejs/node/blob/5d2feb257bcee090e57900eb51720171a6aa92f3/lib/path.js
(function (root: any) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];
  if (typeof runtime.createPathModule === "function") return;

  function detectPlatform() {
    if (root.process && root.process.platform) return root.process.platform;
    var platform = "";
    if (typeof navigator !== "undefined") {
      platform = (navigator as any).userAgentData && (navigator as any).userAgentData.platform || navigator.platform || "";
    }
    return /win/i.test(platform) ? "win32" : /mac|darwin/i.test(platform) ? "darwin" : "linux";
  }

  function initialCwd() {
    try {
      if (typeof process !== "undefined" && typeof process.cwd === "function") return process.cwd();
    } catch (_) {}
    return "/";
  }

  function createPathModule() {
    var cwd = initialCwd();
    var cwdOverride = false;

    function currentCwd(apiName?) {
      if (cwdOverride) return cwd;
      var processModule = root.process;
      if (processModule && typeof processModule.cwd === "function" &&
          (processModule === runtime.process || processModule.versions && processModule.versions.niva)) {
        if (processModule.__nivaAvailable === true && runtime.callSyncAs && root.Niva) {
          return runtime.callSyncAs(root.Niva, apiName || "path.getCwd", "process.currentDir", []);
        }
        return processModule.cwd();
      }
      if (runtime.callSyncAs && root.Niva && root.Niva.bridge && typeof root.Niva.bridge.callSync === "function") {
        return runtime.callSyncAs(root.Niva, apiName || "path.getCwd", "process.currentDir", []);
      }
      if (processModule && typeof processModule.cwd === "function") return processModule.cwd();
      return cwd;
    }

    function makePath(windows) {
      var sep = windows ? "\\" : "/";
      var delimiter = windows ? ";" : ":";
      var isSep = windows
        ? function (code) { return code === 47 || code === 92; }
        : function (code) { return code === 47; };

      function isWindowsDeviceRoot(code) {
        return code >= 65 && code <= 90 || code >= 97 && code <= 122;
      }

      function winRoot(path) {
        var length = path.length;
        var device = "";
        var rootEnd = 0;
        var absolute = false;

        if (length >= 2 && isSep(path.charCodeAt(0)) && isSep(path.charCodeAt(1))) {
          var extended = length >= 4 && (path.charCodeAt(2) === 63 || path.charCodeAt(2) === 46) && isSep(path.charCodeAt(3));
          var serverStart = extended ? 8 : 2;
          if (extended && length >= 8 && path.slice(4, 7).toLowerCase() === "unc" && isSep(path.charCodeAt(7))) {
            return { device: "\\\\?\\UNC", rootEnd: 8, absolute: true, unc: false };
          } else if (extended) {
            var extendedDeviceEnd = 4;
            while (extendedDeviceEnd < length && !isSep(path.charCodeAt(extendedDeviceEnd))) extendedDeviceEnd += 1;
            if (extendedDeviceEnd < length) {
              device = path.slice(0, extendedDeviceEnd).replace(/\//g, "\\");
              rootEnd = extendedDeviceEnd + 1;
              absolute = true;
              return { device: device, rootEnd: rootEnd, absolute: absolute, unc: false };
            }
          }

          var serverEnd = serverStart;
          while (serverEnd < length && !isSep(path.charCodeAt(serverEnd))) serverEnd += 1;
          if (serverEnd > serverStart && serverEnd < length) {
            var shareStart = serverEnd;
            while (shareStart < length && isSep(path.charCodeAt(shareStart))) shareStart += 1;
            var shareEnd = shareStart;
            while (shareEnd < length && !isSep(path.charCodeAt(shareEnd))) shareEnd += 1;
            if (shareEnd > shareStart && shareStart > serverEnd) {
              var prefix = extended ? "\\\\?\\UNC\\" : "\\\\";
              device = prefix + path.slice(serverStart, serverEnd) + "\\" + path.slice(shareStart, shareEnd);
              rootEnd = shareEnd;
              if (shareEnd < length && isSep(path.charCodeAt(shareEnd))) rootEnd += 1;
              absolute = true;
              return { device: device, rootEnd: rootEnd, absolute: absolute, unc: true };
            }
          }
        }

        if (length >= 2 && isWindowsDeviceRoot(path.charCodeAt(0)) && path.charCodeAt(1) === 58) {
          device = path.slice(0, 2);
          rootEnd = length >= 3 && isSep(path.charCodeAt(2)) ? 3 : 2;
          absolute = rootEnd === 3;
          return { device: device, rootEnd: rootEnd, absolute: absolute, unc: false };
        }

        if (length > 0 && isSep(path.charCodeAt(0))) {
          rootEnd = 1;
          absolute = true;
        }
        return { device: device, rootEnd: rootEnd, absolute: absolute, unc: false };
      }

      function normalizeString(path, allowAboveRoot) {
        var result = "";
        var lastSegmentLength = 0;
        var lastSlash = -1;
        var dots = 0;
        var code = 0;
        for (var i = 0; i <= path.length; i += 1) {
          if (i < path.length) code = path.charCodeAt(i);
          else if (isSep(code)) break;
          else code = 47;
          if (isSep(code)) {
            if (lastSlash === i - 1 || dots === 1) {
              // Empty and dot segments are removed.
            } else if (dots === 2) {
              if (result.length < 2 || lastSegmentLength !== 2 ||
                  result.charCodeAt(result.length - 1) !== 46 ||
                  result.charCodeAt(result.length - 2) !== 46) {
                if (result.length > 2) {
                  var lastSlashIndex = result.lastIndexOf(sep);
                  if (lastSlashIndex === -1) {
                    result = "";
                    lastSegmentLength = 0;
                  } else {
                    result = result.slice(0, lastSlashIndex);
                    lastSegmentLength = result.length - 1 - result.lastIndexOf(sep);
                  }
                  lastSlash = i;
                  dots = 0;
                  continue;
                } else if (result.length !== 0) {
                  result = "";
                  lastSegmentLength = 0;
                  lastSlash = i;
                  dots = 0;
                  continue;
                }
              }
              if (allowAboveRoot) {
                result += result.length > 0 ? sep + ".." : "..";
                lastSegmentLength = 2;
              }
            } else {
              var segment = path.slice(lastSlash + 1, i);
              result = result.length > 0 ? result + sep + segment : segment;
              lastSegmentLength = i - lastSlash - 1;
            }
            lastSlash = i;
            dots = 0;
          } else if (code === 46 && dots !== -1) {
            dots += 1;
          } else {
            dots = -1;
          }
        }
        return result;
      }

      function normalize(path) {
        assertPath(path);
        if (path.length === 0) return ".";
        if (!windows) {
          var absolutePosix = path.charCodeAt(0) === 47;
          var trailingPosix = path.charCodeAt(path.length - 1) === 47;
          var posixTail = normalizeString(path, !absolutePosix);
          if (!posixTail && !absolutePosix) posixTail = ".";
          if (posixTail && trailingPosix) posixTail += "/";
          return (absolutePosix ? "/" : "") + posixTail;
        }

        var length = path.length;
        var rootEnd = 0;
        var device;
        var absoluteWin = false;
        var firstCode = path.charCodeAt(0);
        if (length === 1) return firstCode === 47 ? "\\" : path;
        if (isSep(firstCode)) {
          absoluteWin = true;
          if (isSep(path.charCodeAt(1))) {
            var serverStart = 2;
            var serverEnd = serverStart;
            while (serverEnd < length && !isSep(path.charCodeAt(serverEnd))) serverEnd += 1;
            if (serverEnd < length && serverEnd !== serverStart) {
              var serverName = path.slice(serverStart, serverEnd);
              var shareStart = serverEnd;
              while (shareStart < length && isSep(path.charCodeAt(shareStart))) shareStart += 1;
              if (shareStart < length && shareStart !== serverEnd) {
                var shareEnd = shareStart;
                while (shareEnd < length && !isSep(path.charCodeAt(shareEnd))) shareEnd += 1;
                if (shareEnd === length) return "\\\\" + serverName + "\\" + path.slice(shareStart) + "\\";
                if (shareEnd !== shareStart) {
                  device = "\\\\" + serverName + "\\" + path.slice(shareStart, shareEnd);
                  rootEnd = shareEnd;
                }
              }
            }
          } else {
            rootEnd = 1;
          }
        } else if (isWindowsDeviceRoot(firstCode) && path.charCodeAt(1) === 58) {
          device = path.slice(0, 2);
          rootEnd = 2;
          if (length > 2 && isSep(path.charCodeAt(2))) {
            absoluteWin = true;
            rootEnd = 3;
          }
        }

        var tail = rootEnd < length ? normalizeString(path.slice(rootEnd), !absoluteWin) : "";
        if (!tail && !absoluteWin) tail = ".";
        if (tail && isSep(path.charCodeAt(length - 1))) tail += "\\";
        if (!absoluteWin && device === undefined && path.indexOf(":") !== -1) {
          if (tail.length >= 2 && isWindowsDeviceRoot(tail.charCodeAt(0)) && tail.charCodeAt(1) === 58) {
            return ".\\" + tail;
          }
          var colonIndex = path.indexOf(":");
          while (colonIndex !== -1) {
            if (colonIndex === length - 1 || isSep(path.charCodeAt(colonIndex + 1))) return ".\\" + tail;
            colonIndex = path.indexOf(":", colonIndex + 1);
          }
        }
        if (device === undefined) return absoluteWin ? "\\" + tail : tail;
        return absoluteWin ? device + "\\" + tail : device + tail;
      }

      function resolveFor(apiName, paths) {
        var baseCwd = currentCwd(apiName);
        if (!windows) {
          var posixTail = "";
          var posixAbsolute = false;
          var posixArgs = paths;
          for (var posixIndex = posixArgs.length - 1; posixIndex >= -1 && !posixAbsolute; posixIndex -= 1) {
            var posixPath = posixIndex >= 0 ? posixArgs[posixIndex] : baseCwd;
            assertPath(posixPath, posixIndex >= 0 ? "paths[" + posixIndex + "]" : "path");
            if (posixPath.length === 0) continue;
            if (posixPath.charCodeAt(0) === 47) posixAbsolute = true;
            posixTail = posixPath + "/" + posixTail;
          }
          var posixResolved = normalizeString(posixTail, !posixAbsolute);
          return (posixAbsolute ? "/" : "") + (posixResolved || (posixAbsolute ? "" : "."));
        }

        var resolvedDevice = "";
        var resolvedTail = "";
        var resolvedAbsolute = false;
        var args = paths;
        for (var i = args.length - 1; i >= -1; i -= 1) {
          var path;
          if (i >= 0) {
            path = args[i];
            assertPath(path, "paths[" + i + "]");
            if (path.length === 0) continue;
          } else if (!resolvedDevice) {
            path = baseCwd;
          } else {
            var cwdInfo = winRoot(baseCwd);
            path = cwdInfo.device.toLowerCase() === resolvedDevice.toLowerCase() ? baseCwd : resolvedDevice + "\\";
          }
          var rootInfo = winRoot(path);
          var device = rootInfo.device;
          if (device) {
            if (resolvedDevice && device.toLowerCase() !== resolvedDevice.toLowerCase()) continue;
            if (!resolvedDevice) resolvedDevice = device;
          }
          if (resolvedAbsolute) {
            if (resolvedDevice) break;
          } else {
            var tail = path.slice(rootInfo.rootEnd);
            resolvedTail = tail + "\\" + resolvedTail;
            resolvedAbsolute = rootInfo.absolute;
            if (resolvedAbsolute && resolvedDevice) break;
          }
        }
        var normalizedTail = normalizeString(resolvedTail, !resolvedAbsolute);
        if (resolvedAbsolute) return resolvedDevice + "\\" + normalizedTail;
        var resolved = resolvedDevice + normalizedTail;
        return resolved || ".";
      }

      function resolve(...paths: any[]) {
        return resolveFor("path.resolve", paths);
      }

      function isAbsolute(path) {
        assertPath(path);
        if (!windows) return path.length > 0 && path.charCodeAt(0) === 47;
        return winRoot(path).absolute;
      }

      function join() {
        var parts = Array.prototype.slice.call(arguments);
        if (windows) {
          if (parts.length === 0) return ".";
          var winJoined;
          var firstPart;
          for (var winIndex = 0; winIndex < parts.length; winIndex += 1) {
            var winPart = parts[winIndex];
            assertPath(winPart);
            if (winPart.length > 0) {
              if (winJoined === undefined) winJoined = firstPart = winPart;
              else winJoined += "\\" + winPart;
            }
          }
          if (winJoined === undefined) return ".";
          var needsReplace = true;
          var slashCount = 0;
          if (isSep(firstPart.charCodeAt(0))) {
            slashCount += 1;
            if (firstPart.length > 1 && isSep(firstPart.charCodeAt(1))) {
              slashCount += 1;
              if (firstPart.length > 2) {
                if (isSep(firstPart.charCodeAt(2))) slashCount += 1;
                else needsReplace = false;
              }
            }
          }
          if (needsReplace) {
            while (slashCount < winJoined.length && isSep(winJoined.charCodeAt(slashCount))) slashCount += 1;
            if (slashCount >= 2) winJoined = "\\" + winJoined.slice(slashCount);
          }
          return normalize(winJoined);
        }
        var joined = "";
        for (var i = 0; i < parts.length; i += 1) {
          var part = parts[i];
          assertPath(part);
          if (part.length > 0) joined = joined ? joined + sep + part : part;
        }
        if (!joined) return ".";
        return normalize(joined);
      }

      function trimTrailing(path) {
        var end = path.length;
        while (end > 0 && isSep(path.charCodeAt(end - 1))) end -= 1;
        return end;
      }

      function basename(path, suffix?) {
        if (suffix !== undefined) assertPath(suffix, "suffix");
        assertPath(path);
        var start = 0;
        var end = -1;
        var matchedSlash = true;
        if (windows && path.length >= 2 && isWindowsDeviceRoot(path.charCodeAt(0)) && path.charCodeAt(1) === 58) {
          start = 2;
        }
        if (suffix !== undefined && suffix.length > 0 && suffix.length <= path.length) {
          if (suffix === path) return "";
          var extIndex = suffix.length - 1;
          var firstNonSlashEnd = -1;
          for (var i = path.length - 1; i >= start; i -= 1) {
            var code = path.charCodeAt(i);
            if (isSep(code)) {
              if (!matchedSlash) {
                start = i + 1;
                break;
              }
            } else {
              if (firstNonSlashEnd === -1) {
                matchedSlash = false;
                firstNonSlashEnd = i + 1;
              }
              if (extIndex >= 0) {
                if (code === suffix.charCodeAt(extIndex)) {
                  if (--extIndex === -1) end = i;
                } else {
                  extIndex = -1;
                  end = firstNonSlashEnd;
                }
              }
            }
          }
          if (start === end) end = firstNonSlashEnd;
          else if (end === -1) end = path.length;
          return path.slice(start, end);
        }
        for (var j = path.length - 1; j >= start; j -= 1) {
          if (isSep(path.charCodeAt(j))) {
            if (!matchedSlash) {
              start = j + 1;
              break;
            }
          } else if (end === -1) {
            matchedSlash = false;
            end = j + 1;
          }
        }
        if (end === -1) return "";
        return path.slice(start, end);
      }

      function dirname(path) {
        assertPath(path);
        if (path.length === 0) return ".";
        var rootInfo = windows ? winRoot(path) : { device: "", rootEnd: path.charCodeAt(0) === 47 ? 1 : 0, absolute: path.charCodeAt(0) === 47 };
        function rootString() {
          if (!windows) {
            if (path.length > 2 && path.charAt(0) === "/" && path.charAt(1) === "/" && path.charAt(2) !== "/") return "//";
            return "/";
          }
          if (rootInfo.rootEnd > rootInfo.device.length) {
            return rootInfo.device + path.charAt(rootInfo.rootEnd - 1);
          }
          return rootInfo.device || (rootInfo.absolute ? path.charAt(0) : "");
        }
        var end = trimTrailing(path);
        if (end <= rootInfo.rootEnd) {
          if (rootInfo.device && !rootInfo.absolute) return rootInfo.device;
          return rootInfo.absolute ? rootString() : ".";
        }
        var matchedSlash = true;
        var found = false;
        var endIndex = -1;
        for (var i = end - 1; i >= rootInfo.rootEnd; i -= 1) {
          if (isSep(path.charCodeAt(i))) {
            if (!matchedSlash) { endIndex = i; found = true; break; }
          } else {
            matchedSlash = false;
          }
        }
        if (!found) {
          if (rootInfo.device && !rootInfo.absolute) return rootInfo.device;
          return rootInfo.absolute ? rootString() : ".";
        }
        if (endIndex < rootInfo.rootEnd) endIndex = rootInfo.rootEnd;
        if (!windows && endIndex === 1 && path.length > 2 && path.charAt(0) === "/" && path.charAt(1) === "/" && path.charAt(2) !== "/") endIndex = 2;
        var result = path.slice(0, endIndex);
        if (windows && rootInfo.device && !rootInfo.absolute && result === rootInfo.device) return rootInfo.device + ".";
        return result || (rootInfo.absolute ? sep : ".");
      }

      function extname(path) {
        var base = basename(path);
        if (!base) return "";
        var dot = base.lastIndexOf(".");
        if (dot <= 0 || base === ".." || base === ".") return "";
        return base.slice(dot);
      }

      function parse(path) {
        assertPath(path);
        var root = "";
        if (windows) {
          var rootInfo = winRoot(path);
          if (rootInfo.device || rootInfo.absolute) {
            root = rootInfo.device + (rootInfo.rootEnd > rootInfo.device.length ? path.charAt(rootInfo.rootEnd - 1) : "");
            if (!rootInfo.device && rootInfo.absolute) root = path.charAt(0);
          }
        } else if (path.charCodeAt(0) === 47) root = "/";
        var base = basename(path);
        if (windows && trimTrailing(path) <= winRoot(path).rootEnd) base = "";
        if (!windows && trimTrailing(path) <= 1 && path.charCodeAt(0) === 47) base = "";
        var ext = extname(base);
        var dir = dirname(path);
        var explicitDotDir = windows ? /^\.{1,2}[\\/].+/.test(path) : /^\.{1,2}\/.+/.test(path);
        if (dir === "." && !explicitDotDir) dir = "";
        if (!windows && root === "/") {
          var leadingSlashes = (path.match(/^\/+|$/) || [""])[0].length;
          var posixTail = path.slice(leadingSlashes).replace(/\/+$/, "");
          if (leadingSlashes > 1 && posixTail && posixTail.indexOf("/") < 0 && base === posixTail) {
            dir = "/".repeat(leadingSlashes - 1);
          }
        }
        return { root: root, dir: dir, base: base, ext: ext, name: base.slice(0, base.length - ext.length) };
      }

      function format(pathObject) {
        if (pathObject === null || typeof pathObject !== "object") {
          var objectReceived = describeReceived(pathObject);
          var objectError = new TypeError('The "pathObject" argument must be of type object.' + objectReceived);
          objectError.code = "ERR_INVALID_ARG_TYPE";
          throw objectError;
        }
        var dir = pathObject.dir || pathObject.root || "";
        var base;
        if (pathObject.base) {
          base = pathObject.base;
        } else {
          var ext = pathObject.ext || "";
          if (ext && !ext.startsWith(".")) ext = "." + ext;
          base = (pathObject.name || "") + ext;
        }
        if (!dir) return base;
        return dir === pathObject.root ? dir + base : dir + sep + base;
      }

      function relative(from, to) {
        assertPath(from, "from");
        assertPath(to, "to");
        if (from === to) return "";
        var fromResolved = resolveFor("path.relative", [from]);
        var toResolved = resolveFor("path.relative", [to]);
        if (fromResolved === toResolved) return "";
        var fromRoot: any = windows ? winRoot(fromResolved) : { device: "", rootEnd: fromResolved.charCodeAt(0) === 47 ? 1 : 0 };
        var toRoot: any = windows ? winRoot(toResolved) : { device: "", rootEnd: toResolved.charCodeAt(0) === 47 ? 1 : 0 };
        var fromStart = fromRoot.rootEnd;
        var toStart = toRoot.rootEnd;
        if (windows) {
          if (fromRoot.unc && toRoot.unc) {
            var fromServerEnd = fromRoot.device.indexOf("\\", 2);
            var toServerEnd = toRoot.device.indexOf("\\", 2);
            var fromServer = fromServerEnd < 0 ? fromRoot.device.slice(2) : fromRoot.device.slice(2, fromServerEnd);
            var toServer = toServerEnd < 0 ? toRoot.device.slice(2) : toRoot.device.slice(2, toServerEnd);
            if (fromServer.toLowerCase() !== toServer.toLowerCase()) return toResolved;
            fromStart = 2;
            toStart = 2;
          } else if (fromRoot.device.toLowerCase() !== toRoot.device.toLowerCase()) {
            return toResolved;
          }
        }
        var fromParts = fromResolved.slice(fromStart).split(windows ? /[\\/]+/ : /\/+/).filter(Boolean);
        var toParts = toResolved.slice(toStart).split(windows ? /[\\/]+/ : /\/+/).filter(Boolean);
        var common = 0;
        while (common < fromParts.length && common < toParts.length &&
               (windows ? fromParts[common].toLowerCase() === toParts[common].toLowerCase() : fromParts[common] === toParts[common])) {
          common += 1;
        }
        var output = [];
        for (var i = common; i < fromParts.length; i += 1) output.push("..");
        for (var j = common; j < toParts.length; j += 1) output.push(toParts[j]);
        return output.join(sep);
      }

      function toNamespacedPathAs(apiName, path) {
        if (!windows) return path;
        if (typeof path !== "string") return path;
        var resolved = resolveFor(apiName, [path]);
        var rootInfo = winRoot(resolved);
        if (!rootInfo.device) return resolved;
        if (rootInfo.device.indexOf("\\\\?\\") === 0 || rootInfo.device.indexOf("\\\\.\\") === 0) return resolved;
        if (rootInfo.device.indexOf("\\\\") === 0) return "\\\\?\\UNC\\" + resolved.slice(2);
        return "\\\\?\\" + resolved;
      }

      function toNamespacedPath(path) {
        return toNamespacedPathAs("path.toNamespacedPath", path);
      }

      function matchesGlob(path, pattern) {
        assertPath(path);
        assertPath(pattern);
        var candidate = windows ? path.replace(/\\/g, "/") : path;
        var glob = windows ? pattern.replace(/\\/g, "/") : pattern;
        var candidateTrailingSlash = candidate.length > 0 && candidate.endsWith("/");
        var patternTrailingSlash = glob.length > 0 && glob.endsWith("/");
        candidate = candidate.replace(/\/{2,}/g, "/");
        glob = glob.replace(/\/{2,}/g, "/");
        if (candidateTrailingSlash !== patternTrailingSlash && patternTrailingSlash) return false;
        var candidateAbsolute = candidate.startsWith("/");
        var patternAbsolute = glob.startsWith("/");
        if (candidateAbsolute !== patternAbsolute) return false;
        var candidateParts = candidate.split("/").filter(Boolean);
        var patternParts = glob.split("/").filter(Boolean);
        var flags = windows ? "i" : "";

        function splitAlternatives(value, separator) {
          var parts = [];
          var depth = 0;
          var start = 0;
          for (var si = 0; si < value.length; si += 1) {
            if (value[si] === "\\") { si += 1; continue; }
            if (value[si] === "(" || value[si] === "{" || value[si] === "[") depth += 1;
            else if (value[si] === ")" || value[si] === "}" || value[si] === "]") depth = Math.max(0, depth - 1);
            else if (value[si] === separator && depth === 0) { parts.push(value.slice(start, si)); start = si + 1; }
          }
          parts.push(value.slice(start));
          return parts;
        }

        function findClose(value, start, open, close) {
          var depth = 0;
          for (var ci = start; ci < value.length; ci += 1) {
            if (value[ci] === "\\") { ci += 1; continue; }
            if (value[ci] === open) depth += 1;
            else if (value[ci] === close && --depth === 0) return ci;
          }
          return -1;
        }

        function compileSegment(segment) {
          function compilePart(value) {
            var out = "";
            for (var gi = 0; gi < value.length; gi += 1) {
              var character = value[gi];
              if (character === "\\" && gi + 1 < value.length) {
                out += value[++gi].replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
                continue;
              }
              if ((character === "@" || character === "+" || character === "?" || character === "*" || character === "!") && value[gi + 1] === "(") {
                var extEnd = findClose(value, gi + 1, "(", ")");
                if (extEnd > gi + 1) {
                  var alternatives = splitAlternatives(value.slice(gi + 2, extEnd), "|").map(compilePart);
                  var group = "(?:" + alternatives.join("|") + ")";
                  var rest = character === "!" ? compilePart(value.slice(extEnd + 1)) : "";
                  out += character === "@" ? group : character === "+" ? group + "+" : character === "?" ? group + "?" : character === "*" ? group + "*" : "(?!(?:" + alternatives.join("|") + ")" + rest + "$)[^/]*";
                  gi = extEnd;
                  continue;
                }
              }
              if (character === "{") {
                var braceEnd = findClose(value, gi, "{", "}");
                if (braceEnd > gi + 1) {
                  var alternativesText = value.slice(gi + 1, braceEnd);
                  var braceAlternatives = splitAlternatives(alternativesText, ",");
                  if (braceAlternatives.length > 1) out += "(?:" + braceAlternatives.map(compilePart).join("|") + ")";
                  else {
                    var range = /^(-?\d+)\.\.(-?\d+)(?:\.\.(-?\d+))?$/.exec(alternativesText);
                    if (range) {
                      var from = Number(range[1]), to = Number(range[2]), step = Number(range[3] || (from <= to ? 1 : -1));
                      if (step === 0 || Math.sign(to - from) && Math.sign(step) !== Math.sign(to - from)) out += "";
                      else {
                        var choices = [];
                        for (var n = from, guard = 0; guard < 1000 && (step > 0 ? n <= to : n >= to); n += step, guard += 1) choices.push(String(n).replace(/[\\^$.*+?()[\]{}|]/g, "\\$&"));
                        out += "(?:" + choices.join("|") + ")";
                      }
                    } else out += "\\{" + compilePart(alternativesText) + "\\}";
                  }
                  gi = braceEnd;
                  continue;
                }
              }
              if (character === "*") { out += "[^/]*"; continue; }
              if (character === "?") { out += "[^/]"; continue; }
              if (character === "[") {
                var classEnd = findClose(value, gi, "[", "]");
                if (classEnd > gi + 1) {
                  var body = value.slice(gi + 1, classEnd);
                  if (body[0] === "!") body = "^" + body.slice(1);
                  out += "[" + body.replace(/\\/g, "\\\\") + "]";
                  gi = classEnd;
                  continue;
                }
              }
              out += character.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
            }
            return out;
          }
          return new RegExp("^" + compilePart(segment) + "$", flags);
        }

        function matchSegments(patternIndex, pathIndex) {
          if (patternIndex === patternParts.length) return pathIndex === candidateParts.length;
          var segment = patternParts[patternIndex];
          if (segment === "**") {
            if (matchSegments(patternIndex + 1, pathIndex)) return true;
            for (var rest = pathIndex; rest < candidateParts.length; rest += 1) {
              if (candidateParts[rest].startsWith(".") && !segment.startsWith(".")) break;
              if (matchSegments(patternIndex + 1, rest + 1)) return true;
            }
            return false;
          }
          var part = candidateParts[pathIndex];
          if (part === undefined || part.startsWith(".") && !segment.startsWith(".")) return false;
          return compileSegment(segment).test(part) && matchSegments(patternIndex + 1, pathIndex + 1);
        }

        if (!patternParts.length && !candidateParts.length) return true;
        return matchSegments(0, 0);
      }

      function assertPath(path, name?) {
        if (typeof path !== "string") {
          name = name || "path";
          var error = new TypeError('The "' + name + '" argument must be of type string.' + describeReceived(path));
          error.code = "ERR_INVALID_ARG_TYPE";
          throw error;
        }
      }

      function describeReceived(value) {
        if (value === null) return " Received null";
        if (value === undefined) return " Received undefined";
        if (typeof value === "string") {
          var inspected = "'" + value.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t") + "'";
          if (inspected.length > 28) inspected = inspected.slice(0, 25) + "...";
          return " Received type string (" + inspected + ")";
        }
        if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
          return " Received type " + typeof value + " (" + String(value) + (typeof value === "bigint" ? "n" : "") + ")";
        }
        if (typeof value === "function") return " Received function" + (value.name ? " " + value.name : "");
        if (typeof value === "object") {
          var constructorName;
          try { constructorName = value.constructor && value.constructor.name; } catch (_) {}
          if (constructorName) return " Received an instance of " + constructorName;
          return " Received an object";
        }
        return " Received type " + typeof value;
      }

      var api: any = {
        resolve: resolve,
        normalize: normalize,
        isAbsolute: isAbsolute,
        join: join,
        relative: relative,
        toNamespacedPath: toNamespacedPath,
        dirname: dirname,
        basename: basename,
        extname: extname,
        parse: parse,
        format: format,
        matchesGlob: matchesGlob,
        sep: sep,
        delimiter: delimiter,
        _makeLong: function (path) { return windows ? toNamespacedPathAs("path._makeLong", path) : path; },
      };
      return api;
    }

    var posix: any = makePath(false);
    var win32: any = makePath(true);
    posix.posix = posix;
    posix.win32 = win32;
    win32.posix = posix;
    win32.win32 = win32;
    var defaultPath: any = detectPlatform() === "win32" ? win32 : posix;
    defaultPath.setCwd = function (value) {
      if (typeof value !== "string" || value.length === 0) throw new TypeError("cwd must be a non-empty string");
      cwd = value;
      cwdOverride = true;
    };
    defaultPath.getCwd = currentCwd;
    return defaultPath;
  }

  runtime.createPathModule = createPathModule;
  runtime.path = createPathModule();
})(globalThis);
