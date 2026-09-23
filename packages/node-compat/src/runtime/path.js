(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function detectPlatform() {
    if (typeof process !== "undefined" && process.platform) return process.platform;
    var platform = "";
    if (typeof navigator !== "undefined") {
      platform = navigator.userAgentData && navigator.userAgentData.platform || navigator.platform || "";
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

    function makePath(windows) {
      var sep = windows ? "\\" : "/";
      var delimiter = windows ? ";" : ":";
      var isSep = windows
        ? function (code) { return code === 47 || code === 92; }
        : function (code) { return code === 47; };

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
            var shareStart = serverEnd + 1;
            var shareEnd = shareStart;
            while (shareEnd < length && !isSep(path.charCodeAt(shareEnd))) shareEnd += 1;
            if (shareEnd > shareStart) {
              var prefix = extended ? "\\\\?\\UNC\\" : "\\\\";
              device = prefix + path.slice(serverStart, serverEnd) + "\\" + path.slice(shareStart, shareEnd);
              rootEnd = shareEnd;
              if (shareEnd < length && isSep(path.charCodeAt(shareEnd))) rootEnd += 1;
              absolute = true;
              return { device: device, rootEnd: rootEnd, absolute: absolute, unc: true };
            }
          }
        }

        if (length >= 2 && path.charCodeAt(1) === 58) {
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
        for (var i = 0; i <= path.length; i += 1) {
          var code = i < path.length ? path.charCodeAt(i) : (windows ? 92 : 47);
          if (i < path.length && !isSep(code)) {
            if (code === 46) dots += 1;
            else dots = -1;
            continue;
          }
          if (lastSlash === i - 1 || dots === 1) {
            // Empty and dot segments are removed.
          } else if (dots === 2) {
            if (result.length > 0 && lastSegmentLength !== 2) {
              var lastSlashIndex = result.lastIndexOf(sep);
              if (lastSlashIndex === -1) {
                result = "";
                lastSegmentLength = 0;
              } else {
                result = result.slice(0, lastSlashIndex);
                lastSegmentLength = result.length - 1 - result.lastIndexOf(sep);
              }
            } else if (allowAboveRoot) {
              result = result.length > 0 ? result + sep + ".." : "..";
              lastSegmentLength = 2;
            }
          } else {
            var segment = path.slice(lastSlash + 1, i);
            result = result.length > 0 ? result + sep + segment : segment;
            lastSegmentLength = segment.length;
          }
          lastSlash = i;
          dots = 0;
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

        var rootInfo = winRoot(path);
        var absoluteWin = rootInfo.absolute;
        var trailingWin = isSep(path.charCodeAt(path.length - 1));
        var tail = normalizeString(path.slice(rootInfo.rootEnd), !absoluteWin);
        var prefix = rootInfo.device;
        if (prefix && rootInfo.rootEnd > prefix.length) prefix += sep;
        if (absoluteWin && !prefix) prefix = sep;
        if (tail) {
          if (trailingWin) tail += sep;
          return prefix ? prefix + tail : tail;
        }
        if (rootInfo.unc) return rootInfo.device + sep;
        if (absoluteWin) return prefix || sep;
        if (rootInfo.device) return rootInfo.device + ".";
        return prefix || ".";
      }

      function resolve() {
        var resolvedDevice = "";
        var resolvedTail = "";
        var resolvedAbsolute = false;
        var args = Array.prototype.slice.call(arguments);
        for (var i = args.length - 1; i >= -1 && !resolvedAbsolute; i -= 1) {
          var path = i >= 0 ? args[i] : cwd;
          assertPath(path);
          if (path.length === 0) continue;
          if (!windows) {
            if (path.charCodeAt(0) === 47) {
              resolvedAbsolute = true;
            }
            resolvedTail = path + "/" + resolvedTail;
            continue;
          }
          var rootInfo = winRoot(path);
          var device = rootInfo.device;
          if (device) {
            if (resolvedDevice && device.toLowerCase() !== resolvedDevice.toLowerCase()) continue;
            if (!resolvedDevice) resolvedDevice = device;
          } else if (resolvedDevice && rootInfo.absolute) {
            // A root-relative path inherits the already selected drive.
          } else if (!resolvedDevice && rootInfo.absolute) {
            resolvedDevice = "";
          }
          if (rootInfo.absolute) {
            resolvedAbsolute = true;
          }
          var tail = path.slice(rootInfo.rootEnd);
          if (tail) resolvedTail = tail + "\\" + resolvedTail;
        }

        if (windows) {
          if (resolvedDevice && !resolvedAbsolute) {
            var base = cwd;
            var baseInfo = winRoot(base);
            if (baseInfo.device.toLowerCase() === resolvedDevice.toLowerCase()) {
              var baseTail = base.slice(baseInfo.rootEnd);
              resolvedTail = baseTail + "\\" + resolvedTail;
            } else {
              resolvedTail = "\\" + resolvedTail;
              resolvedAbsolute = true;
            }
          } else if (!resolvedDevice && !resolvedAbsolute) {
            var fallbackInfo = winRoot(cwd);
            resolvedDevice = fallbackInfo.device;
            resolvedTail = cwd.slice(fallbackInfo.rootEnd) + "\\" + resolvedTail;
            resolvedAbsolute = fallbackInfo.absolute;
          }
          var normalizedTail = normalizeString(resolvedTail, !resolvedAbsolute);
          if (resolvedAbsolute && !resolvedDevice) normalizedTail = "\\" + normalizedTail;
          if (resolvedDevice && resolvedAbsolute && normalizedTail) return resolvedDevice + "\\" + normalizedTail;
          if (resolvedDevice && resolvedAbsolute) return resolvedDevice + "\\";
          if (resolvedDevice) return resolvedDevice + normalizedTail;
          return normalizedTail || (resolvedAbsolute ? "\\" : ".");
        }

        var posixResolved = normalizeString(resolvedTail, !resolvedAbsolute);
        return (resolvedAbsolute ? "/" : "") + (posixResolved || (resolvedAbsolute ? "" : "."));
      }

      function isAbsolute(path) {
        assertPath(path);
        if (!windows) return path.length > 0 && path.charCodeAt(0) === 47;
        return winRoot(path).absolute;
      }

      function join() {
        var parts = Array.prototype.slice.call(arguments);
        var joined = "";
        for (var i = 0; i < parts.length; i += 1) {
          var part = parts[i];
          assertPath(part);
          if (part.length > 0) joined = joined ? joined + sep + part : part;
        }
        if (!joined) return ".";
        if (windows && /^\\\\[^\\]+[\\/][^\\]+/.test(joined)) {
          // Keep UNC roots intact while collapsing any extra leading slashes.
          joined = joined.replace(/^[\\/]{2,}/, "\\\\");
        }
        return normalize(joined);
      }

      function trimTrailing(path) {
        var end = path.length;
        while (end > 0 && isSep(path.charCodeAt(end - 1))) end -= 1;
        return end;
      }

      function basename(path, suffix) {
        assertPath(path);
        if (suffix !== undefined && typeof suffix !== "string") throw new TypeError("suffix must be a string");
        var end = trimTrailing(path);
        var start = end;
        while (start > 0 && !isSep(path.charCodeAt(start - 1))) start -= 1;
        if (windows) {
          var basenameRoot = winRoot(path);
          if (!basenameRoot.unc) start = Math.max(start, basenameRoot.rootEnd);
        }
        var base = path.slice(start, end);
        if (suffix && base.endsWith(suffix)) base = base.slice(0, base.length - suffix.length);
        return base;
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
        if (pathObject === null || typeof pathObject !== "object") throw new TypeError("pathObject must be an object");
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
        return dir + (dir.endsWith(sep) ? "" : sep) + base;
      }

      function relative(from, to) {
        assertPath(from);
        assertPath(to);
        if (from === to) return "";
        var fromResolved = resolve(from);
        var toResolved = resolve(to);
        var compareFrom = windows ? fromResolved.toLowerCase() : fromResolved;
        var compareTo = windows ? toResolved.toLowerCase() : toResolved;
        if (compareFrom === compareTo) return "";
        var fromRoot = windows ? winRoot(fromResolved) : { device: "", rootEnd: fromResolved.charCodeAt(0) === 47 ? 1 : 0 };
        var toRoot = windows ? winRoot(toResolved) : { device: "", rootEnd: toResolved.charCodeAt(0) === 47 ? 1 : 0 };
        if (windows && fromRoot.device.toLowerCase() !== toRoot.device.toLowerCase()) return toResolved;
        var fromParts = compareFrom.slice(fromRoot.rootEnd).split(windows ? /[\\/]+/ : /\/+/).filter(Boolean);
        var toParts = compareTo.slice(toRoot.rootEnd).split(windows ? /[\\/]+/ : /\/+/).filter(Boolean);
        var common = 0;
        while (common < fromParts.length && common < toParts.length && fromParts[common] === toParts[common]) common += 1;
        var output = [];
        for (var i = common; i < fromParts.length; i += 1) output.push("..");
        for (var j = common; j < toParts.length; j += 1) output.push(toParts[j]);
        return output.join(sep);
      }

      function toNamespacedPath(path) {
        if (!windows) return path;
        if (typeof path !== "string") return path;
        var resolved = resolve(path);
        var rootInfo = winRoot(resolved);
        if (!rootInfo.device) return resolved;
        if (rootInfo.device.indexOf("\\\\?\\") === 0 || rootInfo.device.indexOf("\\\\.\\") === 0) return resolved;
        if (rootInfo.device.indexOf("\\\\") === 0) return "\\\\?\\UNC\\" + resolved.slice(2);
        return "\\\\?\\" + resolved;
      }

      function matchesGlob(path, pattern) {
        assertPath(path);
        assertPath(pattern);
        var candidate = windows ? path.replace(/\\/g, "/") : path;
        var glob = windows ? pattern.replace(/\\/g, "/") : pattern;
        var expression = "^";
        for (var i = 0; i < glob.length; i += 1) {
          var character = glob[i];
          if (character === "*" && glob[i + 1] === "*") {
            i += 1;
            if (glob[i + 1] === "/") { i += 1; expression += "(?:.*/)?"; }
            else expression += ".*";
          } else if (character === "*") expression += "[^/]*";
          else if (character === "?") expression += "[^/]";
          else if (character === "[") {
            var close = glob.indexOf("]", i + 1);
            if (close > i + 1) { expression += glob.slice(i, close + 1); i = close; }
            else expression += "\\[";
          } else expression += character.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
        }
        expression += "$";
        return new RegExp(expression, windows ? "i" : "").test(candidate);
      }

      function assertPath(path) {
        if (typeof path !== "string") throw new TypeError("path must be a string");
      }

      var api = {
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
        _makeLong: function (path) { return path; },
      };
      return api;
    }

    var posix = makePath(false);
    var win32 = makePath(true);
    var defaultPath = detectPlatform() === "win32" ? win32 : posix;
    var module = {};
    Object.keys(defaultPath).forEach(function (key) { module[key] = defaultPath[key]; });
    module.posix = posix;
    module.win32 = win32;
    module.setCwd = function (value) {
      if (typeof value !== "string" || value.length === 0) throw new TypeError("cwd must be a non-empty string");
      cwd = value;
    };
    module.getCwd = function () { return cwd; };
    return module;
  }

  runtime.createPathModule = createPathModule;
  runtime.path = createPathModule();
})(globalThis);
