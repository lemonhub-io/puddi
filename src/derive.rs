//! `@derive(...)`: generates trait impls as Puddi source text.

use std::fmt::Write as _;

use crate::ast::{self, ItemKind, VariantKind};
use crate::diag::Diag;

/// Returns generated source for all derives in a module.
pub fn derive_source(m: &ast::Module) -> Result<String, Vec<Diag>> {
    let mut out = String::new();
    let mut errs = Vec::new();
    for item in &m.items {
        for a in &item.attrs {
            if a.name.name != "derive" {
                continue;
            }
            let (name, generics) = match &item.kind {
                ItemKind::Struct(s) => (&s.name.name, &s.generics),
                ItemKind::Enum(e) => (&e.name.name, &e.generics),
                _ => {
                    errs.push(Diag::new(a.span, "`@derive` can only be used on structs and enums"));
                    continue;
                }
            };
            for arg in &a.args {
                let ast::AttrArg::Ident(t) = arg else {
                    errs.push(Diag::new(a.span, "`@derive` takes trait names"));
                    continue;
                };
                let tr = t.name.as_str();
                if !["Eq", "Ord", "Hash", "Show", "Transfer"].contains(&tr) {
                    errs.push(Diag::new(t.span, format!("cannot derive `{tr}`: only `Eq`, `Ord`, `Hash`, `Show` and `Transfer` can be derived")));
                    continue;
                }
                let gnames: Vec<&str> = generics.iter().map(|g| g.name.name.as_str()).collect();
                let header = if gnames.is_empty() {
                    format!("impl {tr} for {name}")
                } else {
                    let params: Vec<String> = gnames.iter().map(|g| format!("{g}: {tr}")).collect();
                    format!("impl<{}> {tr} for {name}<{}>", params.join(", "), gnames.join(", "))
                };
                let ty = if gnames.is_empty() { name.clone() } else { format!("{name}<{}>", gnames.join(", ")) };
                let body = match &item.kind {
                    ItemKind::Struct(s) => struct_body(tr, name, &ty, &s.fields),
                    ItemKind::Enum(e) => enum_body(tr, name, &ty, &e.variants, &gnames, &mut out),
                    _ => unreachable!(),
                };
                let _ = writeln!(out, "{header} {{\n{body}}}\n");
            }
        }
    }
    if errs.is_empty() { Ok(out) } else { Err(errs) }
}

fn struct_body(tr: &str, name: &str, ty: &str, fields: &[ast::Field]) -> String {
    let fs: Vec<&str> = fields.iter().map(|f| f.name.name.as_str()).collect();
    match tr {
        "Eq" => {
            let mut e: Vec<String> = fs.iter().map(|f| format!("self.{f} == other.{f}")).collect();
            if e.is_empty() {
                e.push("true".into());
            }
            format!("    fn eq(self, other: {ty}) -> Bool {{ {} }}\n", e.join(" && "))
        }
        "Ord" => {
            let mut s = format!("    fn cmp(self, other: {ty}) -> Ordering {{\n");
            for f in &fs {
                let _ = writeln!(s, "        let c = self.{f}.cmp(other.{f})\n        if !c.is_eq() {{ return c }}");
            }
            s.push_str("        Ordering.Equal\n    }\n");
            s
        }
        "Hash" => {
            let mut e = "0".to_string();
            for f in &fs {
                e = format!("__hash_mix({e}, self.{f}.hash())");
            }
            format!("    fn hash(self) -> Int {{ {e} }}\n")
        }
        "Show" => {
            if fs.is_empty() {
                return format!("    fn show(self) -> Str {{ \"{name}\" }}\n");
            }
            let parts: Vec<String> = fs.iter().map(|f| format!("{f}: {{self.{f}.show_nested()}}")).collect();
            format!("    fn show(self) -> Str {{ \"{name} \\{{ {} \\}}\" }}\n", parts.join(", "))
        }
        "Transfer" => {
            let enc: String = fs.iter().map(|f| format!("        self.{f}.__enc(b)\n")).collect();
            let dec: Vec<String> = fs.iter().map(|f| format!("{f}: __tdec(r)")).collect();
            let lit = if fs.is_empty() { format!("{name} {{}}") } else { format!("{name} {{ {} }}", dec.join(", ")) };
            format!("    fn __enc(self, b: StrBuilder) {{\n{enc}    }}\n    fn __dec(r: __Reader) -> {ty} {{ {lit} }}\n")
        }
        _ => String::new(),
    }
}

fn variant_pat(name: &str, v: &ast::Variant, prefix: &str) -> (String, Vec<String>) {
    let vn = &v.name.name;
    match &v.kind {
        VariantKind::Unit => (format!("{name}.{vn}"), Vec::new()),
        VariantKind::Tuple(ts) => {
            let binds: Vec<String> = (0..ts.len()).map(|i| format!("{prefix}{i}")).collect();
            (format!("{name}.{vn}({})", binds.join(", ")), binds)
        }
        VariantKind::Record(fs) => {
            let binds: Vec<String> = (0..fs.len()).map(|i| format!("{prefix}{i}")).collect();
            let parts: Vec<String> = fs.iter().zip(&binds).map(|(f, b)| format!("{}: {b}", f.name.name)).collect();
            (format!("{name}.{vn} {{ {} }}", parts.join(", ")), binds)
        }
    }
}

