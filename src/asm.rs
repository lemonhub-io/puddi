//! Assembles the WebAssembly text that `codegen` writes into a binary module.
//!
//! This is much faster than the general `wat` crate: the text is cut into module fields with
//! a byte scan, names are resolved through hash maps, and function bodies are encoded on
//! several threads. It understands only the part of the text format that `codegen` uses
//! (plus the obvious neighbours); anything else is an error, and the caller falls back to
//! `wat`. The output matches `wat`'s byte for byte apart from the name section, which holds
//! only function names (`PUD_ASM_CHECK=1` compares the two).

use std::collections::HashMap;
use std::sync::OnceLock;

type R<T> = Result<T, String>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tok<'a> {
    L,
    R,
    Atom(&'a str),
    Str(&'a str),
    Eof,
}

#[derive(Clone)]
struct Lx<'a> {
    s: &'a str,
    p: usize,
    end: usize,
}

impl<'a> Lx<'a> {
    fn skip(&mut self) {
        let b = self.s.as_bytes();
        while self.p < self.end {
            match b[self.p] {
                b' ' | b'\n' | b'\r' | b'\t' => self.p += 1,
                b';' if b.get(self.p + 1) == Some(&b';') => {
                    while self.p < self.end && b[self.p] != b'\n' {
                        self.p += 1;
                    }
                }
                b'(' if b.get(self.p + 1) == Some(&b';') => {
                    let mut depth = 0;
                    while self.p < self.end {
                        if b[self.p] == b'(' && b.get(self.p + 1) == Some(&b';') {
                            depth += 1;
                            self.p += 2;
                        } else if b[self.p] == b';' && b.get(self.p + 1) == Some(&b')') {
                            depth -= 1;
                            self.p += 2;
                            if depth == 0 {
                                break;
                            }
                        } else {
                            self.p += 1;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn next(&mut self) -> Tok<'a> {
        self.skip();
        let b = self.s.as_bytes();
        if self.p >= self.end {
            return Tok::Eof;
        }
        match b[self.p] {
            b'(' => {
                self.p += 1;
                Tok::L
            }
            b')' => {
                self.p += 1;
                Tok::R
            }
            b'"' => {
                let st = self.p + 1;
                let mut i = st;
                while i < self.end && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let i = i.min(self.end);
                self.p = (i + 1).min(self.end);
                Tok::Str(&self.s[st..i])
            }
            _ => {
                let st = self.p;
                while self.p < self.end && !matches!(b[self.p], b' ' | b'\n' | b'\r' | b'\t' | b'(' | b')' | b'"') {
                    self.p += 1;
                }
                Tok::Atom(&self.s[st..self.p])
            }
        }
    }

    fn peek(&self) -> Tok<'a> {
        self.clone().next()
    }

    fn peek2(&self) -> (Tok<'a>, Tok<'a>) {
        let mut c = self.clone();
        let a = c.next();
        (a, c.next())
    }

    /// The keyword of a parenthesised form that comes next, if any.
    fn form(&self) -> Option<&'a str> {
        match self.peek2() {
            (Tok::L, Tok::Atom(a)) => Some(a),
            _ => None,
        }
    }

    fn err<T>(&self, what: &str) -> R<T> {
        Err(format!("{what} at byte {}", self.p))
    }

    fn atom(&mut self) -> R<&'a str> {
        match self.next() {
            Tok::Atom(a) => Ok(a),
            t => self.err(&format!("expected an atom, found {t:?}")),
        }
    }

    fn string(&mut self) -> R<&'a str> {
        match self.next() {
            Tok::Str(s) => Ok(s),
            t => self.err(&format!("expected a string, found {t:?}")),
        }
    }

    fn close(&mut self) -> R<()> {
        match self.next() {
            Tok::R => Ok(()),
            t => self.err(&format!("expected `)`, found {t:?}")),
        }
    }

    /// Opens the form `(kw` that `form()` just reported.
    fn open(&mut self) {
        self.next();
        self.next();
    }

    fn id(&mut self) -> Option<&'a str> {
        if let Tok::Atom(a) = self.peek() {
            if a.starts_with('$') {
                self.next();
                return Some(a);
            }
        }
        None
    }

    /// Skips to just after the `)` that closes the current form.
    fn skip_form(&mut self) -> R<()> {
        let mut depth = 1;
        loop {
            match self.next() {
                Tok::L => depth += 1,
                Tok::R => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                Tok::Eof => return self.err("unbalanced parentheses"),
                _ => {}
            }
        }
    }
}

/// Position of the `)` closing the form whose contents start at `from`.
fn close_paren(s: &str, from: usize) -> R<usize> {
    let b = s.as_bytes();
    let mut i = from;
    let mut depth = 1;
    while i < b.len() {
        match b[i] {
            b'(' if b.get(i + 1) == Some(&b';') => {
                let mut lx = Lx { s, p: i, end: s.len() };
                lx.skip();
                i = lx.p;
                continue;
            }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    if b[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            b';' if b.get(i + 1) == Some(&b';') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    Err("unbalanced parentheses".into())
}

// ---------------------------------------------------------------- encoding helpers

fn uleb(o: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            o.push(b);
            return;
        }
        o.push(b | 0x80);
    }
}

fn sleb(o: &mut Vec<u8>, mut v: i64) {
    loop {
        let b = (v & 0x7f) as u8;
        v >>= 7;
        if (v == 0 && b & 0x40 == 0) || (v == -1 && b & 0x40 != 0) {
            o.push(b);
            return;
        }
        o.push(b | 0x80);
    }
}

fn name(o: &mut Vec<u8>, s: &[u8]) {
    uleb(o, s.len() as u64);
    o.extend_from_slice(s);
}

fn section(o: &mut Vec<u8>, id: u8, count: usize, body: &[u8]) {
    let mut c = Vec::with_capacity(body.len() + 5);
    uleb(&mut c, count as u64);
    c.extend_from_slice(body);
    o.push(id);
    uleb(o, c.len() as u64);
    o.extend_from_slice(&c);
}

fn unescape(s: &str) -> R<Vec<u8>> {
    let b = s.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' {
            o.push(b[i]);
            i += 1;
            continue;
        }
        let c = *b.get(i + 1).ok_or("bad escape")?;
        i += 2;
        match c {
            b't' => o.push(b'\t'),
            b'n' => o.push(b'\n'),
            b'r' => o.push(b'\r'),
            b'"' | b'\'' | b'\\' => o.push(c),
            b'u' => {
                let close = s[i..].find('}').ok_or("bad escape")?;
                let v = u32::from_str_radix(&s[i + 1..i + close], 16).map_err(|_| "bad escape")?;
                let ch = char::from_u32(v).ok_or("bad escape")?;
                let mut buf = [0; 4];
                o.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                i += close + 1;
            }
            _ => {
                let h = std::str::from_utf8(&b[i - 1..(i + 1).min(b.len())]).map_err(|_| "bad escape")?;
                o.push(u8::from_str_radix(h, 16).map_err(|_| "bad escape")?);
                i += 1;
            }
        }
    }
    Ok(o)
}

