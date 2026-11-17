// Puddi runtime glue. Works in browsers and in Node.js (>= 22).
// Node usage: node puddi.mjs run app.wasm [args...] | node puddi.mjs test app.wasm

const enc = new TextEncoder();
const dec = new TextDecoder();

export class PuddiPanic extends Error {
  constructor(msg) {
    super(msg);
    this.name = "PuddiPanic";
  }
}

function fmtFloat(x, is32) {
  if (Number.isNaN(x)) return "NaN";
  if (x === Infinity) return "inf";
  if (x === -Infinity) return "-inf";
  let s;
  if (is32) {
    s = String(x);
    for (let p = 1; p <= 9; p++) {
      const t = x.toPrecision(p);
      if (Math.fround(Number(t)) === x) {
        s = String(Number(t));
        break;
      }
    }
  } else {
    s = String(x);
  }
  s = s.replace("e+", "e");
  if (!/[.eE]/.test(s)) s += ".0";
  return s;
}

function parseFloatStrict(s) {
  const t = s.trim();
  if (/^[+-]?(inf|infinity)$/i.test(t)) return t.startsWith("-") ? -Infinity : Infinity;
  if (!/^[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?$/.test(t)) return NaN;
  return Number(t);
}

function mathOp(op, x, y, env) {
  switch (op) {
    case 0: return Math.pow(x, y);
    case 1: return Math.sin(x);
    case 2: return Math.cos(x);
    case 3: return Math.tan(x);
    case 4: return Math.asin(x);
    case 5: return Math.acos(x);
    case 6: return Math.atan(x);
    case 7: return Math.atan2(x, y);
    case 8: return Math.exp(x);
    case 9: return Math.log(x);
    case 10: return Math.log2(x);
    case 11: return Math.log10(x);
    case 12: return Math.cbrt(x);
    case 13: return Math.hypot(x, y);
    case 14: return Math.random();
    case 15: return Date.now();
    case 16: return globalThis.performance ? performance.now() : Date.now();
    case 17: return env.tzOffset ? env.tzOffset() : -new Date().getTimezoneOffset();
    default: return NaN;
  }
}

/// Build the import object. `env` provides write(fd, bytes) and host(op, arg).
export function makeImports(env, extra = {}) {
  let inst = null;
  let pending = new Uint8Array(0);
  let ok = 1;
  const mem = () => new Uint8Array(inst.exports.memory.buffer);
  const str = (p, n) => dec.decode(mem().subarray(p, p + n));
  const imports = {
    puddi: {
      write(fd, p, n) {
        env.write(fd, mem().slice(p, p + n));
      },
      panic(p, n) {
        throw new PuddiPanic(str(p, n));
      },
      fmt_f64(x, p, is32) {
        const b = enc.encode(fmtFloat(x, is32));
        mem().set(b, p);
        return b.length;
      },
      parse_f64(p, n) {
        return parseFloatStrict(str(p, n));
      },
      math(op, x, y) {
        return mathOp(op, x, y, env);
      },
      host(op, p, n) {
        const arg = str(p, n);
        let r;
        try {
          r = env.host(op, arg);
          ok = 1;
        } catch (e) {
          if (e && e.puddiExit !== undefined) throw e;
          ok = 0;
          r = e && e.message !== undefined ? e.message : String(e);
        }
        pending = enc.encode(r == null ? "" : String(r));
        return pending.length;
      },
      host_take(p) {
        mem().set(pending, p);
      },
      host_ok() {
        return ok;
      },
    },
  };
  for (const [mod, fns] of Object.entries(extra)) {
    imports[mod] = imports[mod] || {};
    for (const [name, f] of Object.entries(fns)) {
      // raw @extern imports receive strings as (ptr, len); wrap so hosts can use JS strings
      imports[mod][name] = f;
    }
  }
  return {
    imports,
    bind(instance) {
      inst = instance;
    },
    readStr: str,
  };
}

export function panicMessage(e) {
  if (e instanceof PuddiPanic) return e.message;
  if (e instanceof RangeError && /call stack/i.test(e.message)) return "stack overflow";
  if (typeof WebAssembly !== "undefined" && e instanceof WebAssembly.RuntimeError) {
    const m = e.message;
    if (/divide by zero|remainder by zero/i.test(m)) return "division by zero";
    if (/out of bounds/i.test(m)) return "index out of bounds";
    if (/unreachable/i.test(m)) return "entered unreachable code";
    if (/null/i.test(m)) return "null reference";
    if (/integer overflow/i.test(m)) return "integer overflow";
    return m;
  }
  return null;
}

function lineWriter(sink) {
  const bufs = { 1: "", 2: "" };
  return {
    write(fd, bytes) {
      bufs[fd] += dec.decode(bytes);
      let i;
      while ((i = bufs[fd].indexOf("\n")) >= 0) {
        sink(fd, bufs[fd].slice(0, i));
        bufs[fd] = bufs[fd].slice(i + 1);
      }
    },
    flush() {
      for (const fd of [1, 2]) {
        if (bufs[fd]) sink(fd, bufs[fd]);
        bufs[fd] = "";
      }
    },
  };
}

/// Browser / generic loader. Returns { instance, run(), exports }.
export async function load(source, opts = {}) {
  const lw = lineWriter(opts.output || ((fd, line) => (fd === 2 ? console.error(line) : console.log(line))));
  const env = {
    write: (fd, b) => lw.write(fd, b),
    host(op, arg) {
      if (op === 30) {
        const [lvl, msg] = arg.split("\0");
        (lvl === "error" ? console.error : lvl === "warn" ? console.warn : console.log)(`[${lvl}] ${msg}`);
        return "";
      }
      if (opts.host) return opts.host(op, arg);
      throw new Error("host operation not available in this environment");
    },
  };
  const host = makeImports(env, opts.imports || {});
  let mod;
  if (source instanceof WebAssembly.Module) mod = source;
  else if (typeof source === "string" || source instanceof URL) mod = await WebAssembly.compileStreaming(fetch(source));
  else mod = await WebAssembly.compile(source);
  const instance = await WebAssembly.instantiate(mod, host.imports);
  host.bind(instance);
  return {
    instance,
    exports: instance.exports,
    run() {
      try {
        const code = instance.exports.main ? instance.exports.main() : 0;
        lw.flush();
        return code;
      } catch (e) {
        lw.flush();
        const m = panicMessage(e);
        if (m !== null) {
          console.error(`panic: ${m}`);
          return 101;
        }
        throw e;
      }
    },
  };
}

// ---------------------------------------------------------------- Node CLI

async function nodeMain() {
  const fs = await import("node:fs");
  const process = globalThis.process;
  const [cmd, wasmPath, ...rest] = process.argv.slice(2);
  const bytes = fs.readFileSync(wasmPath);
  const mod = await WebAssembly.compile(bytes);
  let stdinBuf = Buffer.alloc(0);
  let stdinEof = false;
  const readStdinChunk = () => {
    const b = Buffer.alloc(65536);
    let n = 0;
    try {
      n = fs.readSync(0, b, 0, b.length, null);
    } catch (e) {
      if (e.code === "EAGAIN") return true;
      if (e.code === "EOF") n = 0;
      else throw e;
    }
    if (n === 0) {
      stdinEof = true;
      return false;
    }
    stdinBuf = Buffer.concat([stdinBuf, b.subarray(0, n)]);
    return true;
  };
  const ioErr = (e) => {
    const code = e.code === "ENOENT" ? "NotFound" : e.code === "EACCES" || e.code === "EPERM" ? "PermissionDenied" : e.code === "EEXIST" ? "AlreadyExists" : "Other";
    return new Error(`${code}\0${e.message}`);
  };
  const env = {
    write(fd, b) {
      fs.writeSync(fd, b);
    },
    host(op, arg) {
      try {
        switch (op) {
          case 1: return fs.readFileSync(arg, "utf8");
          case 2: { const i = arg.indexOf("\0"); fs.writeFileSync(arg.slice(0, i), arg.slice(i + 1)); return ""; }
          case 3: { const i = arg.indexOf("\0"); fs.appendFileSync(arg.slice(0, i), arg.slice(i + 1)); return ""; }
          case 4: return fs.existsSync(arg) ? "1" : "0";
          case 5: fs.rmSync(arg, { recursive: true }); return "";
          case 6: return fs.readdirSync(arg).sort().join("\n");
          case 7: fs.mkdirSync(arg, { recursive: true }); return "";
          case 8: return fs.statSync(arg).isDirectory() ? "1" : "0";
        }
      } catch (e) {
        throw ioErr(e);
      }
      switch (op) {
        case 10: return [wasmPath.replace(/\.wasm$/, ""), ...rest].join("\0");
        case 11: {
          const v = process.env[arg];
          if (v === undefined) throw new Error("not set");
          return v;
        }
        case 12: {
          const e = new Error("exit");
          e.puddiExit = Number(arg);
          throw e;
        }
        case 20: {
          for (;;) {
            const i = stdinBuf.indexOf(10);
            if (i >= 0) {
              const line = stdinBuf.subarray(0, i).toString("utf8").replace(/\r$/, "");
              stdinBuf = stdinBuf.subarray(i + 1);
              return line;
            }
            if (stdinEof || !readStdinChunk()) {
              if (stdinBuf.length === 0) throw new Error("end of input");
              const line = stdinBuf.toString("utf8");
              stdinBuf = Buffer.alloc(0);
              return line;
            }
          }
        }
        case 21: {
          while (!stdinEof && readStdinChunk()) {}
          const s = stdinBuf.toString("utf8");
          stdinBuf = Buffer.alloc(0);
          return s;
        }
        case 22: fs.writeSync(1, arg); return "";
        case 23: fs.writeSync(2, arg); return "";
        case 30: {
          const [lvl, msg] = arg.split("\0");
          fs.writeSync(2, `[${lvl}] ${msg}\n`);
          return "";
        }
      }
      throw new Error(`unknown host operation ${op}`);
    },
  };
  const instantiate = async () => {
    const host = makeImports(env);
    const instance = await WebAssembly.instantiate(mod, host.imports);
    host.bind(instance);
    return instance;
  };
  if (cmd === "run") {
    const instance = await instantiate();
    try {
      const code = instance.exports.main ? instance.exports.main() : 0;
      process.exitCode = code;
    } catch (e) {
      if (e && e.puddiExit !== undefined) {
        process.exitCode = e.puddiExit;
        return;
      }
      const m = panicMessage(e);
      if (m === null) throw e;
      fs.writeSync(2, `panic: ${m}\n`);
      process.exitCode = 101;
    }
  } else if (cmd === "test") {
    const tests = JSON.parse(fs.readFileSync(wasmPath.replace(/\.wasm$/, ".tests.json"), "utf8"));
    const filter = rest[0];
    let passed = 0;
    let failed = 0;
    const selected = tests.filter(([name]) => !filter || name.includes(filter));
    fs.writeSync(1, `running ${selected.length} test${selected.length === 1 ? "" : "s"}\n`);
    for (const [name, exp] of selected) {
      const instance = await instantiate();
      try {
        instance.exports[exp]();
        passed++;
        fs.writeSync(1, `test "${name}" ... ok\n`);
      } catch (e) {
        const m = panicMessage(e);
        if (m === null && !(e && e.puddiExit !== undefined)) throw e;
        failed++;
        fs.writeSync(1, `test "${name}" ... FAILED\n    panic: ${m ?? "exit"}\n`);
      }
    }
    fs.writeSync(1, `\n${failed === 0 ? "ok" : "FAILED"}: ${passed} passed, ${failed} failed\n`);
    process.exitCode = failed === 0 ? 0 : 1;
  } else {
    fs.writeSync(2, "usage: node puddi.mjs run|test app.wasm [args...]\n");
    process.exitCode = 2;
  }
}

if (typeof process !== "undefined" && process.versions && process.versions.node && process.argv[1] && import.meta.url.endsWith(process.argv[1].split(/[\\/]/).pop())) {
  nodeMain().catch((e) => {
    console.error(e);
    process.exit(70);
  });
}