fn enum_body(tr: &str, name: &str, ty: &str, vs: &[ast::Variant], gnames: &[&str], extra: &mut String) -> String {
    match tr {
        "Eq" => {
            let mut s = format!("    fn eq(self, other: {ty}) -> Bool {{\n        match (self, other) {{\n");
            for v in vs {
                let (pa, ba) = variant_pat(name, v, "a");
                let (pb, bb) = variant_pat(name, v, "b");
                let mut e: Vec<String> = ba.iter().zip(&bb).map(|(a, b)| format!("{a} == {b}")).collect();
                if e.is_empty() {
                    e.push("true".into());
                }
                let _ = writeln!(s, "            ({pa}, {pb}) -> {}", e.join(" && "));
            }
            if vs.len() > 1 {
                s.push_str("            _ -> false\n");
            }
            s.push_str("        }\n    }\n");
            s
        }
        "Ord" => {
            let tagfn = format!("__derive_tag_{name}");
            let g = if gnames.is_empty() { String::new() } else { format!("<{}>", gnames.join(", ")) };
            let _ = writeln!(extra, "fn {tagfn}{g}(v: {ty}) -> Int {{\n    match v {{");
            for (i, v) in vs.iter().enumerate() {
                let (p, _) = variant_pat(name, v, "_x");
                let p = p.replace("_x", "_");
                let p = strip_record_binds(&p);
                let _ = writeln!(extra, "        {p} -> {i}");
            }
            extra.push_str("    }\n}\n");
            let mut s = format!("    fn cmp(self, other: {ty}) -> Ordering {{\n        let ta = {tagfn}(self)\n        let tb = {tagfn}(other)\n        if ta != tb {{ return ta.cmp(tb) }}\n        match (self, other) {{\n");
            for v in vs {
                let (pa, ba) = variant_pat(name, v, "a");
                let (pb, bb) = variant_pat(name, v, "b");
                let mut body = String::from("{\n");
                for (a, b) in ba.iter().zip(&bb) {
                    let _ = writeln!(body, "                let c = {a}.cmp({b})\n                if !c.is_eq() {{ return c }}");
                }
                body.push_str("                Ordering.Equal\n            }");
                let _ = writeln!(s, "            ({pa}, {pb}) -> {body}");
            }
            if vs.len() > 1 {
                s.push_str("            _ -> Ordering.Equal\n");
            }
            s.push_str("        }\n    }\n");
            s
        }
        "Hash" => {
            let mut s = "    fn hash(self) -> Int {\n        match self {\n".to_string();
            for (i, v) in vs.iter().enumerate() {
                let (p, bs) = variant_pat(name, v, "a");
                let mut e = format!("{i}");
                for b in &bs {
                    e = format!("__hash_mix({e}, {b}.hash())");
                }
                let _ = writeln!(s, "            {p} -> {e}");
            }
            s.push_str("        }\n    }\n");
            s
        }
        "Show" => {
            let mut s = "    fn show(self) -> Str {\n        match self {\n".to_string();
            for v in vs {
                let (p, bs) = variant_pat(name, v, "a");
                let vn = &v.name.name;
                let text = match &v.kind {
                    VariantKind::Unit => format!("\"{vn}\""),
                    VariantKind::Tuple(_) => {
                        let parts: Vec<String> = bs.iter().map(|b| format!("{{{b}.show_nested()}}")).collect();
                        format!("\"{vn}({})\"", parts.join(", "))
                    }
                    VariantKind::Record(fs) => {
                        let parts: Vec<String> = fs.iter().zip(&bs).map(|(f, b)| format!("{}: {{{b}.show_nested()}}", f.name.name)).collect();
                        format!("\"{vn} \\{{ {} \\}}\"", parts.join(", "))
                    }
                };
                let _ = writeln!(s, "            {p} -> {text}");
            }
            s.push_str("        }\n    }\n");
            s
        }
        "Transfer" => {
            let mut s = "    fn __enc(self, b: StrBuilder) {\n        match self {\n".to_string();
            for (i, v) in vs.iter().enumerate() {
                let (p, bs) = variant_pat(name, v, "a");
                let mut body = format!("{{\n                __enc_int(b, {i})\n");
                for x in &bs {
                    let _ = writeln!(body, "                {x}.__enc(b)");
                }
                body.push_str("            }");
                let _ = writeln!(s, "            {p} -> {body}");
            }
            s.push_str("        }\n    }\n");
            let _ = write!(s, "    fn __dec(r: __Reader) -> {ty} {{\n        let t = r.int()\n");
            for (i, v) in vs.iter().enumerate() {
                let vn = &v.name.name;
                let e = match &v.kind {
                    VariantKind::Unit => format!("{name}.{vn}"),
                    VariantKind::Tuple(ts) => format!("{name}.{vn}({})", vec!["__tdec(r)"; ts.len()].join(", ")),
                    VariantKind::Record(fs) => format!("{name}.{vn} {{ {} }}", fs.iter().map(|f| format!("{}: __tdec(r)", f.name.name)).collect::<Vec<_>>().join(", ")),
                };
                let _ = writeln!(s, "        if t == {i} {{ return {e} }}");
            }
            s.push_str("        panic(\"malformed worker message\")\n    }\n");
            s
        }
        _ => String::new(),
    }
}

/// `E.V { f: _, g: _ }` -> `E.V { .. }`
fn strip_record_binds(p: &str) -> String {
    match p.find(" {") {
        Some(i) => format!("{} {{ .. }}", &p[..i]),
        None => p.to_string(),
    }
}
