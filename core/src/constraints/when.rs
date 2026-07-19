//! `when` 式（宣言式のみ, D-019 / D-026）。
//!
//! 文法（EBNF, docs/design.md 6.7）:
//! ```text
//! expr   := term ( "|" term )*
//! term   := factor ( "&" factor )*
//! factor := cond | "(" expr ")"
//! cond   := "{" key "=" val ( "," key "=" val )* "}"
//! ```
//! 比較演算子は6種: `equals` / `not_equal` / `gt` / `lt` / `in` / `matches`。
//! 括弧 `()` で `&`/`|` の優先順位をグループ化できる（既定 `&` > `|`）。

use crate::error::{bail, Result};
use anyhow::anyhow;
use regex::Regex;
use std::collections::BTreeMap;

/// 条件の対象（列 or 行）。行対象の意味論は暫定（issue I-WHEN 参照）。
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Column(String),
    Row(String),
}

/// 比較演算子（6種, D-019）。
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Equals,
    NotEqual,
    Gt,
    Lt,
    In,
    Matches,
}

impl CompareOp {
    fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "equals" => CompareOp::Equals,
            "not_equal" => CompareOp::NotEqual,
            "gt" => CompareOp::Gt,
            "lt" => CompareOp::Lt,
            "in" => CompareOp::In,
            "matches" => CompareOp::Matches,
            _ => return None,
        })
    }
}

/// 条件値（`in` はリスト, それ以外は単値）。
#[derive(Debug, Clone, PartialEq)]
pub enum CondValue {
    One(String),
    Many(Vec<String>),
}

/// 1つの条件 `{column=.., op=..}`。
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    pub target: Target,
    pub op: CompareOp,
    pub value: CondValue,
}

impl Condition {
    /// 現在行のカラム値（`cols`）と行ラベル（`row_label`）に対して真偽を返す。
    pub fn eval(&self, cols: &BTreeMap<&str, &str>, row_label: Option<&str>) -> bool {
        let actual: Option<&str> = match &self.target {
            Target::Column(name) => cols.get(name.as_str()).copied(),
            // 行対象は暫定: 現在行のラベルと比較（name は無視, issue I-WHEN）
            Target::Row(_) => row_label,
        };
        let Some(actual) = actual else { return false };
        match (&self.op, &self.value) {
            (CompareOp::Equals, CondValue::One(v)) => actual == v,
            (CompareOp::NotEqual, CondValue::One(v)) => actual != v,
            (CompareOp::Gt, CondValue::One(v)) => num_cmp(actual, v).map_or(false, |o| o.is_gt()),
            (CompareOp::Lt, CondValue::One(v)) => num_cmp(actual, v).map_or(false, |o| o.is_lt()),
            (CompareOp::In, CondValue::Many(vs)) => vs.iter().any(|v| v == actual),
            (CompareOp::Matches, CondValue::One(v)) => {
                Regex::new(v).map_or(false, |re| re.is_match(actual))
            }
            _ => false, // op と value の型不一致（parse 段で基本弾く）
        }
    }
}

fn num_cmp(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let (x, y) = (a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?);
    x.partial_cmp(&y)
}

/// `when` 式の AST。
#[derive(Debug, Clone, PartialEq)]
pub enum WhenExpr {
    Or(Vec<WhenExpr>),
    And(Vec<WhenExpr>),
    Cond(Condition),
}

impl WhenExpr {
    /// 文字列をパースする。
    pub fn parse(src: &str) -> Result<Self> {
        let tokens = tokenize(src)?;
        let mut p = Parser { tokens, pos: 0 };
        let expr = p.parse_expr()?;
        if p.pos != p.tokens.len() {
            bail!("when 式に余分なトークンがあります: {src:?}");
        }
        Ok(expr)
    }

    /// 現在行に対して評価する。
    pub fn eval(&self, cols: &BTreeMap<&str, &str>, row_label: Option<&str>) -> bool {
        match self {
            WhenExpr::Or(xs) => xs.iter().any(|x| x.eval(cols, row_label)),
            WhenExpr::And(xs) => xs.iter().all(|x| x.eval(cols, row_label)),
            WhenExpr::Cond(c) => c.eval(cols, row_label),
        }
    }
}