fn int(a: &str) -> R<i128> {
    let (neg, r) = match a.as_bytes().first() {
        Some(b'-') => (true, &a[1..]),
        Some(b'+') => (false, &a[1..]),
        _ => (false, a),
    };
    let owned;
    let r = if r.contains('_') {
        owned = r.replace('_', "");
        owned.as_str()
    } else {
        r
    };
    let v = match r.strip_prefix("0x") {
        Some(h) => u128::from_str_radix(h, 16),
        None => r.parse::<u128>(),
    }
    .map_err(|_| format!("bad integer `{a}`"))? as i128;
    Ok(if neg { -v } else { v })
}

fn float_text(a: &str) -> R<std::borrow::Cow<'_, str>> {
    if a.contains("0x") || a.contains(':') {
        return Err(format!("unsupported float `{a}`"));
    }
    Ok(if a.contains('_') { a.replace('_', "").into() } else { a.into() })
}

fn abstract_heap(a: &str) -> Option<u8> {
    Some(match a {
        "func" => 0x70,
        "extern" => 0x6f,
        "any" => 0x6e,
        "eq" => 0x6d,
        "i31" => 0x6c,
        "struct" => 0x6b,
        "array" => 0x6a,
        "exn" => 0x69,
        "noexn" => 0x74,
        "nofunc" => 0x73,
        "noextern" => 0x72,
        "none" => 0x71,
        _ => return None,
    })
}

// ---------------------------------------------------------------- names

#[derive(Default)]
struct Names<'a> {
    types: HashMap<&'a str, u32>,
    funcs: HashMap<&'a str, u32>,
    globals: HashMap<&'a str, u32>,
    tags: HashMap<&'a str, u32>,
    tables: HashMap<&'a str, u32>,
    mems: HashMap<&'a str, u32>,
    datas: HashMap<&'a str, u32>,
    elems: HashMap<&'a str, u32>,
}

fn lookup(map: &HashMap<&str, u32>, a: &str, what: &str) -> R<u32> {
    if a.starts_with('$') {
        map.get(a).copied().ok_or_else(|| format!("unknown {what} `{a}`"))
    } else {
        a.parse().map_err(|_| format!("bad {what} index `{a}`"))
    }
}

impl<'a> Names<'a> {
    fn ty(&self, a: &str) -> R<u32> {
        lookup(&self.types, a, "type")
    }

    fn heap(&self, a: &str, o: &mut Vec<u8>) -> R<()> {
        match abstract_heap(a) {
            Some(b) => o.push(b),
            None => sleb(o, self.ty(a)? as i64),
        }
        Ok(())
    }

