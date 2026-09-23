(function (root) {
  "use strict";

  var runtime = root[Symbol.for("niva.node-compat.runtime")];

  function nodePlatform(value) {
    var os = String(value || "").toLowerCase();
    if (os.indexOf("windows") >= 0 || os === "win32") return "win32";
    if (os.indexOf("mac") >= 0 || os.indexOf("darwin") >= 0 || os.indexOf("os x") >= 0) return "darwin";
    if (os.indexOf("linux") >= 0) return "linux";
    if (os.indexOf("freebsd") >= 0) return "freebsd";
    if (os.indexOf("openbsd") >= 0) return "openbsd";
    if (os.indexOf("netbsd") >= 0) return "netbsd";
    if (os.indexOf("dragonfly") >= 0) return "dragonfly";
    if (os.indexOf("sunos") >= 0 || os.indexOf("solaris") >= 0) return "sunos";
    return os || "unknown";
  }

  function nodeArch(value) {
    var arch = String(value || "");
    if (arch === "x86_64" || arch === "x86-64" || arch === "amd64") return "x64";
    if (arch === "aarch64" || arch === "arm64") return "arm64";
    if (arch === "i386" || arch === "i686" || arch === "x86") return "ia32";
    if (arch === "arm") return "arm";
    if (arch === "powerpc64") return "ppc64";
    if (arch === "powerpc") return "ppc";
    if (arch === "riscv64") return "riscv64";
    return arch;
  }

  function createOsModule(niva) {
    function osApi() { return runtime.api(niva, "os"); }
    function info() { return osApi().info(); }
    function dirs() { return osApi().dirs(); }

    var separator = runtime.path ? runtime.path.sep : "/";
    var module = {
      // These constants are available synchronously from the WebView platform
      // hint. Dynamic OS details come from Niva and therefore stay async.
      EOL: separator === "\\" ? "\r\n" : "\n",
      devNull: separator === "\\" ? "\\\\.\\NUL" : "/dev/null",
      sep: separator,
      delimiter: separator === "\\" ? ";" : ":",
      info: info,
      dirs: dirs,
      platform: function () { return info().then(function (value) { return nodePlatform(value.os); }); },
      arch: function () { return info().then(function (value) { return nodeArch(value.arch); }); },
      homedir: function () {
        return dirs().then(function (value) {
          if (!value || typeof value.home !== "string") throw runtime.bridgeError("The Niva bridge did not provide a home directory", "ENOSYS");
          return value.home;
        });
      },
      tmpdir: function () {
        return dirs().then(function (value) {
          if (!value || typeof value.temp !== "string") throw runtime.bridgeError("The Niva bridge did not provide a temporary directory", "ENOSYS");
          return value.temp;
        });
      },
    };
    return module;
  }

  runtime.createOsModule = createOsModule;
  runtime.os = createOsModule();
})(globalThis);