// ---- tokenizer ----

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Or,
    And,
    LParen,
    RParen,
    Cond(String), // `{...}` の中身
}

fn tokenize(src: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            ws if ws.is_whitespace() => i += 1,
            '|' => {
                tokens.push(Token::Or);
                i += 1;
            }
            '&' => {
                tokens.push(Token::And);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            '{' => {
                let start = i + 1;
                let mut j = start;
                while j < chars.len() && chars[j] != '}' {
                    j += 1;
                }
                if j >= chars.len() {
                    bail!("`{{` に対応する `}}` がありません");
                }
                let body: String = chars[start..j].iter().collect();
                tokens.push(Token::Cond(body));
                i = j + 1;
            }
            other => bail!("when 式に不正な文字: {other:?}"),
        }
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn parse_expr(&mut self) -> Result<WhenExpr> {
        let mut terms = vec![self.parse_term()?];
        while matches!(self.peek(), Some(Token::Or)) {
            self.pos += 1;
            terms.push(self.parse_term()?);
        }
        Ok(if terms.len() == 1 {
            terms.pop().unwrap()
        } else {
            WhenExpr::Or(terms)
        })
    }

    fn parse_term(&mut self) -> Result<WhenExpr> {
        let mut factors = vec![self.parse_factor()?];
        while matches!(self.peek(), Some(Token::And)) {
            self.pos += 1;
            factors.push(self.parse_factor()?);
        }
        Ok(if factors.len() == 1 {
            factors.pop().unwrap()
        } else {
            WhenExpr::And(factors)
        })
    }

    fn parse_factor(&mut self) -> Result<WhenExpr> {
        match self.peek() {
            Some(Token::LParen) => {
                self.pos += 1;
                let expr = self.parse_expr()?;
                match self.peek() {
                    Some(Token::RParen) => {
                        self.pos += 1;
                        Ok(expr)
                    }
                    _ => bail!("`)` が必要です"),
                }
            }
            Some(Token::Cond(body)) => {
                let body = body.clone();
                self.pos += 1;
                Ok(WhenExpr::Cond(parse_condition(&body)?))
            }
            other => bail!("条件または `(` が必要です: {other:?}"),
        }
    }
}

/// `column=processor_type, equals=ARM` のような中身をパースする。
fn parse_condition(body: &str) -> Result<Condition> {
    let mut target: Option<Target> = None;
    let mut op_val: Option<(CompareOp, CondValue)> = None;

    for pair in split_top_level(body) {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let (key, val) = pair
            .split_once('=')
            .ok_or_else(|| anyhow!("`key=value` 形式ではありません: {pair:?}"))?;
        let key = key.trim();
        let val = val.trim();
        match key {
            "column" => target = Some(Target::Column(val.to_string())),
            "row" => target = Some(Target::Row(val.to_string())),
            _ => {
                let op = CompareOp::from_key(key)
                    .ok_or_else(|| anyhow!("未知のキー/演算子: {key:?}"))?;
                let value = if op == CompareOp::In {
                    CondValue::Many(parse_list(val)?)
                } else {
                    CondValue::One(val.to_string())
                };
                op_val = Some((op, value));
            }
        }
    }

    let target = target.ok_or_else(|| anyhow!("条件に column/row がありません: {body:?}"))?;
    let (op, value) = op_val.ok_or_else(|| anyhow!("条件に演算子がありません: {body:?}"))?;
    Ok(Condition { target, op, value })
}

