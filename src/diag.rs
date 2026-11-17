//! Source spans and diagnostics.

use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub file: u32,
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(file: u32, lo: u32, hi: u32) -> Span {
        Span { file, lo, hi }
    }
    pub fn to(self, other: Span) -> Span {
        if self.file != other.file {
            return self;
        }
        Span { file: self.file, lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }
}

#[derive(Clone, Debug)]
pub struct Diag {
    pub span: Span,
    pub msg: String,
    pub notes: Vec<String>,
    pub warning: bool,
}

impl Diag {
    pub fn new(span: Span, msg: impl Into<String>) -> Diag {
        Diag { span, msg: msg.into(), notes: Vec::new(), warning: false }
    }
    pub fn note(mut self, n: impl Into<String>) -> Diag {
        self.notes.push(n.into());
        self
    }
}

pub struct SourceFile {
    pub path: String,
    pub text: String,
}

#[derive(Default)]
pub struct SourceMap {
    pub files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add(&mut self, path: String, text: String) -> u32 {
        self.files.push(SourceFile { path, text });
        (self.files.len() - 1) as u32
    }

    pub fn line_col(&self, span: Span) -> (usize, usize) {
        let f = &self.files[span.file as usize];
        let lo = (span.lo as usize).min(f.text.len());
        let before = &f.text[..lo];
        let line = before.matches('\n').count() + 1;
        let col = before.rsplit('\n').next().map(|s| s.chars().count()).unwrap_or(0) + 1;
        (line, col)
    }

    pub fn render(&self, d: &Diag) -> String {
        let mut out = String::new();
        let kind = if d.warning { "warning" } else { "error" };
        let _ = writeln!(out, "{kind}: {}", d.msg);
        if let Some(f) = self.files.get(d.span.file as usize) {
            let (line, col) = self.line_col(d.span);
            let _ = writeln!(out, "  --> {}:{}:{}", f.path, line, col);
            if let Some(text) = f.text.lines().nth(line - 1) {
                let num = line.to_string();
                let pad = " ".repeat(num.len());
                let _ = writeln!(out, " {pad} |");
                let _ = writeln!(out, " {num} | {text}");
                let lo = d.span.lo as usize;
                let hi = (d.span.hi as usize).max(lo + 1);
                let line_start = f.text[..lo.min(f.text.len())].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = line_start + text.len();
                let width = f.text.get(lo..hi.min(line_end)).map(|s| s.chars().count()).unwrap_or(1).max(1);
                let lead: String = text.chars().take(col - 1).map(|c| if c == '\t' { '\t' } else { ' ' }).collect();
                let _ = writeln!(out, " {pad} | {lead}{}", "^".repeat(width));
            }
        }
        for n in &d.notes {
            let _ = writeln!(out, "  = note: {n}");
        }
        out
    }
}