    /// The rest of `(ref null? ht)` after `(ref`.
    fn reftype(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<bool> {
        let mut a = lx.atom()?;
        let null = a == "null";
        if null {
            a = lx.atom()?;
        }
        match abstract_heap(a) {
            Some(b) if null => o.push(b),
            Some(b) => {
                o.push(0x64);
                o.push(b);
            }
            None => {
                o.push(if null { 0x63 } else { 0x64 });
                sleb(o, self.ty(a)? as i64);
            }
        }
        lx.close()?;
        Ok(null)
    }

    fn valtype(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        match lx.next() {
            Tok::Atom(a) => o.push(match a {
                "i32" => 0x7f,
                "i64" => 0x7e,
                "f32" => 0x7d,
                "f64" => 0x7c,
                "v128" => 0x7b,
                "funcref" => 0x70,
                "externref" => 0x6f,
                "anyref" => 0x6e,
                "eqref" => 0x6d,
                "i31ref" => 0x6c,
                "structref" => 0x6b,
                "arrayref" => 0x6a,
                "exnref" => 0x69,
                "nullexnref" => 0x74,
                "nullfuncref" => 0x73,
                "nullexternref" => 0x72,
                "nullref" => 0x71,
                _ => return lx.err(&format!("unknown value type `{a}`")),
            }),
            Tok::L => {
                if lx.atom()? != "ref" {
                    return lx.err("expected `ref`");
                }
                self.reftype(lx, o)?;
            }
            t => return lx.err(&format!("expected a value type, found {t:?}")),
        }
        Ok(())
    }

    /// Value types up to the closing `)`, counted.
    fn valtypes(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<u32> {
        let mut n = 0;
        while lx.peek() != Tok::R {
            self.valtype(lx, o)?;
            n += 1;
        }
        lx.close()?;
        Ok(n)
    }

    fn storage(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        match lx.peek() {
            Tok::Atom("i8") => {
                lx.next();
                o.push(0x78);
            }
            Tok::Atom("i16") => {
                lx.next();
                o.push(0x77);
            }
            _ => self.valtype(lx, o)?,
        }
        Ok(())
    }

    fn fieldtype(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        if lx.form() == Some("mut") {
            lx.open();
            self.storage(lx, o)?;
            lx.close()?;
            o.push(1);
        } else {
            self.storage(lx, o)?;
            o.push(0);
        }
        Ok(())
    }

    /// `(param ...)* (result ...)*`, as a function type `0x60 params results`, and the
    /// parameter names.
    fn signature(&self, lx: &mut Lx<'a>, mut names: Option<&mut Vec<Option<&'a str>>>) -> R<Vec<u8>> {
        let (mut ps, mut rs) = (Vec::new(), Vec::new());
        let (mut np, mut nr) = (0u32, 0u32);
        loop {
            match lx.form() {
                Some("param") => {
                    lx.open();
                    if let Some(id) = lx.id() {
                        self.valtype(lx, &mut ps)?;
                        lx.close()?;
                        np += 1;
                        if let Some(n) = names.as_deref_mut() {
                            n.push(Some(id));
                        }
                    } else {
                        let k = self.valtypes(lx, &mut ps)?;
                        np += k;
                        if let Some(n) = names.as_deref_mut() {
                            n.extend((0..k).map(|_| None));
                        }
                    }
                }
                Some("result") => {
                    lx.open();
                    nr += self.valtypes(lx, &mut rs)?;
                }
                _ => break,
            }
        }
        let mut o = vec![0x60];
        uleb(&mut o, np as u64);
        o.extend_from_slice(&ps);
        uleb(&mut o, nr as u64);
        o.extend_from_slice(&rs);
        Ok(o)
    }

    /// A type definition after `(type`, up to its closing `)` (not consumed), as a subtype;
    /// returns the parameter count and, for a function type, its encoding.
    fn typedef(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<(u32, Option<Vec<u8>>)> {
        lx.id();
        if lx.peek() != Tok::L {
            return lx.err("expected a type definition");
        }
        lx.next();
        let kw = lx.atom()?;
        let r = if kw == "sub" {
            let fin = lx.peek() == Tok::Atom("final");
            if fin {
                lx.next();
            }
            let mut supers = Vec::new();
            while let Tok::Atom(a) = lx.peek() {
                lx.next();
                supers.push(self.ty(a)?);
            }
            // a final type without supertypes has the short encoding
            if !(fin && supers.is_empty()) {
                o.push(if fin { 0x4f } else { 0x50 });
                uleb(o, supers.len() as u64);
                for s in supers {
                    uleb(o, s as u64);
                }
            }
            if lx.peek() != Tok::L {
                return lx.err("expected a composite type");
            }
            lx.next();
            let kw = lx.atom()?;
            let r = self.composite(kw, lx, o)?;
            lx.close()?;
            r
        } else {
            self.composite(kw, lx, o)?
        };
        Ok(r)
    }

    /// Returns the parameter count and, for a function type, its encoding (the key that
    /// inline type uses are matched against).
    fn composite(&self, kw: &str, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<(u32, Option<Vec<u8>>)> {
        let r = match kw {
            "func" => {
                let sig = self.signature(lx, None)?;
                o.extend_from_slice(&sig);
                let np = sig[1..].first().copied().unwrap_or(0) as u32;
                // more than 127 parameters would need a longer LEB; not produced
                if np >= 0x80 {
                    return lx.err("too many parameters");
                }
                (np, Some(sig))
            }
            "struct" => {
                let mut fs = Vec::new();
                let mut n = 0;
                while lx.form() == Some("field") {
                    lx.open();
                    if lx.id().is_some() {
                        self.fieldtype(lx, &mut fs)?;
                        n += 1;
                    } else {
                        while lx.peek() != Tok::R {
                            self.fieldtype(lx, &mut fs)?;
                            n += 1;
                        }
                    }
                    lx.close()?;
                }
                o.push(0x5f);
                uleb(o, n);
                o.extend_from_slice(&fs);
                (0, None)
            }
            "array" => {
                o.push(0x5e);
                if lx.form() == Some("field") {
                    lx.open();
                    self.fieldtype(lx, o)?;
                    lx.close()?;
                } else {
                    self.fieldtype(lx, o)?;
                }
                (0, None)
            }
            _ => return lx.err(&format!("unsupported type `{kw}`")),
        };
        lx.close()?;
        Ok(r)
    }
}

// ---------------------------------------------------------------- instructions

#[derive(Clone, Copy)]
enum Imm {
    None,
    Local,
    Global,
    Func,
    Label,
    BrTable,
    Type,
    TypeField,
    TypeN,
    TypeData,
    TypeElem,
    TypeType,
    Data,
    MemInit,
    Mem0,
    Mem00,
    Tag,
    Heap,
    Cast,
    BrOnCast,
    Select,
    Mem(u8),
    I32,
    I64,
    F32,
    F64,
    CallIndirect,
}

fn ops() -> &'static HashMap<String, (u32, Imm)> {
    static OPS: OnceLock<HashMap<String, (u32, Imm)>> = OnceLock::new();
    OPS.get_or_init(|| {
        let mut m: HashMap<String, (u32, Imm)> = HashMap::new();
        let mut code = 0x45u32;
        let mut num = |n: String| {
            m.insert(n, (code, Imm::None));
            code += 1;
        };
        for t in ["i32", "i64"] {
            num(format!("{t}.eqz"));
            for op in ["eq", "ne", "lt_s", "lt_u", "gt_s", "gt_u", "le_s", "le_u", "ge_s", "ge_u"] {
                num(format!("{t}.{op}"));
            }
        }
        for t in ["f32", "f64"] {
            for op in ["eq", "ne", "lt", "gt", "le", "ge"] {
                num(format!("{t}.{op}"));
            }
        }
        for t in ["i32", "i64"] {
            for op in ["clz", "ctz", "popcnt", "add", "sub", "mul", "div_s", "div_u", "rem_s", "rem_u", "and", "or", "xor", "shl", "shr_s", "shr_u", "rotl", "rotr"] {
                num(format!("{t}.{op}"));
            }
        }
        for t in ["f32", "f64"] {
            for op in ["abs", "neg", "ceil", "floor", "trunc", "nearest", "sqrt", "add", "sub", "mul", "div", "min", "max", "copysign"] {
                num(format!("{t}.{op}"));
            }
        }
        for n in [
            "i32.wrap_i64", "i32.trunc_f32_s", "i32.trunc_f32_u", "i32.trunc_f64_s", "i32.trunc_f64_u", "i64.extend_i32_s", "i64.extend_i32_u", "i64.trunc_f32_s", "i64.trunc_f32_u",
            "i64.trunc_f64_s", "i64.trunc_f64_u", "f32.convert_i32_s", "f32.convert_i32_u", "f32.convert_i64_s", "f32.convert_i64_u", "f32.demote_f64", "f64.convert_i32_s",
            "f64.convert_i32_u", "f64.convert_i64_s", "f64.convert_i64_u", "f64.promote_f32", "i32.reinterpret_f32", "i64.reinterpret_f64", "f32.reinterpret_i32",
            "f64.reinterpret_i64", "i32.extend8_s", "i32.extend16_s", "i64.extend8_s", "i64.extend16_s", "i64.extend32_s",
        ] {
            num(n.to_string());
        }
        assert_eq!(code, 0xc5);
        for (i, n) in ["i32.trunc_sat_f32_s", "i32.trunc_sat_f32_u", "i32.trunc_sat_f64_s", "i32.trunc_sat_f64_u", "i64.trunc_sat_f32_s", "i64.trunc_sat_f32_u", "i64.trunc_sat_f64_s", "i64.trunc_sat_f64_u"].iter().enumerate() {
            m.insert(n.to_string(), (0xfc00 + i as u32, Imm::None));
        }
        let mems: [(&str, u32, u8); 23] = [
            ("i32.load", 0x28, 2), ("i64.load", 0x29, 3), ("f32.load", 0x2a, 2), ("f64.load", 0x2b, 3), ("i32.load8_s", 0x2c, 0), ("i32.load8_u", 0x2d, 0),
            ("i32.load16_s", 0x2e, 1), ("i32.load16_u", 0x2f, 1), ("i64.load8_s", 0x30, 0), ("i64.load8_u", 0x31, 0), ("i64.load16_s", 0x32, 1), ("i64.load16_u", 0x33, 1),
            ("i64.load32_s", 0x34, 2), ("i64.load32_u", 0x35, 2), ("i32.store", 0x36, 2), ("i64.store", 0x37, 3), ("f32.store", 0x38, 2), ("f64.store", 0x39, 3),
            ("i32.store8", 0x3a, 0), ("i32.store16", 0x3b, 1), ("i64.store8", 0x3c, 0), ("i64.store16", 0x3d, 1), ("i64.store32", 0x3e, 2),
        ];
        for (n, c, a) in mems {
            m.insert(n.to_string(), (c, Imm::Mem(a)));
        }
        let rest: &[(&str, u32, Imm)] = &[
            ("unreachable", 0x00, Imm::None), ("nop", 0x01, Imm::None), ("return", 0x0f, Imm::None), ("drop", 0x1a, Imm::None), ("throw_ref", 0x0a, Imm::None),
            ("ref.is_null", 0xd1, Imm::None), ("ref.eq", 0xd3, Imm::None), ("ref.as_non_null", 0xd4, Imm::None),
            ("local.get", 0x20, Imm::Local), ("local.set", 0x21, Imm::Local), ("local.tee", 0x22, Imm::Local), ("global.get", 0x23, Imm::Global), ("global.set", 0x24, Imm::Global),
            ("call", 0x10, Imm::Func), ("return_call", 0x12, Imm::Func), ("ref.func", 0xd2, Imm::Func), ("call_indirect", 0x11, Imm::CallIndirect),
            ("br", 0x0c, Imm::Label), ("br_if", 0x0d, Imm::Label), ("br_on_null", 0xd5, Imm::Label), ("br_on_non_null", 0xd6, Imm::Label), ("br_table", 0x0e, Imm::BrTable),
            ("call_ref", 0x14, Imm::Type), ("return_call_ref", 0x15, Imm::Type), ("throw", 0x08, Imm::Tag), ("ref.null", 0xd0, Imm::Heap), ("select", 0x1b, Imm::Select),
            ("i32.const", 0x41, Imm::I32), ("i64.const", 0x42, Imm::I64), ("f32.const", 0x43, Imm::F32), ("f64.const", 0x44, Imm::F64),
            ("memory.size", 0x3f, Imm::Mem0), ("memory.grow", 0x40, Imm::Mem0), ("memory.fill", 0xfc0b, Imm::Mem0), ("memory.copy", 0xfc0a, Imm::Mem00),
            ("memory.init", 0xfc08, Imm::MemInit), ("data.drop", 0xfc09, Imm::Data),
            ("struct.new", 0xfb00, Imm::Type), ("struct.new_default", 0xfb01, Imm::Type), ("struct.get", 0xfb02, Imm::TypeField), ("struct.get_s", 0xfb03, Imm::TypeField),
            ("struct.get_u", 0xfb04, Imm::TypeField), ("struct.set", 0xfb05, Imm::TypeField), ("array.new", 0xfb06, Imm::Type), ("array.new_default", 0xfb07, Imm::Type),
            ("array.new_fixed", 0xfb08, Imm::TypeN), ("array.new_data", 0xfb09, Imm::TypeData), ("array.new_elem", 0xfb0a, Imm::TypeElem), ("array.get", 0xfb0b, Imm::Type),
            ("array.get_s", 0xfb0c, Imm::Type), ("array.get_u", 0xfb0d, Imm::Type), ("array.set", 0xfb0e, Imm::Type), ("array.len", 0xfb0f, Imm::None),
            ("array.fill", 0xfb10, Imm::Type), ("array.copy", 0xfb11, Imm::TypeType), ("array.init_data", 0xfb12, Imm::TypeData), ("array.init_elem", 0xfb13, Imm::TypeElem),
            ("ref.test", 0xfb14, Imm::Cast), ("ref.cast", 0xfb16, Imm::Cast), ("br_on_cast", 0xfb18, Imm::BrOnCast), ("br_on_cast_fail", 0xfb19, Imm::BrOnCast),
            ("any.convert_extern", 0xfb1a, Imm::None), ("extern.convert_any", 0xfb1b, Imm::None), ("ref.i31", 0xfb1c, Imm::None), ("i31.get_s", 0xfb1d, Imm::None),
            ("i31.get_u", 0xfb1e, Imm::None),
        ];
        for &(n, c, i) in rest {
            m.insert(n.to_string(), (c, i));
        }
        m
    })
}

fn opcode(o: &mut Vec<u8>, c: u32) {
    if c < 0x100 {
        o.push(c as u8);
    } else {
        o.push((c >> 8) as u8);
        uleb(o, (c & 0xff) as u64);
    }
}

/// Encodes the instructions of one function body or constant expression.
struct Fe<'a, 'n> {
    n: &'n Names<'a>,
    locals: HashMap<&'a str, u32>,
    labels: Vec<Option<&'a str>>,
    data: bool,
}

impl<'a, 'n> Fe<'a, 'n> {
    fn label(&self, a: &str) -> R<u32> {
        if a.starts_with('$') {
            match self.labels.iter().rev().position(|l| *l == Some(a)) {
                Some(d) => Ok(d as u32),
                None => Err(format!("unknown label `{a}`")),
            }
        } else {
            a.parse().map_err(|_| format!("bad label `{a}`"))
        }
    }

    fn blocktype(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        match lx.form() {
            Some("result") => {
                lx.open();
                let mut t = Vec::new();
                match self.n.valtypes(lx, &mut t)? {
                    0 => o.push(0x40),
                    1 => o.extend_from_slice(&t),
                    _ => return lx.err("multi-value blocks are not supported"),
                }
            }
            Some("type") | Some("param") => return lx.err("typed blocks are not supported"),
            _ => o.push(0x40),
        }
        Ok(())
    }

    /// Instructions up to a `)` or the end (not consumed).
    fn instrs(&mut self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        loop {
            match lx.next() {
                Tok::R => {
                    lx.p -= 1;
                    return Ok(());
                }
                Tok::Eof => return Ok(()),
                Tok::L => self.folded(lx, o)?,
                Tok::Atom(a) => self.flat(a, lx, o)?,
                Tok::Str(_) => return lx.err("unexpected string"),
            }
        }
    }

    fn catches(&self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        let mut cs = Vec::new();
        let mut n = 0;
        while let Some(kw @ ("catch" | "catch_ref" | "catch_all" | "catch_all_ref")) = lx.form() {
            lx.open();
            let (b, tag) = match kw {
                "catch" => (0, true),
                "catch_ref" => (1, true),
                "catch_all" => (2, false),
                _ => (3, false),
            };
            cs.push(b);
            if tag {
                uleb(&mut cs, lookup(&self.n.tags, lx.atom()?, "tag")? as u64);
            }
            // catch labels are resolved outside the try_table block
            uleb(&mut cs, self.label(lx.atom()?)? as u64);
            lx.close()?;
            n += 1;
        }
        uleb(o, n);
        o.extend_from_slice(&cs);
        Ok(())
    }

    /// A folded instruction, after its `(`.
    fn folded(&mut self, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        let kw = lx.atom()?;
        match kw {
            "block" | "loop" => {
                let label = lx.id();
                o.push(if kw == "block" { 0x02 } else { 0x03 });
                self.blocktype(lx, o)?;
                self.labels.push(label);
                self.instrs(lx, o)?;
                lx.close()?;
                self.labels.pop();
                o.push(0x0b);
            }
            "if" => {
                let label = lx.id();
                let mut bt = Vec::new();
                self.blocktype(lx, &mut bt)?;
                loop {
                    match lx.form() {
                        Some("then") => break,
                        Some(_) => {
                            lx.next();
                            self.folded(lx, o)?;
                        }
                        None => return lx.err("expected `(then`"),
                    }
                }
                o.push(0x04);
                o.extend_from_slice(&bt);
                self.labels.push(label);
                lx.open();
                self.instrs(lx, o)?;
                lx.close()?;
                if lx.form() == Some("else") {
                    lx.open();
                    o.push(0x05);
                    self.instrs(lx, o)?;
                    lx.close()?;
                }
                lx.close()?;
                self.labels.pop();
                o.push(0x0b);
            }
            "try_table" => {
                let label = lx.id();
                o.push(0x1f);
                self.blocktype(lx, o)?;
                self.catches(lx, o)?;
                self.labels.push(label);
                self.instrs(lx, o)?;
                lx.close()?;
                self.labels.pop();
                o.push(0x0b);
            }
            "then" | "else" => return lx.err("misplaced `then`/`else`"),
            _ => {
                let mut ins = Vec::new();
                self.op(kw, lx, &mut ins)?;
                loop {
                    match lx.next() {
                        Tok::L => self.folded(lx, o)?,
                        Tok::R => break,
                        t => return lx.err(&format!("unexpected {t:?} in folded instruction")),
                    }
                }
                o.extend_from_slice(&ins);
            }
        }
        Ok(())
    }

    fn flat(&mut self, a: &'a str, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        match a {
            "block" | "loop" | "if" => {
                let label = lx.id();
                o.push(match a {
                    "block" => 0x02,
                    "loop" => 0x03,
                    _ => 0x04,
                });
                self.blocktype(lx, o)?;
                self.labels.push(label);
            }
            "try_table" => {
                let label = lx.id();
                o.push(0x1f);
                self.blocktype(lx, o)?;
                self.catches(lx, o)?;
                self.labels.push(label);
            }
            "else" => {
                lx.id();
                o.push(0x05);
            }
            "end" => {
                lx.id();
                self.labels.pop().ok_or("unbalanced `end`")?;
                o.push(0x0b);
            }
            _ => self.op(a, lx, o)?,
        }
        Ok(())
    }

    fn op(&mut self, a: &str, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
        let Some(&(code, imm)) = ops().get(a) else { return lx.err(&format!("unknown instruction `{a}`")) };
        let n = self.n;
        match imm {
            Imm::Cast => {
                if lx.form() != Some("ref") {
                    return lx.err("expected a reference type");
                }
                lx.open();
                let mut ht = Vec::new();
                let null = n.reftype(lx, &mut ht)?;
                opcode(o, code + null as u32);
                // `reftype` wrote the full reference type; the instruction takes the heap type
                let heap = match ht[0] {
                    0x63 | 0x64 => &ht[1..],
                    _ => &ht[..],
                };
                o.extend_from_slice(heap);
                return Ok(());
            }
            Imm::Select => {
                if lx.form() == Some("result") {
                    lx.open();
                    let mut t = Vec::new();
                    let k = n.valtypes(lx, &mut t)?;
                    o.push(0x1c);
                    uleb(o, k as u64);
                    o.extend_from_slice(&t);
                } else {
                    o.push(0x1b);
                }
                return Ok(());
            }
            _ => opcode(o, code),
        }
        match imm {
            Imm::None => {}
            Imm::Local => {
                let x = lx.atom()?;
                let i = if x.starts_with('$') { *self.locals.get(x).ok_or_else(|| format!("unknown local `{x}`"))? } else { x.parse().map_err(|_| format!("bad local `{x}`"))? };
                uleb(o, i as u64);
            }
            Imm::Global => uleb(o, lookup(&n.globals, lx.atom()?, "global")? as u64),
            Imm::Func => uleb(o, lookup(&n.funcs, lx.atom()?, "function")? as u64),
            Imm::Label => uleb(o, self.label(lx.atom()?)? as u64),
            Imm::BrTable => {
                let mut ls = Vec::new();
                while let Tok::Atom(x) = lx.peek() {
                    if !(x.starts_with('$') || x.as_bytes()[0].is_ascii_digit()) {
                        break;
                    }
                    lx.next();
                    ls.push(self.label(x)?);
                }
                let d = ls.pop().ok_or("`br_table` needs a label")?;
                uleb(o, ls.len() as u64);
                for l in ls {
                    uleb(o, l as u64);
                }
                uleb(o, d as u64);
            }
            Imm::Type => uleb(o, n.ty(lx.atom()?)? as u64),
            Imm::TypeField | Imm::TypeN => {
                uleb(o, n.ty(lx.atom()?)? as u64);
                let f = lx.atom()?;
                uleb(o, f.parse::<u32>().map_err(|_| format!("bad immediate `{f}`"))? as u64);
            }
            Imm::TypeData => {
                uleb(o, n.ty(lx.atom()?)? as u64);
                uleb(o, lookup(&n.datas, lx.atom()?, "data segment")? as u64);
                self.data = true;
            }
            Imm::TypeElem => {
                uleb(o, n.ty(lx.atom()?)? as u64);
                uleb(o, lookup(&n.elems, lx.atom()?, "element segment")? as u64);
            }
            Imm::TypeType => {
                uleb(o, n.ty(lx.atom()?)? as u64);
                uleb(o, n.ty(lx.atom()?)? as u64);
            }
            Imm::Data => {
                uleb(o, lookup(&n.datas, lx.atom()?, "data segment")? as u64);
                self.data = true;
            }
            Imm::MemInit => {
                uleb(o, lookup(&n.datas, lx.atom()?, "data segment")? as u64);
                o.push(0);
                self.data = true;
            }
            Imm::Mem0 => o.push(0),
            Imm::Mem00 => o.extend_from_slice(&[0, 0]),
            Imm::Tag => uleb(o, lookup(&n.tags, lx.atom()?, "tag")? as u64),
            Imm::Heap => n.heap(lx.atom()?, o)?,
            Imm::BrOnCast => {
                let l = self.label(lx.atom()?)?;
                let (mut t1, mut t2) = (Vec::new(), Vec::new());
                let mut nulls = [false; 2];
                for (k, t) in [&mut t1, &mut t2].into_iter().enumerate() {
                    if lx.form() != Some("ref") {
                        return lx.err("expected a reference type");
                    }
                    lx.open();
                    nulls[k] = n.reftype(lx, t)?;
                }
                o.push(nulls[0] as u8 | (nulls[1] as u8) << 1);
                uleb(o, l as u64);
                for t in [&t1, &t2] {
                    o.extend_from_slice(match t[0] {
                        0x63 | 0x64 => &t[1..],
                        _ => &t[..],
                    });
                }
            }
            Imm::Mem(natural) => {
                let (mut align, mut offset) = (natural as u64, 0u64);
                while let Tok::Atom(x) = lx.peek() {
                    if let Some(v) = x.strip_prefix("offset=") {
                        offset = int(v)? as u64;
                    } else if let Some(v) = x.strip_prefix("align=") {
                        align = (int(v)? as u64).trailing_zeros() as u64;
                    } else {
                        break;
                    }
                    lx.next();
                }
                uleb(o, align);
                uleb(o, offset);
            }
            Imm::I32 => {
                let v = int(lx.atom()?)?;
                sleb(o, if v < 0 { v as i32 as i64 } else { v as u32 as i32 as i64 });
            }
            Imm::I64 => {
                let v = int(lx.atom()?)?;
                sleb(o, if v < 0 { v as i64 } else { v as u64 as i64 });
            }
            Imm::F32 => {
                let a = lx.atom()?;
                let v: f32 = float_text(a)?.parse().map_err(|_| format!("bad float `{a}`"))?;
                o.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            Imm::F64 => {
                let a = lx.atom()?;
                let v: f64 = float_text(a)?.parse().map_err(|_| format!("bad float `{a}`"))?;
                o.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            Imm::CallIndirect => {
                let table = match lx.peek() {
                    Tok::Atom(x) => {
                        lx.next();
                        lookup(&n.tables, x, "table")?
                    }
                    _ => 0,
                };
                if lx.form() != Some("type") {
                    return lx.err("`call_indirect` needs `(type ...)`");
                }
                lx.open();
                uleb(o, n.ty(lx.atom()?)? as u64);
                lx.close()?;
                if lx.form().is_some_and(|f| f == "param" || f == "result") {
                    return lx.err("inline `call_indirect` types are not supported");
                }
                uleb(o, table as u64);
            }
            Imm::Cast | Imm::Select => unreachable!(),
        }
        Ok(())
    }
}

// ---------------------------------------------------------------- module

struct Field<'a> {
    kw: &'a str,
    lo: usize,
    hi: usize,
}

struct FuncDef<'a> {
    params: Vec<Option<&'a str>>,
    body: usize,
    hi: usize,
}

pub fn assemble(s: &str) -> R<Vec<u8>> {
    // cut the module into fields
    let mut lx = Lx { s, p: 0, end: s.len() };
    if lx.next() != Tok::L || lx.atom()? != "module" {
        return Err("expected `(module`".into());
    }
    lx.id();
    let mut fields = Vec::new();
    loop {
        match lx.next() {
            Tok::L => {
                let kw = lx.atom()?;
                let lo = lx.p;
                let hi = close_paren(s, lo)?;
                fields.push(Field { kw, lo, hi });
                lx.p = hi + 1;
            }
            Tok::R => break,
            t => return lx.err(&format!("unexpected {t:?} at module level")),
        }
    }
    let fl = |f: &Field| Lx { s, p: f.lo, end: f.hi };

    // index spaces: imports first, then definitions
    let mut n = Names::default();
    let mut counts = [0u32; 8]; // funcs, globals, tags, tables, mems, datas, elems, types
    let mut func_names: Vec<(u32, &str)> = Vec::new();
    for f in fields.iter().filter(|f| f.kw == "import") {
        let mut lx = fl(f);
        lx.string()?;
        lx.string()?;
        if lx.next() != Tok::L {
            return lx.err("expected an import kind");
        }
        let kind = lx.atom()?;
        let id = lx.id();
        let (map, k) = match kind {
            "func" => (&mut n.funcs, 0),
            "global" => (&mut n.globals, 1),
            "tag" => (&mut n.tags, 2),
            "table" => (&mut n.tables, 3),
            "memory" => (&mut n.mems, 4),
            _ => return lx.err(&format!("unsupported import `{kind}`")),
        };
        if let Some(id) = id {
            map.insert(id, counts[k]);
            if k == 0 {
                func_names.push((counts[k], id));
            }
        }
        counts[k] += 1;
    }
    let imported = counts;
    for f in &fields {
        let mut lx = fl(f);
        let (map, k) = match f.kw {
            "func" => (&mut n.funcs, 0),
            "global" => (&mut n.globals, 1),
            "tag" => (&mut n.tags, 2),
            "table" => (&mut n.tables, 3),
            "memory" => (&mut n.mems, 4),
            "data" => (&mut n.datas, 5),
            "elem" => (&mut n.elems, 6),
            "type" => (&mut n.types, 7),
            "rec" => {
                while lx.form() == Some("type") {
                    lx.open();
                    if let Some(id) = lx.id() {
                        n.types.insert(id, counts[7]);
                    }
                    counts[7] += 1;
                    lx.skip_form()?;
                }
                continue;
            }
            "import" | "export" | "start" => continue,
            k => return lx.err(&format!("unsupported module field `{k}`")),
        };
        if let Some(id) = lx.id() {
            map.insert(id, counts[k]);
            if k == 0 {
                func_names.push((counts[k], id));
            }
        }
        counts[k] += 1;
    }
    let n = n;

    // types, signatures and everything but function bodies, in text order
    let mut types = Vec::new(); // encoded type section entries
    let mut ntype_entries = 0;
    let mut nparams: Vec<u32> = Vec::new(); // per type index
    let mut keys: HashMap<Vec<u8>, u32> = HashMap::new();
    let mut implicit: Vec<Vec<u8>> = Vec::new();
    let explicit = counts[7];
    let key = |sig: Vec<u8>, implicit: &mut Vec<Vec<u8>>, keys: &mut HashMap<Vec<u8>, u32>| -> u32 {
        if let Some(&i) = keys.get(&sig) {
            return i;
        }
        let i = explicit + implicit.len() as u32;
        keys.insert(sig.clone(), i);
        implicit.push(sig);
        i
    };
    let (mut imports, mut nimports) = (Vec::new(), 0);
    let mut func_types = Vec::new();
    let mut defs: Vec<FuncDef> = Vec::new();
    let (mut tables, mut ntables) = (Vec::new(), 0);
    let (mut mems, mut nmems) = (Vec::new(), 0);
    let (mut tags, mut ntags) = (Vec::new(), 0);
    let (mut globals, mut nglobals) = (Vec::new(), 0);
    let (mut exports, mut nexports) = (Vec::new(), 0);
    let (mut elems, mut nelems) = (Vec::new(), 0);
    let (mut datas, mut ndatas) = (Vec::new(), 0);
    let mut start = None;
    let mut func_index = imported[0];
    for f in &fields {
        let mut lx = fl(f);
        match f.kw {
            "type" => {
                let (np, sig) = n.typedef(&mut lx, &mut types)?;
                if let Some(sig) = sig {
                    keys.entry(sig).or_insert(nparams.len() as u32);
                }
                nparams.push(np);
                ntype_entries += 1;
            }
            "rec" => {
                let mut body = Vec::new();
                let mut k = 0;
                while lx.form() == Some("type") {
                    lx.open();
                    let (np, _) = n.typedef(&mut lx, &mut body)?;
                    lx.close()?;
                    nparams.push(np);
                    k += 1;
                }
                types.push(0x4e);
                uleb(&mut types, k);
                types.extend_from_slice(&body);
                ntype_entries += 1;
            }
            "import" => {
                let m = unescape(lx.string()?)?;
                let nm = unescape(lx.string()?)?;
                name(&mut imports, &m);
                name(&mut imports, &nm);
                lx.next();
                let kind = lx.atom()?;
                lx.id();
                match kind {
                    "func" | "tag" => {
                        let explicit_ty = if lx.form() == Some("type") {
                            lx.open();
                            let t = n.ty(lx.atom()?)?;
                            lx.close()?;
                            Some(t)
                        } else {
                            None
                        };
                        let sig = n.signature(&mut lx, None)?;
                        let t = match explicit_ty {
                            Some(t) => t,
                            None => key(sig, &mut implicit, &mut keys),
                        };
                        if kind == "func" {
                            imports.push(0x00);
                        } else {
                            imports.extend_from_slice(&[0x04, 0x00]);
                        }
                        uleb(&mut imports, t as u64);
                    }
                    _ => return lx.err(&format!("unsupported import `{kind}`")),
                }
                lx.close()?;
                nimports += 1;
            }
            "func" => {
                lx.id();
                inline_exports(&mut lx, 0, func_index, &mut exports, &mut nexports)?;
                let explicit_ty = if lx.form() == Some("type") {
                    lx.open();
                    let t = n.ty(lx.atom()?)?;
                    lx.close()?;
                    Some(t)
                } else {
                    None
                };
                let mut params = Vec::new();
                let sig = n.signature(&mut lx, Some(&mut params))?;
                let t = match explicit_ty {
                    Some(t) => {
                        if params.is_empty() {
                            let np = *nparams.get(t as usize).ok_or("bad type index")?;
                            params = vec![None; np as usize];
                        }
                        t
                    }
                    None => key(sig, &mut implicit, &mut keys),
                };
                func_types.push(t);
                defs.push(FuncDef { params, body: lx.p, hi: f.hi });
                func_index += 1;
            }
            "tag" => {
                let id = lx.id();
                let idx = id.map_or(imported[2] + ntags as u32, |i| n.tags[i]);
                inline_exports(&mut lx, 4, idx, &mut exports, &mut nexports)?;
                let explicit_ty = if lx.form() == Some("type") {
                    lx.open();
                    let t = n.ty(lx.atom()?)?;
                    lx.close()?;
                    Some(t)
                } else {
                    None
                };
                let sig = n.signature(&mut lx, None)?;
                let t = match explicit_ty {
                    Some(t) => t,
                    None => key(sig, &mut implicit, &mut keys),
                };
                tags.push(0x00);
                uleb(&mut tags, t as u64);
                ntags += 1;
            }
            "memory" => {
                let id = lx.id();
                let idx = id.map_or(imported[4] + nmems, |i| n.mems[i]);
                inline_exports(&mut lx, 2, idx, &mut exports, &mut nexports)?;
                let min = int(lx.atom()?)? as u64;
                match lx.peek() {
                    Tok::Atom(x) => {
                        lx.next();
                        mems.push(0x01);
                        uleb(&mut mems, min);
                        uleb(&mut mems, int(x)? as u64);
                    }
                    _ => {
                        mems.push(0x00);
                        uleb(&mut mems, min);
                    }
                }
                nmems += 1;
            }
            "table" => {
                let id = lx.id();
                let idx = id.map_or(imported[3] + ntables, |i| n.tables[i]);
                inline_exports(&mut lx, 1, idx, &mut exports, &mut nexports)?;
                let min = int(lx.atom()?)? as u64;
                let max = match lx.peek() {
                    Tok::Atom(x) if x.as_bytes()[0].is_ascii_digit() => {
                        lx.next();
                        Some(int(x)? as u64)
                    }
                    _ => None,
                };
                n.valtype(&mut lx, &mut tables)?;
                match max {
                    Some(m) => {
                        tables.push(0x01);
                        uleb(&mut tables, min);
                        uleb(&mut tables, m);
                    }
                    None => {
                        tables.push(0x00);
                        uleb(&mut tables, min);
                    }
                }
                ntables += 1;
            }
            "global" => {
                let id = lx.id();
                let idx = id.map_or(imported[1] + nglobals, |i| n.globals[i]);
                inline_exports(&mut lx, 3, idx, &mut exports, &mut nexports)?;
                if lx.form() == Some("mut") {
                    lx.open();
                    n.valtype(&mut lx, &mut globals)?;
                    lx.close()?;
                    globals.push(1);
                } else {
                    n.valtype(&mut lx, &mut globals)?;
                    globals.push(0);
                }
                const_expr(&n, &mut lx, &mut globals)?;
                nglobals += 1;
            }
            "export" => {
                name(&mut exports, &unescape(lx.string()?)?);
                lx.next();
                let kind = lx.atom()?;
                let x = lx.atom()?;
                let (k, i) = match kind {
                    "func" => (0, lookup(&n.funcs, x, "function")?),
                    "table" => (1, lookup(&n.tables, x, "table")?),
                    "memory" => (2, lookup(&n.mems, x, "memory")?),
                    "global" => (3, lookup(&n.globals, x, "global")?),
                    "tag" => (4, lookup(&n.tags, x, "tag")?),
                    _ => return lx.err(&format!("unsupported export `{kind}`")),
                };
                lx.close()?;
                exports.push(k);
                uleb(&mut exports, i as u64);
                nexports += 1;
            }
            "start" => start = Some(lookup(&n.funcs, lx.atom()?, "function")?),
            "elem" => {
                lx.id();
                if lx.peek() == Tok::Atom("declare") {
                    lx.next();
                    elems.extend_from_slice(&[0x03, 0x00]);
                    elem_funcs(&n, &mut lx, &mut elems)?;
                } else if lx.form() == Some("table") {
                    lx.open();
                    let t = lookup(&n.tables, lx.atom()?, "table")?;
                    lx.close()?;
                    elems.push(0x02);
                    uleb(&mut elems, t as u64);
                    offset_expr(&n, &mut lx, &mut elems)?;
                    elems.push(0x00);
                    elem_funcs(&n, &mut lx, &mut elems)?;
                } else if lx.peek() == Tok::L {
                    elems.push(0x00);
                    offset_expr(&n, &mut lx, &mut elems)?;
                    elem_funcs(&n, &mut lx, &mut elems)?;
                } else {
                    elems.extend_from_slice(&[0x01, 0x00]);
                    elem_funcs(&n, &mut lx, &mut elems)?;
                }
                nelems += 1;
            }
            "data" => {
                lx.id();
                if lx.form() == Some("memory") {
                    return lx.err("explicit data memories are not supported");
                }
                if lx.peek() == Tok::L {
                    datas.push(0x00);
                    offset_expr(&n, &mut lx, &mut datas)?;
                } else {
                    datas.push(0x01);
                }
                let mut bytes = Vec::new();
                while let Tok::Str(x) = lx.peek() {
                    lx.next();
                    bytes.extend_from_slice(&unescape(x)?);
                }
                name(&mut datas, &bytes);
                ndatas += 1;
            }
            _ => {}
        }
    }

    // function bodies, on several threads
    let next = std::sync::atomic::AtomicUsize::new(0);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(16).min(defs.len() / 64 + 1);
    let n = &n;
    let defs = &defs;
    let encode = |d: &FuncDef| -> R<(Vec<u8>, bool)> {
        let mut lx = Lx { s, p: d.body, end: d.hi };
        let mut fe = Fe { n, locals: HashMap::new(), labels: Vec::new(), data: false };
        for (i, p) in d.params.iter().enumerate() {
            if let Some(p) = p {
                fe.locals.insert(*p, i as u32);
            }
        }
        let mut next_local = d.params.len() as u32;
        let mut runs: Vec<(u32, Vec<u8>)> = Vec::new();
        while lx.form() == Some("local") {
            lx.open();
            let mut add = |t: Vec<u8>| match runs.last_mut() {
                Some((k, last)) if *last == t => *k += 1,
                _ => runs.push((1, t)),
            };
            if let Some(id) = lx.id() {
                let mut t = Vec::new();
                n.valtype(&mut lx, &mut t)?;
                lx.close()?;
                add(t);
                fe.locals.insert(id, next_local);
                next_local += 1;
            } else {
                while lx.peek() != Tok::R {
                    let mut t = Vec::new();
                    n.valtype(&mut lx, &mut t)?;
                    add(t);
                    next_local += 1;
                }
                lx.close()?;
            }
        }
        let mut code = Vec::with_capacity((d.hi - d.body) / 6);
        uleb(&mut code, runs.len() as u64);
        for (k, t) in &runs {
            uleb(&mut code, *k as u64);
            code.extend_from_slice(t);
        }
        fe.instrs(&mut lx, &mut code)?;
        if lx.next() != Tok::Eof {
            return lx.err("unexpected `)` in function body");
        }
        if !fe.labels.is_empty() {
            return lx.err("unclosed block");
        }
        code.push(0x0b);
        Ok((code, fe.data))
    };
    let mut bodies: Vec<Option<R<(Vec<u8>, bool)>>> = (0..defs.len()).map(|_| None).collect();
    if threads <= 1 {
        for (i, d) in defs.iter().enumerate() {
            bodies[i] = Some(encode(d));
        }
    } else {
        let done: Vec<Vec<(usize, R<(Vec<u8>, bool)>)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads)
                .map(|_| {
                    sc.spawn(|| {
                        let mut mine = Vec::new();
                        loop {
                            // small batches keep the threads busy to the end
                            let k = next.fetch_add(16, std::sync::atomic::Ordering::Relaxed);
                            if k >= defs.len() {
                                break;
                            }
                            for i in k..(k + 16).min(defs.len()) {
                                mine.push((i, encode(&defs[i])));
                            }
                        }
                        mine
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for (i, r) in done.into_iter().flatten() {
            bodies[i] = Some(r);
        }
    }

    // sections
    let mut out = b"\0asm\x01\0\0\0".to_vec();
    if ntype_entries + implicit.len() > 0 {
        for sig in &implicit {
            types.extend_from_slice(sig);
        }
        section(&mut out, 1, ntype_entries + implicit.len(), &types);
    }
    if nimports > 0 {
        section(&mut out, 2, nimports, &imports);
    }
    if !func_types.is_empty() {
        let mut b = Vec::new();
        for t in &func_types {
            uleb(&mut b, *t as u64);
        }
        section(&mut out, 3, func_types.len(), &b);
    }
    if ntables > 0 {
        section(&mut out, 4, ntables as usize, &tables);
    }
    if nmems > 0 {
        section(&mut out, 5, nmems as usize, &mems);
    }
    if ntags > 0 {
        section(&mut out, 13, ntags, &tags);
    }
    if nglobals > 0 {
        section(&mut out, 6, nglobals as usize, &globals);
    }
    if nexports > 0 {
        section(&mut out, 7, nexports, &exports);
    }
    if let Some(st) = start {
        out.push(8);
        let mut b = Vec::new();
        uleb(&mut b, st as u64);
        uleb(&mut out, b.len() as u64);
        out.extend_from_slice(&b);
    }
    if nelems > 0 {
        section(&mut out, 9, nelems, &elems);
    }
    let mut code = Vec::new();
    let mut needs_count = false;
    for b in bodies {
        let (body, data) = b.ok_or("missing body")??;
        needs_count |= data;
        uleb(&mut code, body.len() as u64);
        code.extend_from_slice(&body);
    }
    if needs_count {
        out.push(12);
        let mut b = Vec::new();
        uleb(&mut b, ndatas as u64);
        uleb(&mut out, b.len() as u64);
        out.extend_from_slice(&b);
    }
    if !defs.is_empty() {
        section(&mut out, 10, defs.len(), &code);
    }
    if ndatas > 0 {
        section(&mut out, 11, ndatas, &datas);
    }
    // name section with function names, for backtraces
    if !func_names.is_empty() {
        func_names.sort_by_key(|f| f.0);
        let mut sub = Vec::new();
        for (i, f) in &func_names {
            uleb(&mut sub, *i as u64);
            name(&mut sub, f[1..].as_bytes());
        }
        let mut c = Vec::new();
        name(&mut c, b"name");
        c.push(1);
        let mut v = Vec::new();
        uleb(&mut v, func_names.len() as u64);
        v.extend_from_slice(&sub);
        uleb(&mut c, v.len() as u64);
        c.extend_from_slice(&v);
        out.push(0);
        uleb(&mut out, c.len() as u64);
        out.extend_from_slice(&c);
    }
    Ok(out)
}

fn inline_exports(lx: &mut Lx<'_>, kind: u8, idx: u32, exports: &mut Vec<u8>, nexports: &mut usize) -> R<()> {
    while lx.form() == Some("export") {
        lx.open();
        name(exports, &unescape(lx.string()?)?);
        exports.push(kind);
        uleb(exports, idx as u64);
        lx.close()?;
        *nexports += 1;
    }
    Ok(())
}

fn const_expr<'a>(n: &Names<'a>, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
    let mut fe = Fe { n, locals: HashMap::new(), labels: Vec::new(), data: false };
    fe.instrs(lx, o)?;
    o.push(0x0b);
    Ok(())
}

fn elem_funcs<'a>(n: &Names<'a>, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
    if lx.atom()? != "func" {
        return lx.err("only `func` element segments are supported");
    }
    let mut fs = Vec::new();
    let mut k = 0;
    while let Tok::Atom(x) = lx.peek() {
        lx.next();
        uleb(&mut fs, lookup(&n.funcs, x, "function")? as u64);
        k += 1;
    }
    uleb(o, k);
    o.extend_from_slice(&fs);
    Ok(())
}

/// An active segment's offset: `(offset instr*)` or one folded instruction.
fn offset_expr<'a>(n: &Names<'a>, lx: &mut Lx<'a>, o: &mut Vec<u8>) -> R<()> {
    if lx.form() == Some("offset") {
        lx.open();
        let mut inner = lx.clone();
        let end = close_paren(lx.s, lx.p)?;
        inner.end = end;
        const_expr(n, &mut inner, o)?;
        lx.p = end + 1;
    } else {
        let start = lx.p;
        lx.next();
        let end = close_paren(lx.s, lx.p)?;
        let mut inner = Lx { s: lx.s, p: start, end: end + 1 };
        const_expr(n, &mut inner, o)?;
        lx.p = end + 1;
    }
    Ok(())
}

/// Compares the sections of two modules other than custom sections; `None` when they agree.
pub fn compare(a: &[u8], b: &[u8]) -> Option<String> {
    fn sections(m: &[u8]) -> Result<Vec<(u8, &[u8])>, String> {
        let mut out = Vec::new();
        for p in wasmparser::Parser::new(0).parse_all(m) {
            let p = p.map_err(|e| e.to_string())?;
            if let Some((id, range)) = p.as_section() {
                if id != 0 {
                    out.push((id, &m[range.start as usize..range.end as usize]));
                }
            }
        }
        Ok(out)
    }
    let (sa, sb) = match (sections(a), sections(b)) {
        (Ok(x), Ok(y)) => (x, y),
        (Err(e), _) | (_, Err(e)) => return Some(e),
    };
    let ids = |s: &[(u8, &[u8])]| s.iter().map(|x| x.0).collect::<Vec<_>>();
    if ids(&sa) != ids(&sb) {
        return Some(format!("section ids differ: {:?} vs {:?}", ids(&sa), ids(&sb)));
    }
    for ((id, x), (_, y)) in sa.iter().zip(&sb) {
        if x != y {
            let at = x.iter().zip(y.iter()).position(|(p, q)| p != q).unwrap_or(x.len().min(y.len()));
            let mut msg = format!("section {id} differs at byte {at} (lengths {} and {})", x.len(), y.len());
            if *id == 10 {
                if let (Ok(ca), Ok(cb)) = (wasmparser::CodeSectionReader::new(wasmparser::BinaryReader::new(x, 0)), wasmparser::CodeSectionReader::new(wasmparser::BinaryReader::new(y, 0))) {
                    for (i, (fa, fb)) in ca.into_iter().zip(cb).enumerate() {
                        if let (Ok(fa), Ok(fb)) = (fa, fb) {
                            let (ra, rb) = (fa.range(), fb.range());
                            if x[ra.start as usize..ra.end as usize] != y[rb.start as usize..rb.end as usize] {
                                msg.push_str(&format!("; first differing body: defined function {i}"));
                                break;
                            }
                        }
                    }
                }
            }
            return Some(msg);
        }
    }
    None
}