/// `[a, b, c]` を要素に分解（暫定: クォート非対応, issue I-WHEN-2）。
fn parse_list(val: &str) -> Result<Vec<String>> {
    let val = val.trim();
    let inner = val
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| anyhow!("`in` の値はリスト `[..]` である必要があります: {val:?}"))?;
    Ok(inner
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// カンマ区切り。ただし `[...]` の内側のカンマは分割しない。
fn split_top_level(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in body.chars() {
        match c {
            '[' => {
                depth += 1;
                cur.push(c);
            }
            ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pairs: &[(&'static str, &'static str)]) -> BTreeMap<&'static str, &'static str> {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert(*k, *v);
        }
        m
    }

    #[test]
    fn parse_single_equals() {
        let e = WhenExpr::parse("{column=processor_type, equals=ARM}").unwrap();
        assert!(e.eval(&row(&[("processor_type", "ARM")]), None));
        assert!(!e.eval(&row(&[("processor_type", "MIPS")]), None));
    }

    #[test]
    fn not_equal() {
        let e = WhenExpr::parse("{column=status, not_equal=deprecated}").unwrap();
        assert!(e.eval(&row(&[("status", "active")]), None));
        assert!(!e.eval(&row(&[("status", "deprecated")]), None));
    }

    #[test]
    fn and_or() {
        let e = WhenExpr::parse("{column=a, equals=1} & {column=b, equals=2}").unwrap();
        assert!(e.eval(&row(&[("a", "1"), ("b", "2")]), None));
        assert!(!e.eval(&row(&[("a", "1"), ("b", "9")]), None));

        let e = WhenExpr::parse("{column=t, equals=A} | {column=t, equals=B}").unwrap();
        assert!(e.eval(&row(&[("t", "A")]), None));
        assert!(e.eval(&row(&[("t", "B")]), None));
        assert!(!e.eval(&row(&[("t", "C")]), None));
    }

    #[test]
    fn precedence_and_over_or() {
        // a=1 | (b=1 & c=1) : a=1 のみでも真
        let e = WhenExpr::parse("{column=a, equals=1} | {column=b, equals=1} & {column=c, equals=1}")
            .unwrap();
        assert!(e.eval(&row(&[("a", "1"), ("b", "0"), ("c", "0")]), None));
        assert!(!e.eval(&row(&[("a", "0"), ("b", "1"), ("c", "0")]), None));
        assert!(e.eval(&row(&[("a", "0"), ("b", "1"), ("c", "1")]), None));
    }

    #[test]
    fn parens_group() {
        let e = WhenExpr::parse(
            "({column=a, equals=1} | {column=a, equals=2}) & {column=b, gt=10}",
        )
        .unwrap();
        assert!(e.eval(&row(&[("a", "1"), ("b", "11")]), None));
        assert!(!e.eval(&row(&[("a", "1"), ("b", "9")]), None));
        assert!(!e.eval(&row(&[("a", "3"), ("b", "11")]), None));
    }

    #[test]
    fn gt_lt_numeric() {
        let e = WhenExpr::parse("{column=v, gt=10}").unwrap();
        assert!(e.eval(&row(&[("v", "11")]), None));
        assert!(!e.eval(&row(&[("v", "10")]), None));
        assert!(!e.eval(&row(&[("v", "abc")]), None)); // 数値化不能 → false
    }

    #[test]
    fn in_list() {
        let e = WhenExpr::parse("{column=cores, in=[2, 4, 8]}").unwrap();
        assert!(e.eval(&row(&[("cores", "4")]), None));
        assert!(!e.eval(&row(&[("cores", "3")]), None));
    }

    #[test]
    fn matches_regex() {
        let e = WhenExpr::parse("{column=id, matches=^D[0-9]+$}").unwrap();
        assert!(e.eval(&row(&[("id", "D123")]), None));
        assert!(!e.eval(&row(&[("id", "X1")]), None));
    }

    #[test]
    fn errors() {
        assert!(WhenExpr::parse("{column=a}").is_err()); // 演算子なし
        assert!(WhenExpr::parse("{equals=A}").is_err()); // target なし
        assert!(WhenExpr::parse("{column=a, equals=1} &").is_err()); // 末尾演算子
        assert!(WhenExpr::parse("({column=a, equals=1}").is_err()); // 閉じ括弧なし
    }
}
