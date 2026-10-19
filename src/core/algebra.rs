use std::collections::HashMap;

use crate::core::Function;
use crate::core::node::Node;
use crate::core::rational::Rational;

// Guards against exploding recursion when expanding recursive user functions
const MAX_DIFF_DEPTH: usize = 128;
const MAX_INTEGRAL_DEPTH: usize = 128;
const MAX_CALL_DEPTH: usize = 128;

// Operator precedence used by the source printer
const P_ADD: u8 = 1;
const P_MUL: u8 = 2;
const P_NEG: u8 = 3;
const P_POW: u8 = 4;
const P_ATOM: u8 = 5;

// Integer powers up to this bound are multiplied out during simplification
const MAX_EXPAND_POWER: f64 = 16.0;

// --- rendering ---------------------------------------------------------

// Renders a node back to infix source text with minimal parentheses
pub fn to_source(node: &Node) -> String {
    render(node, 0)
}

fn render(node: &Node, parent: u8) -> String {
    let (text, precedence) = render_inner(node);

    if precedence < parent {
        format!("({})", text)
    } else {
        text
    }
}

fn render_inner(node: &Node) -> (String, u8) {
    match node {
        Node::IntNumber(value) => (value.to_string(), P_ATOM),
        Node::FloatNumber(value) => (render_float(*value), P_ATOM),
        Node::Variable(name) => (name.to_string(), P_ATOM),

        Node::Add(left, right) => (
            format!("{} + {}", render(left, P_ADD), render(right, P_ADD + 1)),
            P_ADD,
        ),
        Node::Sub(left, right) => (
            format!("{} - {}", render(left, P_ADD), render(right, P_ADD + 1)),
            P_ADD,
        ),
        Node::Mul(left, right) => {
            let left_text = render(left, P_MUL);
            let right_text = render(right, P_MUL + 1);

            let left_is_fraction = matches!(&**left, Node::Div(..));

            if !left_is_fraction
                && !ends_in_exponent(left)
                && mul_join_safe(&left_text, &right_text, right)
            {
                (format!("{}{}", left_text, right_text), P_MUL)
            } else {
                (format!("{}*{}", left_text, right_text), P_MUL)
            }
        }
        Node::Div(left, right) => (
            format!("{}/{}", render(left, P_MUL), render(right, P_MUL + 1)),
            P_MUL,
        ),

        // '-x*y' groups as (-x)*y and only sums change meaning without brackets
        Node::Neg(inner) => {
            let parent = match **inner {
                Node::Add(..) | Node::Sub(..) | Node::Neg(..) => P_NEG + 1,
                _ => P_MUL,
            };

            (format!("-{}", render(inner, parent)), P_NEG)
        }

        // The exponent may carry a sign, so 'x^-2' needs no brackets
        Node::Pow(base, exponent) => (
            format!("{}^{}", render(base, P_POW + 1), render(exponent, P_NEG)),
            P_POW,
        ),

        Node::Builtin(function, arguments) => (render_call(function.name(), arguments), P_ATOM),
        Node::Call(name, arguments) => (render_call(name, arguments), P_ATOM),

        Node::Sum(index, lower, upper, body) => {
            (render_reduction('∑', *index, lower, upper, body), P_ATOM)
        }
        Node::Product(index, lower, upper, body) => {
            (render_reduction('∏', *index, lower, upper, body), P_ATOM)
        }
    }
}

fn render_reduction(symbol: char, index: char, lower: &Node, upper: &Node, body: &Node) -> String {
    format!(
        "{}({}, {}, {}, {})",
        symbol,
        index,
        to_source(lower),
        to_source(upper),
        to_source(body)
    )
}

fn render_call(name: &str, arguments: &[Node]) -> String {
    let rendered: Vec<String> = arguments.iter().map(to_source).collect();

    format!("{}({})", name, rendered.join(", "))
}

// True when the rendered '*' can be dropped: the tokenizer must re-split the
// joined text into the same tokens (unknown names fall apart letter by letter)
fn mul_join_safe(left: &str, right: &str, right_node: &Node) -> bool {
    let (Some(first), Some(last)) = (right.chars().next(), left.chars().last()) else {
        return false;
    };

    // 'x(y)' would re-parse as a call if a function x is ever defined
    if first == '(' {
        return !last.is_ascii_alphabetic();
    }

    if !first.is_ascii_alphabetic() || !starts_ident(right_node) {
        return false;
    }

    // A clean cut at a digit or bracket never merges tokens
    if !last.is_ascii_alphabetic() {
        return true;
    }

    // A trailing name must survive the split again ('pi' would become p*i)
    let run: String = left
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let run = run.chars().rev().collect::<String>().to_ascii_lowercase();

    !matches!(run.as_str(), "pi" | "nan" | "inf")
}

// True when the node starts with a lone identifier that push_split restores
fn starts_ident(node: &Node) -> bool {
    match node {
        Node::Variable(_) => true,
        Node::FloatNumber(value) => *value == std::f64::consts::E,
        Node::Pow(base, _) => starts_ident(base),
        _ => false,
    }
}

fn ends_in_exponent(node: &Node) -> bool {
    match node {
        Node::Pow(..) => true,
        Node::Neg(inner) => ends_in_exponent(inner),
        Node::Mul(_, right) => ends_in_exponent(right),
        _ => false,
    }
}

fn render_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_string();
    }

    if value.is_infinite() {
        return if value < 0.0 { "-inf" } else { "inf" }.to_string();
    }

    if value == std::f64::consts::PI {
        return "pi".to_string();
    }

    if value == std::f64::consts::E {
        return "e".to_string();
    }

    if value == value.trunc() && value.abs() < 1e15 {
        return (value as i64).to_string();
    }

    format!("{}", value)
}

// --- node helpers ------------------------------------------------------

fn variable(name: char) -> Node {
    Node::Variable(name)
}

fn number(value: f64) -> Node {
    if value.fract() == 0.0 && value.is_finite() && value.abs() < 1e15 {
        Node::IntNumber(value as i64)
    } else {
        Node::FloatNumber(value)
    }
}

// Folds a constant-only expression into an exact rational, None otherwise
pub fn fold_rational(node: &Node) -> Option<Rational> {
    match node {
        Node::IntNumber(value) => Some(Rational::from_integer(*value as i128)),
        Node::FloatNumber(value) => {
            if *value == std::f64::consts::PI || *value == std::f64::consts::E {
                return None;
            }

            Rational::from_decimal(*value)
        }
        Node::Neg(inner) => Some(fold_rational(inner)?.neg()),
        Node::Add(left, right) => fold_rational(left)?.add(fold_rational(right)?),
        Node::Sub(left, right) => fold_rational(left)?.sub(fold_rational(right)?),
        Node::Mul(left, right) => fold_rational(left)?.mul(fold_rational(right)?),
        Node::Div(numerator, denominator) => {
            fold_rational(numerator)?.div(fold_rational(denominator)?)
        }
        Node::Pow(base, exponent) => {
            let base = fold_rational(base)?;
            let exponent = fold_rational(exponent)?.to_integer()?;

            base.pow(exponent)
        }
        _ => None,
    }
}

// Renders a rational as an exact node, falling back to a float when too large
pub fn rational_to_node(value: Rational) -> Node {
    let numerator = value.numerator();
    let denominator = value.denominator();

    if value.is_integer() && fits_i64(numerator) {
        return Node::IntNumber(numerator as i64);
    }

    if fits_i64(numerator) && fits_i64(denominator) {
        div(
            Node::IntNumber(numerator as i64),
            Node::IntNumber(denominator as i64),
        )
    } else {
        Node::FloatNumber(value.to_f64())
    }
}

fn fits_i64(value: i128) -> bool {
    value >= i64::MIN as i128 && value <= i64::MAX as i128
}

// An exact fraction as a node, used for coefficient literals
fn fraction(numerator: i128, denominator: i128) -> Node {
    rational_to_node(
        Rational::new(numerator, denominator).unwrap_or_else(|| Rational::from_integer(0)),
    )
}

// A term coefficient: exact whenever it came from integers or a small
// fraction, a float only for pi, e and awkward decimals
#[derive(Clone, Copy, Debug, PartialEq)]
enum Coeff {
    Rational(Rational),
    Float(f64),
}

impl Coeff {
    fn from_i128(value: i128) -> Coeff {
        Coeff::Rational(Rational::from_integer(value))
    }

    fn one() -> Coeff {
        Coeff::from_i128(1)
    }

    fn zero() -> Coeff {
        Coeff::from_i128(0)
    }

    fn is_zero(self) -> bool {
        match self {
            Coeff::Rational(rational) => rational.numerator() == 0,
            Coeff::Float(value) => value == 0.0,
        }
    }

    fn is_one(self) -> bool {
        match self {
            Coeff::Rational(rational) => rational.is_integer() && rational.numerator() == 1,
            Coeff::Float(value) => value == 1.0,
        }
    }

    fn is_negative_one(self) -> bool {
        match self {
            Coeff::Rational(rational) => rational.is_integer() && rational.numerator() == -1,
            Coeff::Float(value) => value == -1.0,
        }
    }

    fn is_negative(self) -> bool {
        match self {
            Coeff::Rational(rational) => rational.numerator() < 0,
            Coeff::Float(value) => value < 0.0,
        }
    }

    fn to_f64(self) -> f64 {
        match self {
            Coeff::Rational(rational) => rational.to_f64(),
            Coeff::Float(value) => value,
        }
    }

    fn to_node(self) -> Node {
        match self {
            Coeff::Rational(rational) => rational_to_node(rational),
            Coeff::Float(value) => number(value),
        }
    }

    fn neg(self) -> Coeff {
        match self {
            Coeff::Rational(rational) => Coeff::Rational(rational.neg()),
            Coeff::Float(value) => Coeff::Float(-value),
        }
    }

    fn add(self, other: Coeff) -> Coeff {
        match (self, other) {
            (Coeff::Rational(left), Coeff::Rational(right)) => left
                .add(right)
                .map(Coeff::Rational)
                .unwrap_or_else(|| Coeff::Float(left.to_f64() + right.to_f64())),
            _ => Coeff::Float(self.to_f64() + other.to_f64()),
        }
    }

    fn mul(self, other: Coeff) -> Coeff {
        match (self, other) {
            (Coeff::Rational(left), Coeff::Rational(right)) => left
                .mul(right)
                .map(Coeff::Rational)
                .unwrap_or_else(|| Coeff::Float(left.to_f64() * right.to_f64())),
            _ => Coeff::Float(self.to_f64() * other.to_f64()),
        }
    }

    fn div(self, other: Coeff) -> Coeff {
        match (self, other) {
            (Coeff::Rational(left), Coeff::Rational(right)) => left
                .div(right)
                .map(Coeff::Rational)
                .unwrap_or_else(|| Coeff::Float(left.to_f64() / right.to_f64())),
            _ => Coeff::Float(self.to_f64() / other.to_f64()),
        }
    }
}

// Decimals only join the exact path when they reduce to a small fraction
const MAX_COEFFICIENT: i128 = 100;

fn is_small(rational: Rational) -> bool {
    rational.numerator().unsigned_abs() as i128 <= MAX_COEFFICIENT
        && rational.denominator() <= MAX_COEFFICIENT
}

fn as_coefficient(node: &Node) -> Option<Coeff> {
    match node {
        Node::IntNumber(value) => Some(Coeff::from_i128(*value as i128)),
        Node::FloatNumber(value) => {
            if *value == std::f64::consts::PI || *value == std::f64::consts::E {
                return Some(Coeff::Float(*value));
            }

            Some(
                Rational::from_decimal(*value)
                    .filter(|rational| is_small(*rational))
                    .map(Coeff::Rational)
                    .unwrap_or(Coeff::Float(*value)),
            )
        }
        _ => fold_rational(node).map(Coeff::Rational),
    }
}

fn add(left: Node, right: Node) -> Node {
    Node::Add(Box::new(left), Box::new(right))
}

fn sub(left: Node, right: Node) -> Node {
    Node::Sub(Box::new(left), Box::new(right))
}

fn mul(left: Node, right: Node) -> Node {
    Node::Mul(Box::new(left), Box::new(right))
}

fn div(numerator: Node, denominator: Node) -> Node {
    Node::Div(Box::new(numerator), Box::new(denominator))
}

fn neg(inner: Node) -> Node {
    Node::Neg(Box::new(inner))
}

fn pow(base: Node, exponent: Node) -> Node {
    Node::Pow(Box::new(base), Box::new(exponent))
}

fn builtin(function: Function, argument: Node) -> Node {
    Node::Builtin(function, vec![argument])
}

fn ln(argument: Node) -> Node {
    builtin(Function::Ln, argument)
}

fn abs(argument: Node) -> Node {
    builtin(Function::Abs, argument)
}

fn as_number(node: &Node) -> Option<f64> {
    match node {
        Node::IntNumber(value) => Some(*value as f64),
        Node::FloatNumber(value) => Some(*value),
        _ => None,
    }
}

fn as_integer(node: &Node) -> Option<f64> {
    match node {
        Node::IntNumber(value) => Some(*value as f64),
        Node::FloatNumber(value) if value.fract() == 0.0 => Some(*value),
        _ => None,
    }
}

fn is_variable(node: &Node, name: char) -> bool {
    matches!(node, Node::Variable(other) if *other == name)
}

// True for the synthetic constant 1 used as the base of additive terms
fn is_unit(node: &Node) -> bool {
    matches!(node, Node::IntNumber(1))
}

// A term that has no variable structure: a bare number or a lone uppercase
// letter (the integration constants), both sorted to the end of a sum
fn is_constant_term(node: &Node) -> bool {
    is_unit(node) || matches!(node, Node::Variable(name) if name.is_ascii_uppercase())
}

// True when the expression references the variable
pub fn has_var(node: &Node, name: char) -> bool {
    match node {
        Node::Variable(other) => *other == name,
        Node::IntNumber(_) | Node::FloatNumber(_) => false,
        Node::Neg(inner) => has_var(inner, name),
        Node::Add(left, right)
        | Node::Sub(left, right)
        | Node::Mul(left, right)
        | Node::Div(left, right)
        | Node::Pow(left, right) => has_var(left, name) || has_var(right, name),
        Node::Builtin(_, arguments) | Node::Call(_, arguments) => {
            arguments.iter().any(|argument| has_var(argument, name))
        }
        Node::Sum(index, lower, upper, body) | Node::Product(index, lower, upper, body) => {
            *index != name && (has_var(lower, name) || has_var(upper, name) || has_var(body, name))
        }
    }
}

// Replaces variables by the given expressions (used to expand user functions)
pub fn substitute(node: &Node, replacements: &HashMap<char, Node>) -> Node {
    match node {
        Node::Variable(name) => replacements
            .get(name)
            .cloned()
            .unwrap_or(Node::Variable(*name)),
        Node::IntNumber(_) | Node::FloatNumber(_) => node.clone(),
        Node::Neg(inner) => neg(substitute(inner, replacements)),
        Node::Add(left, right) => add(
            substitute(left, replacements),
            substitute(right, replacements),
        ),
        Node::Sub(left, right) => sub(
            substitute(left, replacements),
            substitute(right, replacements),
        ),
        Node::Mul(left, right) => mul(
            substitute(left, replacements),
            substitute(right, replacements),
        ),
        Node::Div(left, right) => div(
            substitute(left, replacements),
            substitute(right, replacements),
        ),
        Node::Pow(base, exponent) => pow(
            substitute(base, replacements),
            substitute(exponent, replacements),
        ),
        Node::Builtin(function, arguments) => Node::Builtin(
            *function,
            arguments
                .iter()
                .map(|argument| substitute(argument, replacements))
                .collect(),
        ),
        Node::Call(name, arguments) => Node::Call(
            name.clone(),
            arguments
                .iter()
                .map(|argument| substitute(argument, replacements))
                .collect(),
        ),
        Node::Sum(index, lower, upper, body) => Node::Sum(
            *index,
            Box::new(substitute(lower, replacements)),
            Box::new(substitute(upper, replacements)),
            Box::new(substitute_shadowed(body, replacements, *index)),
        ),
        Node::Product(index, lower, upper, body) => Node::Product(
            *index,
            Box::new(substitute(lower, replacements)),
            Box::new(substitute(upper, replacements)),
            Box::new(substitute_shadowed(body, replacements, *index)),
        ),
    }
}

// Substitution inside a reduction body must not capture the bound index
fn substitute_shadowed(node: &Node, replacements: &HashMap<char, Node>, index: char) -> Node {
    if !replacements.contains_key(&index) {
        return substitute(node, replacements);
    }

    let mut scoped = replacements.clone();

    scoped.remove(&index);

    substitute(node, &scoped)
}

// Replaces user function calls by their substituted bodies for symbolic work
pub fn expand_calls(
    node: &Node,
    functions: &HashMap<String, (Vec<char>, Node)>,
    depth: usize,
) -> Node {
    if depth > MAX_CALL_DEPTH {
        return node.clone();
    }

    match node {
        Node::Variable(_) | Node::IntNumber(_) | Node::FloatNumber(_) => node.clone(),
        Node::Neg(inner) => Node::Neg(Box::new(expand_calls(inner, functions, depth))),
        Node::Add(left, right) => Node::Add(
            Box::new(expand_calls(left, functions, depth)),
            Box::new(expand_calls(right, functions, depth)),
        ),
        Node::Sub(left, right) => Node::Sub(
            Box::new(expand_calls(left, functions, depth)),
            Box::new(expand_calls(right, functions, depth)),
        ),
        Node::Mul(left, right) => Node::Mul(
            Box::new(expand_calls(left, functions, depth)),
            Box::new(expand_calls(right, functions, depth)),
        ),
        Node::Div(left, right) => Node::Div(
            Box::new(expand_calls(left, functions, depth)),
            Box::new(expand_calls(right, functions, depth)),
        ),
        Node::Pow(base, exponent) => Node::Pow(
            Box::new(expand_calls(base, functions, depth)),
            Box::new(expand_calls(exponent, functions, depth)),
        ),
        Node::Builtin(function, arguments) => Node::Builtin(
            *function,
            arguments
                .iter()
                .map(|argument| expand_calls(argument, functions, depth))
                .collect(),
        ),
        Node::Call(name, arguments) => {
            let arguments: Vec<Node> = arguments
                .iter()
                .map(|argument| expand_calls(argument, functions, depth))
                .collect();

            match functions.get(name) {
                Some((params, body)) if params.len() == arguments.len() => {
                    let replacements: HashMap<char, Node> =
                        params.iter().copied().zip(arguments).collect();
                    let expanded = substitute(body, &replacements);

                    expand_calls(&expanded, functions, depth + 1)
                }
                _ => Node::Call(name.clone(), arguments),
            }
        }
        Node::Sum(index, lower, upper, body) => Node::Sum(
            *index,
            Box::new(expand_calls(lower, functions, depth)),
            Box::new(expand_calls(upper, functions, depth)),
            Box::new(expand_calls(body, functions, depth)),
        ),
        Node::Product(index, lower, upper, body) => Node::Product(
            *index,
            Box::new(expand_calls(lower, functions, depth)),
            Box::new(expand_calls(upper, functions, depth)),
            Box::new(expand_calls(body, functions, depth)),
        ),
    }
}

// --- simplification ----------------------------------------------------

// Applies algebraic rewrite rules to make expressions more compact
pub fn simplify(node: &Node) -> Node {
    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) | Node::Variable(_) => node.clone(),
        Node::Neg(inner) => simplify_neg(simplify(inner)),
        Node::Add(left, right) => simplify_sum(&simplify(left), &simplify(right), false),
        Node::Sub(left, right) => simplify_sum(&simplify(left), &simplify(right), true),
        Node::Mul(left, right) => simplify_product(&[simplify(left), simplify(right)]),
        Node::Div(numerator, denominator) => {
            simplify_div(simplify(numerator), simplify(denominator))
        }
        Node::Pow(base, exponent) => simplify_power(simplify(base), simplify(exponent)),
        Node::Builtin(function, arguments) => {
            let simplified: Vec<Node> = arguments.iter().map(simplify).collect();

            simplify_builtin(*function, simplified)
        }
        Node::Call(name, arguments) => {
            Node::Call(name.clone(), arguments.iter().map(simplify).collect())
        }
        Node::Sum(index, lower, upper, body) => Node::Sum(
            *index,
            Box::new(simplify(lower)),
            Box::new(simplify(upper)),
            Box::new(simplify(body)),
        ),
        Node::Product(index, lower, upper, body) => Node::Product(
            *index,
            Box::new(simplify(lower)),
            Box::new(simplify(upper)),
            Box::new(simplify(body)),
        ),
    }
}

fn simplify_neg(inner: Node) -> Node {
    match inner {
        Node::IntNumber(value) => number(-(value as f64)),
        Node::FloatNumber(value) => number(-value),
        Node::Neg(deeper) => *deeper,
        Node::Add(left, right) => simplify_sum(&simplify_neg(*left), &simplify_neg(*right), false),
        Node::Sub(left, right) => simplify_sum(&simplify_neg(*left), &simplify_neg(*right), true),
        other => neg(other),
    }
}

// Collects additive terms as (coefficient, base) pairs, honouring the sign
fn collect_terms(node: &Node, sign: Coeff, out: &mut Vec<(Coeff, Node)>) {
    match node {
        Node::Add(left, right) => {
            collect_terms(left, sign, out);
            collect_terms(right, sign, out);
        }
        Node::Sub(left, right) => {
            collect_terms(left, sign, out);
            collect_terms(right, sign.neg(), out);
        }
        Node::Neg(inner) => collect_terms(inner, sign.neg(), out),
        _ => {
            let (coefficient, base) = split_coefficient(node);

            out.push((sign.mul(coefficient), base));
        }
    }
}

// Splits a multiplicative term into its numeric coefficient and symbolic base
fn split_coefficient(node: &Node) -> (Coeff, Node) {
    match node {
        Node::Neg(inner) => {
            let (coefficient, base) = split_coefficient(inner);

            (coefficient.neg(), base)
        }
        Node::Mul(_, _) => {
            let mut factors = Vec::new();

            flatten_factors(node, &mut factors);

            let mut coefficient = Coeff::one();
            let mut symbolic = Vec::new();

            for factor in factors {
                match as_coefficient(&factor) {
                    Some(value) => coefficient = coefficient.mul(value),
                    None => symbolic.push(factor),
                }
            }

            (coefficient, combine_factors(symbolic))
        }
        _ => match as_coefficient(node) {
            Some(value) => (value, number(1.0)),
            None => (Coeff::one(), node.clone()),
        },
    }
}

// Collects flat multiplication factors, folding unary minuses into a -1 factor
fn flatten_factors(node: &Node, out: &mut Vec<Node>) {
    match node {
        Node::Mul(left, right) => {
            flatten_factors(left, out);
            flatten_factors(right, out);
        }
        Node::Neg(inner) => {
            out.push(number(-1.0));
            flatten_factors(inner, out);
        }
        other => out.push(other.clone()),
    }
}
fn build_product(factors: Vec<Node>) -> Node {
    let mut iter = factors.into_iter();

    let Some(first) = iter.next() else {
        return number(1.0);
    };

    let mut result = first;

    for factor in iter {
        result = mul(result, factor);
    }

    result
}

// Groups equal factors into powers and multiplies the rest: x*x -> x^2
fn combine_factors(factors: Vec<Node>) -> Node {
    let (coefficient, bases) = decompose_product(factors);

    let product = bases_to_node(bases);

    if coefficient.is_one() {
        product
    } else {
        mul(coefficient.to_node(), product)
    }
}

// Splits a flat factor list into a numeric coefficient and (base, exponent) pairs
fn decompose_product(nodes: Vec<Node>) -> (Coeff, Vec<(String, Node, f64)>) {
    let mut factors = Vec::new();

    for node in nodes {
        flatten_factors(&node, &mut factors);
    }

    let mut coefficient = Coeff::one();
    let mut bases: Vec<(String, Node, f64)> = Vec::new();

    for factor in factors {
        if let Some(value) = as_coefficient(&factor) {
            coefficient = coefficient.mul(value);
            continue;
        }

        let (base, exponent) = match factor {
            Node::Pow(inner, exponent) => match as_number(&exponent) {
                Some(value) => ((*inner).clone(), value),
                None => (Node::Pow(inner, exponent), 1.0),
            },
            other => (other, 1.0),
        };

        let key = to_source(&base);

        if let Some(entry) = bases.iter_mut().find(|(other, _, _)| *other == key) {
            entry.2 += exponent;
        } else {
            bases.push((key, base, exponent));
        }
    }

    (coefficient, bases)
}

// Rebuilds a product from (base, exponent) pairs, dropping zero exponents
fn bases_to_node(mut bases: Vec<(String, Node, f64)>) -> Node {
    bases.retain(|(_, _, exponent)| *exponent != 0.0);
    bases.sort_by(|a, b| a.0.cmp(&b.0));

    let factors = bases
        .into_iter()
        .map(|(_, base, exponent)| {
            if exponent == 1.0 {
                base
            } else {
                simplify_power(base, number(exponent))
            }
        })
        .collect();

    build_product(factors)
}

// Distributes products over sums so like terms across factorised forms can cancel
fn expand(node: &Node) -> Node {
    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) | Node::Variable(_) => node.clone(),
        Node::Neg(inner) => neg(expand(inner)),
        Node::Add(left, right) => add(expand(left), expand(right)),
        Node::Sub(left, right) => sub(expand(left), expand(right)),
        Node::Mul(left, right) => distribute(expand(left), expand(right)),
        Node::Div(numerator, denominator) => div(expand(numerator), expand(denominator)),
        Node::Pow(base, exponent) => {
            let base = expand(base);

            match as_integer(exponent) {
                Some(power) if (1.0..=MAX_EXPAND_POWER).contains(&power) => {
                    let mut result = base.clone();

                    for _ in 1..power as usize {
                        result = distribute(result, base.clone());
                    }

                    result
                }
                _ => pow(base, (**exponent).clone()),
            }
        }
        // Function arguments stay as they are: expanding a power inside a call
        // like sin(x^2) would rewrite it to sin(x x)
        Node::Builtin(..) | Node::Call(..) => node.clone(),
        Node::Sum(index, lower, upper, body) => Node::Sum(
            *index,
            Box::new(expand(lower)),
            Box::new(expand(upper)),
            Box::new(expand(body)),
        ),
        Node::Product(index, lower, upper, body) => Node::Product(
            *index,
            Box::new(expand(lower)),
            Box::new(expand(upper)),
            Box::new(expand(body)),
        ),
    }
}

// Multiplies two expanded operands, distributing over any remaining sums
fn distribute(left: Node, right: Node) -> Node {
    match left {
        Node::Add(l, r) => add(distribute(*l, right.clone()), distribute(*r, right)),
        Node::Sub(l, r) => sub(distribute(*l, right.clone()), distribute(*r, right)),
        Node::Neg(inner) => neg(distribute(*inner, right)),
        other => match right {
            Node::Add(l, r) => add(distribute(other.clone(), *l), distribute(other, *r)),
            Node::Sub(l, r) => sub(distribute(other.clone(), *l), distribute(other, *r)),
            Node::Neg(inner) => neg(distribute(other, *inner)),
            other_right => mul(other, other_right),
        },
    }
}

fn simplify_sum(left: &Node, right: &Node, subtract: bool) -> Node {
    let left = expand(left);
    let right = expand(right);
    let mut terms: Vec<(Coeff, Node)> = Vec::new();

    collect_terms(&left, Coeff::one(), &mut terms);
    collect_terms(
        &right,
        if subtract {
            Coeff::one().neg()
        } else {
            Coeff::one()
        },
        &mut terms,
    );

    let mut grouped: Vec<(String, Coeff, Node)> = Vec::new();

    for (coefficient, base) in terms {
        let key = to_source(&base);

        if let Some(entry) = grouped.iter_mut().find(|(other, _, _)| *other == key) {
            entry.1 = entry.1.add(coefficient);
        } else {
            grouped.push((key, coefficient, base));
        }
    }

    grouped.retain(|(_, coefficient, _)| !coefficient.is_zero());

    // Like a classical polynomial: constants last, descending degree in x
    grouped.sort_by(|a, b| {
        let a_constant = is_constant_term(&a.2);
        let b_constant = is_constant_term(&b.2);

        a_constant
            .cmp(&b_constant)
            .then_with(|| {
                degree_in(&b.2, 'x')
                    .partial_cmp(&degree_in(&a.2, 'x'))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                total_degree(&b.2)
                    .partial_cmp(&total_degree(&a.2))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.0.cmp(&b.0))
    });

    build_sum(grouped)
}

// Power of the variable inside a term, counting only explicit factors
fn degree_in(node: &Node, name: char) -> f64 {
    match node {
        Node::Variable(other) if *other == name => 1.0,
        Node::Variable(_) | Node::IntNumber(_) | Node::FloatNumber(_) => 0.0,
        Node::Neg(inner) => degree_in(inner, name),
        Node::Mul(left, right) => degree_in(left, name) + degree_in(right, name),
        Node::Div(numerator, denominator) => {
            degree_in(numerator, name) - degree_in(denominator, name)
        }
        Node::Pow(base, exponent) => {
            if is_variable(base, name) {
                as_number(exponent).unwrap_or(1.0)
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

// Total degree over all variables, used as the secondary sort key
fn total_degree(node: &Node) -> f64 {
    match node {
        Node::Variable(_) => 1.0,
        Node::IntNumber(_) | Node::FloatNumber(_) => 0.0,
        Node::Neg(inner) => total_degree(inner),
        Node::Mul(left, right) => total_degree(left) + total_degree(right),
        Node::Div(numerator, denominator) => total_degree(numerator) - total_degree(denominator),
        Node::Pow(base, exponent) => match as_number(exponent) {
            Some(value) => total_degree(base) * value,
            None => total_degree(base),
        },
        _ => 0.0,
    }
}

fn build_sum(grouped: Vec<(String, Coeff, Node)>) -> Node {
    let mut result: Option<Node> = None;

    for (_, coefficient, base) in grouped {
        if !coefficient.is_negative() {
            let term = make_term(coefficient, base);

            result = Some(match result {
                None => term,
                Some(accumulator) => add(accumulator, term),
            });
        } else {
            let term = make_term(coefficient.neg(), base);

            result = Some(match result {
                None => neg(term),
                Some(accumulator) => sub(accumulator, term),
            });
        }
    }

    result.unwrap_or_else(|| number(0.0))
}

fn make_term(coefficient: Coeff, base: Node) -> Node {
    if is_unit(&base) {
        return coefficient.to_node();
    }

    if coefficient.is_one() {
        return base;
    }

    let mut factors = vec![coefficient.to_node()];

    flatten_factors(&base, &mut factors);

    build_product(factors)
}

fn simplify_product(nodes: &[Node]) -> Node {
    let mut numerator = Vec::new();
    let mut denominator = Vec::new();

    for node in nodes {
        collect_ratio(node, &mut numerator, &mut denominator);
    }

    simplify_ratio(numerator, denominator)
}

fn simplify_div(numerator: Node, denominator: Node) -> Node {
    simplify_ratio(vec![numerator], vec![denominator])
}

// Walks a product, routing division factors onto the right side of the ratio
fn collect_ratio(node: &Node, numerator: &mut Vec<Node>, denominator: &mut Vec<Node>) {
    match node {
        Node::Mul(left, right) => {
            collect_ratio(left, numerator, denominator);
            collect_ratio(right, numerator, denominator);
        }
        Node::Div(top, bottom) => {
            collect_ratio(top, numerator, denominator);
            collect_ratio(bottom, denominator, numerator);
        }
        Node::Neg(inner) => {
            numerator.push(number(-1.0));
            collect_ratio(inner, numerator, denominator);
        }
        other => numerator.push(other.clone()),
    }
}

// Reduces a ratio of factor lists to a single expression
fn simplify_ratio(numerator: Vec<Node>, denominator: Vec<Node>) -> Node {
    let (top_coefficient, top_bases) = decompose_product(numerator);
    let (bottom_coefficient, mut bottom_bases) = decompose_product(denominator);

    if top_coefficient.is_zero() && !bottom_coefficient.is_zero() {
        return number(0.0);
    }

    let mut remaining: Vec<(String, Node, f64)> = Vec::new();

    for (key, base, exponent) in top_bases {
        match bottom_bases.iter().position(|(other, _, _)| *other == key) {
            Some(index) => {
                let common = exponent.min(bottom_bases[index].2);
                let top_left = exponent - common;
                let bottom_left = bottom_bases[index].2 - common;

                if top_left != 0.0 {
                    remaining.push((key, base, top_left));
                }

                if bottom_left == 0.0 {
                    bottom_bases.remove(index);
                } else {
                    bottom_bases[index].2 = bottom_left;
                }
            }
            None => remaining.push((key, base, exponent)),
        }
    }

    if bottom_bases.is_empty() {
        if bottom_coefficient.is_zero() {
            return number(top_coefficient.to_f64() / 0.0);
        }

        let base = bases_to_node(remaining);

        // A fraction coefficient reads well in front of a single factor
        // ('1/3*x^3'), but a sum keeps its division ('(y - 3)/2')
        if matches!(base, Node::Add(..) | Node::Sub(..)) && !bottom_coefficient.is_one() {
            return div(
                with_coefficient(top_coefficient, base),
                bottom_coefficient.to_node(),
            );
        }

        return with_coefficient(top_coefficient.div(bottom_coefficient), base);
    }

    let (top_coefficient, bottom_coefficient) =
        reduce_coefficients(top_coefficient, bottom_coefficient);

    let top = with_coefficient(top_coefficient, bases_to_node(remaining));
    let bottom = with_coefficient(bottom_coefficient, bases_to_node(bottom_bases));

    // An exact polynomial division reads far better than the fraction
    if let Some(quotient) = polynomial_quotient(&top, &bottom) {
        return quotient;
    }

    div(top, bottom)
}

// Exact univariate polynomial division: Some(quotient) when the remainder is zero
fn polynomial_quotient(top: &Node, bottom: &Node) -> Option<Node> {
    let name = single_variable_in(top).filter(|name| single_variable_in(bottom) == Some(*name))?;
    let numerator = to_polynomial(top, name)?;
    let denominator = to_polynomial(bottom, name)?;

    let (quotient, remainder) = poly_divmod(&numerator, &denominator)?;

    if !remainder.is_empty() {
        return None;
    }

    Some(simplify(&poly_to_node(&quotient, name)))
}

// The only variable referenced by the node, if there is exactly one
fn single_variable_in(node: &Node) -> Option<char> {
    let mut names = Vec::new();

    collect_variable_names(node, &mut names);
    names.sort();
    names.dedup();

    match names.as_slice() {
        [name] => Some(*name),
        _ => None,
    }
}

fn collect_variable_names(node: &Node, out: &mut Vec<char>) {
    match node {
        Node::Variable(name) => out.push(*name),
        Node::Neg(inner) => collect_variable_names(inner, out),
        Node::Add(left, right) | Node::Sub(left, right) | Node::Mul(left, right) => {
            collect_variable_names(left, out);
            collect_variable_names(right, out);
        }
        Node::Div(numerator, denominator) => {
            collect_variable_names(numerator, out);
            collect_variable_names(denominator, out);
        }
        Node::Pow(base, exponent) => {
            collect_variable_names(base, out);
            collect_variable_names(exponent, out);
        }
        Node::Builtin(_, arguments) => {
            for argument in arguments {
                collect_variable_names(argument, out);
            }
        }
        Node::Call(_, arguments) => {
            for argument in arguments {
                collect_variable_names(argument, out);
            }
        }
        Node::Sum(index, lower, upper, body) => {
            out.push(*index);
            collect_variable_names(lower, out);
            collect_variable_names(upper, out);
            collect_variable_names(body, out);
        }
        Node::Product(index, lower, upper, body) => {
            out.push(*index);
            collect_variable_names(lower, out);
            collect_variable_names(upper, out);
            collect_variable_names(body, out);
        }
        Node::IntNumber(_) | Node::FloatNumber(_) => {}
    }
}

// Coefficients of an expanded univariate polynomial, lowest degree first
fn to_polynomial(node: &Node, name: char) -> Option<Vec<Coeff>> {
    match node {
        Node::Variable(other) if *other == name => Some(vec![Coeff::zero(), Coeff::one()]),
        Node::Variable(_) => None,
        Node::IntNumber(_) | Node::FloatNumber(_) => Some(vec![as_coefficient(node)?]),
        Node::Neg(inner) => {
            let inner = to_polynomial(inner, name)?;

            Some(inner.iter().map(|coefficient| coefficient.neg()).collect())
        }
        Node::Add(left, right) => {
            let left = to_polynomial(left, name)?;
            let right = to_polynomial(right, name)?;

            Some(poly_add(&left, &right))
        }
        Node::Sub(left, right) => {
            let left = to_polynomial(left, name)?;
            let right = to_polynomial(right, name)?;

            Some(poly_add(&left, &poly_scale(&right, Coeff::one().neg())))
        }
        Node::Mul(left, right) => {
            let left = to_polynomial(left, name)?;
            let right = to_polynomial(right, name)?;

            Some(poly_mul(&left, &right))
        }
        Node::Pow(base, exponent) => {
            let power = as_integer(exponent)?;

            if !(0.0..=32.0).contains(&power) {
                return None;
            }

            let base = to_polynomial(base, name)?;

            Some(poly_pow(&base, power as usize))
        }
        _ => None,
    }
}

fn poly_trim(poly: &[Coeff]) -> Vec<Coeff> {
    let mut trimmed = poly.to_vec();

    while trimmed
        .last()
        .is_some_and(|coefficient| coefficient.is_zero())
    {
        trimmed.pop();
    }

    trimmed
}

fn poly_add(left: &[Coeff], right: &[Coeff]) -> Vec<Coeff> {
    let length = left.len().max(right.len());
    let mut result = Vec::with_capacity(length);

    for index in 0..length {
        let a = left.get(index).copied().unwrap_or_else(Coeff::zero);
        let b = right.get(index).copied().unwrap_or_else(Coeff::zero);

        result.push(a.add(b));
    }

    poly_trim(&result)
}

fn poly_scale(poly: &[Coeff], factor: Coeff) -> Vec<Coeff> {
    poly.iter()
        .map(|coefficient| coefficient.mul(factor))
        .collect()
}

fn poly_mul(left: &[Coeff], right: &[Coeff]) -> Vec<Coeff> {
    if left.is_empty() || right.is_empty() {
        return Vec::new();
    }

    let mut result = vec![Coeff::zero(); left.len() + right.len() - 1];

    for (i, a) in left.iter().enumerate() {
        for (j, b) in right.iter().enumerate() {
            result[i + j] = result[i + j].add(a.mul(*b));
        }
    }

    poly_trim(&result)
}

fn poly_pow(base: &[Coeff], power: usize) -> Vec<Coeff> {
    let mut result = vec![Coeff::one()];

    for _ in 0..power {
        result = poly_mul(&result, base);
    }

    result
}

fn poly_divmod(numerator: &[Coeff], denominator: &[Coeff]) -> Option<(Vec<Coeff>, Vec<Coeff>)> {
    let numerator = poly_trim(numerator);
    let denominator = poly_trim(denominator);

    if denominator.is_empty() {
        return None;
    }

    if numerator.len() < denominator.len() {
        return Some((Vec::new(), numerator));
    }

    let lead = denominator[denominator.len() - 1];
    let mut quotient = vec![Coeff::zero(); numerator.len() - denominator.len() + 1];
    let mut remainder = numerator;

    while remainder.len() >= denominator.len() {
        let shift = remainder.len() - denominator.len();
        let factor = remainder[remainder.len() - 1].div(lead);

        quotient[shift] = factor;

        for (index, coefficient) in denominator.iter().enumerate() {
            let term = factor.mul(*coefficient);
            remainder[shift + index] = remainder[shift + index].add(term.neg());
        }

        remainder = poly_trim(&remainder);

        if remainder.len() < denominator.len() {
            break;
        }
    }

    Some((poly_trim(&quotient), remainder))
}

fn poly_to_node(poly: &[Coeff], name: char) -> Node {
    let mut grouped = Vec::new();

    for degree in (0..poly.len()).rev() {
        let coefficient = poly[degree];

        if coefficient.is_zero() {
            continue;
        }

        let base = match degree {
            0 => number(1.0),
            1 => variable(name),
            _ => pow(variable(name), number(degree as f64)),
        };

        grouped.push((to_source(&base), coefficient, base));
    }

    build_sum(grouped)
}

// Attaches a numeric coefficient to a node, keeping readable forms
fn with_coefficient(coefficient: Coeff, node: Node) -> Node {
    if is_unit(&node) {
        return coefficient.to_node();
    }

    if coefficient.is_one() {
        return node;
    }

    if coefficient.is_negative_one() {
        return neg(node);
    }

    mul(coefficient.to_node(), node)
}

// Splits an exact coefficient ratio into its integer numerator and denominator
fn reduce_coefficients(top: Coeff, bottom: Coeff) -> (Coeff, Coeff) {
    if top.is_zero() || bottom.is_zero() {
        return (top, bottom);
    }

    match top.div(bottom) {
        Coeff::Rational(ratio) => (
            Coeff::from_i128(ratio.numerator()),
            Coeff::from_i128(ratio.denominator()),
        ),
        Coeff::Float(_) => (top, bottom),
    }
}

fn simplify_power(base: Node, exponent: Node) -> Node {
    if let Some(power) = as_number(&exponent) {
        if power == 0.0 {
            return number(1.0);
        }

        if power == 1.0 {
            return base;
        }

        if let Some(value) = as_number(&base) {
            if value == 0.0 && power > 0.0 {
                return number(0.0);
            }

            if value == 1.0 {
                return number(1.0);
            }

            let result = value.powf(power);

            if result.is_finite() {
                return number(result);
            }
        }

        // Nested powers: (a^b)^c collapses to a^(b*c)
        if let Node::Pow(inner, inner_exponent) = &base {
            let folded = simplify_product(&[(**inner_exponent).clone(), number(power)]);

            return pow((**inner).clone(), folded);
        }

        // Roots cancel against whole powers: sqrt(u)^2 and cbrt(u)^3
        if let Node::Builtin(function, arguments) = &base
            && let [argument] = arguments.as_slice()
        {
            match function {
                Function::Sqrt if power == 2.0 => return argument.clone(),
                Function::Cbrt if power == 3.0 => return argument.clone(),
                _ => {}
            }
        }
    }

    pow(base, exponent)
}

fn simplify_builtin(function: Function, arguments: Vec<Node>) -> Node {
    // Exact values worth keeping symbolic
    if let [argument] = arguments.as_slice()
        && let Some(value) = as_number(argument)
    {
        if function == Function::Ln && value == std::f64::consts::E {
            return number(1.0);
        }

        if function == Function::Log && value == 10.0 {
            return number(1.0);
        }

        if function == Function::Log2 && value == 2.0 {
            return number(1.0);
        }
    }

    // Fold only clearly integer results so exact forms like sqrt(2) are kept
    let values: Option<Vec<f64>> = arguments.iter().map(as_integer).collect();

    if let Some(values) = values {
        let result = function.eval(&values);

        if result.is_finite() && result.fract() == 0.0 {
            return number(result);
        }
    }

    Node::Builtin(function, arguments)
}

// --- differentiation ---------------------------------------------------

// Symbolic derivative with respect to one variable
pub fn differentiate(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
) -> Option<Node> {
    diff(node, name, functions, 0)
}

fn diff(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
    depth: usize,
) -> Option<Node> {
    if depth > MAX_DIFF_DEPTH {
        return None;
    }

    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) => Some(number(0.0)),
        Node::Variable(other) => Some(number(if *other == name { 1.0 } else { 0.0 })),

        Node::Neg(inner) => Some(neg(diff(inner, name, functions, depth)?)),

        Node::Add(left, right) => Some(add(
            diff(left, name, functions, depth)?,
            diff(right, name, functions, depth)?,
        )),
        Node::Sub(left, right) => Some(sub(
            diff(left, name, functions, depth)?,
            diff(right, name, functions, depth)?,
        )),

        Node::Mul(left, right) => {
            let left_derivative = diff(left, name, functions, depth)?;
            let right_derivative = diff(right, name, functions, depth)?;

            Some(add(
                mul(left_derivative, (**right).clone()),
                mul((**left).clone(), right_derivative),
            ))
        }

        Node::Div(numerator, denominator) => {
            let top = diff(numerator, name, functions, depth)?;
            let bottom = diff(denominator, name, functions, depth)?;

            Some(div(
                sub(
                    mul(top, (**denominator).clone()),
                    mul((**numerator).clone(), bottom),
                ),
                pow((**denominator).clone(), number(2.0)),
            ))
        }

        Node::Pow(base, exponent) => {
            let base_derivative = diff(base, name, functions, depth)?;
            let exponent_derivative = diff(exponent, name, functions, depth)?;

            if !has_var(exponent, name) {
                // Power rule: n * base^(n-1) * base'
                Some(mul(
                    mul(
                        (**exponent).clone(),
                        pow((**base).clone(), sub((**exponent).clone(), number(1.0))),
                    ),
                    base_derivative,
                ))
            } else {
                // General rule: base^exponent * (exponent' * ln(base) + exponent * base'/base)
                let term = add(
                    mul(exponent_derivative, ln((**base).clone())),
                    mul((**exponent).clone(), div(base_derivative, (**base).clone())),
                );

                Some(mul(pow((**base).clone(), (**exponent).clone()), term))
            }
        }

        Node::Builtin(function, arguments) => {
            builtin_derivative(*function, arguments, name, functions, depth)
        }

        Node::Call(function, arguments) => {
            let (parameters, body) = functions.get(function)?;

            let mut replacements = HashMap::new();

            for (parameter, argument) in parameters.iter().zip(arguments) {
                replacements.insert(*parameter, argument.clone());
            }

            let expanded = substitute(body, &replacements);

            diff(&expanded, name, functions, depth + 1)
        }

        Node::Sum(index, lower, upper, body) => {
            // The bound index is a dummy, so it never contributes a derivative
            if *index == name {
                return Some(number(0.0));
            }

            if has_var(lower, name) || has_var(upper, name) {
                return None;
            }

            Some(Node::Sum(
                *index,
                lower.clone(),
                upper.clone(),
                Box::new(diff(body, name, functions, depth)?),
            ))
        }

        Node::Product(index, lower, upper, body) => {
            if *index == name {
                return Some(number(0.0));
            }

            if has_var(lower, name) || has_var(upper, name) || has_var(body, name) {
                return None;
            }

            Some(number(0.0))
        }
    }
}

fn builtin_derivative(
    function: Function,
    arguments: &[Node],
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
    depth: usize,
) -> Option<Node> {
    match function {
        // Constant almost everywhere
        Function::Floor | Function::Ceil | Function::Round | Function::Trunc | Function::Sign => {
            Some(number(0.0))
        }

        // Piecewise / not differentiable in closed form
        Function::Min | Function::Max | Function::Mod => None,

        Function::Atan2 => {
            let [y, x] = arguments else {
                return None;
            };

            let dy = diff(y, name, functions, depth)?;
            let dx = diff(x, name, functions, depth)?;

            // (x * y' - y * x') / (x^2 + y^2)
            let numerator = sub(mul(x.clone(), dy), mul(y.clone(), dx));
            let denominator = add(pow(x.clone(), number(2.0)), pow(y.clone(), number(2.0)));

            Some(div(numerator, denominator))
        }

        Function::Log => match arguments {
            [value] => {
                let inner = diff(value, name, functions, depth)?;

                Some(div(inner, mul(value.clone(), ln(number(10.0)))))
            }
            [value, base] => {
                let inner = diff(value, name, functions, depth)?;

                if !has_var(base, name) {
                    return Some(div(inner, mul(value.clone(), ln(base.clone()))));
                }

                let base_derivative = diff(base, name, functions, depth)?;

                // d(ln value / ln base)
                let numerator = sub(
                    mul(div(inner, value.clone()), ln(base.clone())),
                    mul(ln(value.clone()), div(base_derivative, base.clone())),
                );
                let denominator = pow(ln(base.clone()), number(2.0));

                Some(div(numerator, denominator))
            }
            _ => None,
        },

        _ => {
            let [argument] = arguments else {
                return None;
            };

            let inner = diff(argument, name, functions, depth)?;
            let outer = outer_derivative(function, argument.clone())?;

            Some(mul(outer, inner))
        }
    }
}

// Derivative of a built-in in terms of its own argument (chain rule factor)
fn outer_derivative(function: Function, argument: Node) -> Option<Node> {
    let derivative = match function {
        Function::Sqrt => div(
            number(1.0),
            mul(number(2.0), builtin(Function::Sqrt, argument)),
        ),
        Function::Cbrt => div(
            number(1.0),
            mul(
                number(3.0),
                pow(builtin(Function::Cbrt, argument.clone()), number(2.0)),
            ),
        ),
        Function::Abs => builtin(Function::Sign, argument),
        Function::Exp => builtin(Function::Exp, argument),
        Function::Ln => div(number(1.0), argument),
        Function::Log2 => div(number(1.0), mul(argument, ln(number(2.0)))),
        Function::Sin => builtin(Function::Cos, argument),
        Function::Cos => neg(builtin(Function::Sin, argument)),
        Function::Tan => div(
            number(1.0),
            pow(builtin(Function::Cos, argument), number(2.0)),
        ),
        Function::Asin => div(
            number(1.0),
            builtin(Function::Sqrt, sub(number(1.0), pow(argument, number(2.0)))),
        ),
        Function::Acos => neg(div(
            number(1.0),
            builtin(
                Function::Sqrt,
                sub(number(1.0), pow(argument.clone(), number(2.0))),
            ),
        )),
        Function::Atan => div(number(1.0), add(number(1.0), pow(argument, number(2.0)))),
        Function::Sinh => builtin(Function::Cosh, argument),
        Function::Cosh => builtin(Function::Sinh, argument),
        Function::Tanh => sub(
            number(1.0),
            pow(builtin(Function::Tanh, argument), number(2.0)),
        ),
        Function::Asinh => div(
            number(1.0),
            builtin(Function::Sqrt, add(pow(argument, number(2.0)), number(1.0))),
        ),
        Function::Acosh => div(
            number(1.0),
            builtin(Function::Sqrt, sub(pow(argument, number(2.0)), number(1.0))),
        ),
        Function::Atanh => div(number(1.0), sub(number(1.0), pow(argument, number(2.0)))),
        _ => return None,
    };

    Some(derivative)
}

// --- integration -------------------------------------------------------

// Symbolic antiderivative with respect to one variable (None when unknown)
pub fn integrate(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
) -> Option<Node> {
    integral(node, name, functions, 0)
}

fn integral(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
    depth: usize,
) -> Option<Node> {
    if depth > MAX_INTEGRAL_DEPTH {
        return None;
    }

    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) => Some(mul(node.clone(), variable(name))),

        Node::Variable(other) => {
            if *other == name {
                power_antiderivative(variable(name), 1.0)
            } else {
                Some(mul(node.clone(), variable(name)))
            }
        }

        Node::Neg(inner) => Some(neg(integral(inner, name, functions, depth)?)),

        Node::Add(left, right) => Some(add(
            integral(left, name, functions, depth)?,
            integral(right, name, functions, depth)?,
        )),
        Node::Sub(left, right) => Some(sub(
            integral(left, name, functions, depth)?,
            integral(right, name, functions, depth)?,
        )),

        Node::Mul(left, right) => {
            if !has_var(left, name) {
                Some(mul(
                    (**left).clone(),
                    integral(right, name, functions, depth)?,
                ))
            } else if !has_var(right, name) {
                Some(mul(
                    (**right).clone(),
                    integral(left, name, functions, depth)?,
                ))
            } else if let Some(substituted) = integrate_substitution(node, name, functions) {
                Some(substituted)
            } else {
                integrate_by_parts(node, name, functions, depth)
            }
        }

        Node::Div(numerator, denominator) => {
            if !has_var(denominator, name) {
                Some(mul(
                    div(number(1.0), (**denominator).clone()),
                    integral(numerator, name, functions, depth)?,
                ))
            } else if !has_var(numerator, name) {
                if is_variable(denominator, name) {
                    Some(mul((**numerator).clone(), ln(abs(variable(name)))))
                } else if let Node::Pow(base, exponent) = &**denominator {
                    if has_var(exponent, name) {
                        None
                    } else {
                        let power = -as_number(exponent)?;

                        Some(mul(
                            (**numerator).clone(),
                            integrate_linear_power(base, power, name)?,
                        ))
                    }
                } else if let Some(reciprocal) = integrate_quadratic_reciprocal(denominator, name) {
                    Some(mul((**numerator).clone(), reciprocal))
                } else if let Some(reciprocal) = integrate_exp_reciprocal(denominator, name) {
                    Some(mul((**numerator).clone(), reciprocal))
                } else {
                    // a / (k*x + b) -> (a / k) * ln|k*x + b|
                    let (slope, _) = linear_coefficients(denominator, name)?;

                    Some(mul(
                        div((**numerator).clone(), number(slope)),
                        ln(abs((**denominator).clone())),
                    ))
                }
            } else {
                // f'(x) / f(x) -> ln|f(x)|
                let derivative = diff(denominator, name, functions, 0);

                match derivative {
                    Some(derivative) => {
                        let ratio = simplify(&div((**numerator).clone(), derivative));

                        if has_var(&ratio, name) {
                            None
                        } else {
                            Some(mul(ratio, ln(abs((**denominator).clone()))))
                        }
                    }
                    None => None,
                }
            }
        }

        Node::Pow(base, exponent) => {
            if is_variable(base, name) && !has_var(exponent, name) {
                power_antiderivative(variable(name), as_number(exponent)?)
            } else if !has_var(base, name) && !has_var(exponent, name) {
                Some(mul(node.clone(), variable(name)))
            } else if !has_var(base, name) {
                // a^u with linear exponent u
                let constant = as_number(base)?;
                let (slope, _) = linear_coefficients(exponent, name)?;

                Some(div(
                    pow((**base).clone(), (**exponent).clone()),
                    mul(number(constant.ln()), number(slope)),
                ))
            } else {
                // (k*x + b)^n
                integrate_linear_power(base, as_number(exponent)?, name)
            }
        }

        Node::Builtin(function, arguments) if arguments.len() == 1 => {
            let inner = &arguments[0];

            if !has_var(inner, name) {
                return Some(mul(node.clone(), variable(name)));
            }

            let (slope, _) = linear_coefficients(inner, name)?;
            let antiderivative = builtin_integral(*function, inner.clone())?;

            Some(div(antiderivative, number(slope)))
        }

        Node::Call(function, arguments) => {
            let (params, body) = functions.get(function)?;

            let mut replacements = HashMap::new();

            for (param, argument) in params.iter().zip(arguments) {
                replacements.insert(*param, argument.clone());
            }

            let expanded = substitute(body, &replacements);

            integral(&expanded, name, functions, depth + 1)
        }

        _ => None,
    }
}

fn power_antiderivative(base: Node, power: f64) -> Option<Node> {
    if power == -1.0 {
        Some(ln(abs(base)))
    } else {
        Some(div(pow(base, number(power + 1.0)), number(power + 1.0)))
    }
}

// Antiderivative of a power of a linear expression (k*x + b)^power
fn integrate_linear_power(base: &Node, power: f64, name: char) -> Option<Node> {
    let (slope, _) = linear_coefficients(base, name)?;

    if power == -1.0 {
        Some(mul(div(number(1.0), number(slope)), ln(abs(base.clone()))))
    } else {
        Some(div(
            pow(base.clone(), number(power + 1.0)),
            mul(number(power + 1.0), number(slope)),
        ))
    }
}

// 1 / (c + b*x^2) -> (1 / sqrt(b*c)) * atan(x * sqrt(b/c)), valid for b*c > 0
fn integrate_quadratic_reciprocal(denominator: &Node, name: char) -> Option<Node> {
    let polynomial = to_polynomial(denominator, name)?;

    if polynomial.len() != 3 || !polynomial[1].is_zero() {
        return None;
    }

    let constant = polynomial[0].to_f64();
    let quadratic = polynomial[2].to_f64();

    if constant <= 0.0 || quadratic <= 0.0 {
        return None;
    }

    let scale = (quadratic / constant).sqrt();
    let factor = 1.0 / (quadratic * constant).sqrt();

    Some(mul(
        number(factor),
        builtin(Function::Atan, mul(variable(name), number(scale))),
    ))
}

// 1 / (c0 + c1*e^(k*x)) -> (1 / (k*c0)) * ln|e^(k*x) / (c0 + c1*e^(k*x))|
fn integrate_exp_reciprocal(denominator: &Node, name: char) -> Option<Node> {
    let mut constant = 0.0f64;
    let mut exponent = None;

    for (negative, term) in additive_terms(denominator) {
        let sign = if negative { -1.0 } else { 1.0 };

        if let Some(power) = split_exp_term(&term, name) {
            if exponent.is_some() {
                return None;
            }

            exponent = Some(power);
        } else if !has_var(&term, name) {
            constant += sign * as_number(&term)?;
        } else {
            return None;
        }
    }

    let exponent = exponent?;
    let (slope, _) = linear_coefficients(&exponent, name)?;

    if slope == 0.0 || constant == 0.0 {
        return None;
    }

    let exponential = pow(number(std::f64::consts::E), exponent);
    let divisor = slope * constant;

    let coefficient = if divisor.fract() == 0.0 && divisor.abs() < 1e9 {
        fraction(1, divisor as i128)
    } else {
        number(1.0 / divisor)
    };

    Some(mul(
        coefficient,
        ln(abs(div(exponential, denominator.clone()))),
    ))
}

// Additive terms of a sum, each tagged with a sign flip for subtraction
fn additive_terms(node: &Node) -> Vec<(bool, Node)> {
    fn walk(node: &Node, negative: bool, out: &mut Vec<(bool, Node)>) {
        match node {
            Node::Add(left, right) => {
                walk(left, negative, out);
                walk(right, negative, out);
            }
            Node::Sub(left, right) => {
                walk(left, negative, out);
                walk(right, !negative, out);
            }
            Node::Neg(inner) => walk(inner, !negative, out),
            other => out.push((negative, other.clone())),
        }
    }

    let mut out = Vec::new();

    walk(node, false, &mut out);

    out
}

// The exponent of a factor shaped like e^u, alone or scaled by a constant
fn split_exp_term(term: &Node, name: char) -> Option<Node> {
    match term {
        Node::Pow(base, exponent) if is_e_base(base) => Some((**exponent).clone()),
        Node::Mul(left, right) => {
            let exponential = if !has_var(left, name) {
                right
            } else if !has_var(right, name) {
                left
            } else {
                return None;
            };

            let Node::Pow(base, exponent) = &**exponential else {
                return None;
            };

            if !is_e_base(base) {
                return None;
            }

            Some((**exponent).clone())
        }
        _ => None,
    }
}

// True when the node is the base of the natural exponential
fn is_e_base(node: &Node) -> bool {
    matches!(node, Node::FloatNumber(value) if *value == std::f64::consts::E)
        || matches!(node, Node::Variable('e'))
}

// Substitution rule: f'(x) * g(f(x)) integrates to G(f(x)) with G' = g
fn integrate_substitution(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
) -> Option<Node> {
    let mut factors = Vec::new();

    flatten_factors(node, &mut factors);

    for (index, factor) in factors.iter().enumerate() {
        let (function, argument) = match factor {
            Node::Builtin(function, arguments) if arguments.len() == 1 => {
                (*function, &arguments[0])
            }
            Node::Pow(base, exponent) if is_e_base(base) => (Function::Exp, &**exponent),
            _ => continue,
        };

        if !has_var(argument, name) {
            continue;
        }

        let Some(derivative) = diff(argument, name, functions, 0) else {
            continue;
        };

        let mut rest = factors.clone();

        rest.remove(index);

        let ratio = simplify(&div(build_product(rest), derivative));

        if has_var(&ratio, name) {
            continue;
        }

        let antiderivative = builtin_integral(function, argument.clone())?;

        return Some(mul(ratio, antiderivative));
    }

    // Second mode: rest = c * F' for one factor F, so u = F and rest = c F^2 / 2
    for (index, factor) in factors.iter().enumerate() {
        let Some(derivative) = diff(factor, name, functions, 0) else {
            continue;
        };

        if !has_var(&derivative, name) {
            continue;
        }

        let mut rest = factors.clone();

        rest.remove(index);

        let ratio = simplify(&div(build_product(rest), derivative));

        if has_var(&ratio, name) {
            continue;
        }

        return Some(mul(
            div(ratio, number(2.0)),
            pow((*factor).clone(), number(2.0)),
        ));
    }

    None
}

// Integration by parts, choosing u by the LIATE order: u*v - integral(v*du)
fn integrate_by_parts(
    node: &Node,
    name: char,
    functions: &HashMap<String, (Vec<char>, Node)>,
    depth: usize,
) -> Option<Node> {
    let Node::Mul(left, right) = node else {
        return None;
    };

    let mut candidates = [(left, right), (right, left)];

    candidates.sort_by_key(|(u, _)| liate_priority(u, name).unwrap_or(u8::MAX));

    for (u, dv) in candidates {
        if liate_priority(u, name).is_none() || !has_var(dv, name) {
            continue;
        }

        let Some(antiderivative) = integral(dv, name, functions, depth + 1) else {
            continue;
        };

        let Some(derivative) = diff(u, name, functions, 0) else {
            continue;
        };

        let derivative = simplify(&derivative);
        let remainder = integral(
            &simplify(&mul(derivative, antiderivative.clone())),
            name,
            functions,
            depth + 1,
        );

        if let Some(remainder) = remainder {
            return Some(simplify(&sub(
                mul((**u).clone(), antiderivative),
                remainder,
            )));
        }
    }

    None
}

// LIATE priority: lower wins the choice of u (log, inverse, algebraic, trig, exp)
fn liate_priority(node: &Node, name: char) -> Option<u8> {
    match node {
        Node::Builtin(function, _) => match function {
            Function::Ln | Function::Log | Function::Log2 => Some(0),
            Function::Asin | Function::Acos | Function::Atan => Some(1),
            Function::Sin | Function::Cos | Function::Tan => Some(3),
            Function::Exp => Some(4),
            _ => None,
        },
        Node::Pow(base, exponent) if is_e_base(base) && has_var(exponent, name) => Some(4),
        _ if is_polynomial(node, name) => Some(2),
        _ => None,
    }
}

// True for a polynomial in the variable (integer or real constant powers)
fn is_polynomial(node: &Node, name: char) -> bool {
    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) => true,
        Node::Variable(other) => *other == name,
        Node::Neg(inner) => is_polynomial(inner, name),
        Node::Add(left, right) | Node::Sub(left, right) | Node::Mul(left, right) => {
            is_polynomial(left, name) && is_polynomial(right, name)
        }
        Node::Pow(base, exponent) => is_polynomial(base, name) && !has_var(exponent, name),
        _ => false,
    }
}

fn builtin_integral(function: Function, argument: Node) -> Option<Node> {
    let antiderivative = match function {
        Function::Sin => neg(builtin(Function::Cos, argument)),
        Function::Cos => builtin(Function::Sin, argument),
        Function::Exp => pow(number(std::f64::consts::E), argument),
        Function::Sinh => builtin(Function::Cosh, argument),
        Function::Cosh => builtin(Function::Sinh, argument),
        Function::Tan => neg(ln(abs(builtin(Function::Cos, argument)))),
        Function::Sqrt => mul(fraction(2, 3), pow(argument, number(1.5))),
        Function::Cbrt => mul(
            fraction(3, 4),
            mul(argument.clone(), builtin(Function::Cbrt, argument)),
        ),
        Function::Ln => sub(mul(argument.clone(), ln(argument.clone())), argument),
        _ => return None,
    };

    Some(antiderivative)
}

// Coefficients (slope, intercept) of a linear expression in the variable
// Rewrites 'left = right' into solutions for 'name', or None when unsolvable
pub fn solve_equation(left: &Node, right: &Node, name: char) -> Option<Vec<Node>> {
    let equation = simplify(&sub(left.clone(), right.clone()));

    if let Some((coefficient, constant)) = linear_parts(&equation, name) {
        let coefficient = simplify(&coefficient);
        let constant = simplify(&constant);

        if !is_zero(&coefficient) {
            return Some(vec![exactify(simplify(&div(neg(constant), coefficient)))]);
        }

        return None;
    }

    quadratic_solutions(&equation, name)
}

// Splits a linear expression into (coefficient, constant) nodes in 'name'
fn linear_parts(node: &Node, name: char) -> Option<(Node, Node)> {
    if !has_var(node, name) {
        return Some((number(0.0), node.clone()));
    }

    match node {
        Node::Variable(other) if *other == name => Some((number(1.0), number(0.0))),
        Node::Neg(inner) => {
            let (coefficient, constant) = linear_parts(inner, name)?;

            Some((neg(coefficient), neg(constant)))
        }
        Node::Add(left, right) => {
            let (left_coefficient, left_constant) = linear_parts(left, name)?;
            let (right_coefficient, right_constant) = linear_parts(right, name)?;

            Some((
                add(left_coefficient, right_coefficient),
                add(left_constant, right_constant),
            ))
        }
        Node::Sub(left, right) => {
            let (left_coefficient, left_constant) = linear_parts(left, name)?;
            let (right_coefficient, right_constant) = linear_parts(right, name)?;

            Some((
                sub(left_coefficient, right_coefficient),
                sub(left_constant, right_constant),
            ))
        }
        Node::Mul(left, right) => {
            if !has_var(left, name) {
                let (coefficient, constant) = linear_parts(right, name)?;

                Some((
                    mul((**left).clone(), coefficient),
                    mul((**left).clone(), constant),
                ))
            } else if !has_var(right, name) {
                let (coefficient, constant) = linear_parts(left, name)?;

                Some((
                    mul((**right).clone(), coefficient),
                    mul((**right).clone(), constant),
                ))
            } else {
                None
            }
        }
        Node::Div(numerator, denominator) => {
            if has_var(denominator, name) {
                None
            } else {
                let (coefficient, constant) = linear_parts(numerator, name)?;

                Some((
                    div(coefficient, (**denominator).clone()),
                    div(constant, (**denominator).clone()),
                ))
            }
        }
        _ => None,
    }
}

// Solves a degree-two polynomial with numeric coefficients
fn quadratic_solutions(equation: &Node, name: char) -> Option<Vec<Node>> {
    let (square, linear, constant) = quadratic_parts(equation, name)?;

    let square = numeric_value(&simplify(&square))?;
    let linear = numeric_value(&simplify(&linear))?;
    let constant = numeric_value(&simplify(&constant))?;

    if square == 0.0 {
        return None;
    }

    let discriminant = linear * linear - 4.0 * square * constant;

    if discriminant < 0.0 {
        return Some(Vec::new());
    }

    let root = discriminant.sqrt();

    if (root - root.round()).abs() < 1e-9 {
        let first = exactify(number((-linear + root) / (2.0 * square)));
        let second = exactify(number((-linear - root) / (2.0 * square)));

        return Some(vec![first, second]);
    }

    let discriminant_node = simplify(&sub(
        pow(number(linear), number(2.0)),
        number(4.0 * square * constant),
    ));
    let root_node = square_root_node(discriminant, discriminant_node);
    let denominator = number(2.0 * square);

    Some(vec![
        exactify(simplify(&div(
            add(number(-linear), root_node.clone()),
            denominator.clone(),
        ))),
        exactify(simplify(&div(sub(number(-linear), root_node), denominator))),
    ])
}

// Rewrites a constant node as an exact fraction when possible
fn exactify(node: Node) -> Node {
    match fold_rational(&node) {
        Some(rational) => rational_to_node(rational),
        None => node,
    }
}

// Builds sqrt(discriminant) pulling out square factors of an integer radicand
fn square_root_node(value: f64, node: Node) -> Node {
    if value.fract() != 0.0 || value <= 0.0 {
        return builtin(Function::Sqrt, node);
    }

    let mut radicand = value as i64;
    let mut outside = 1i64;
    let mut factor = 2i64;

    while factor * factor <= radicand {
        while radicand % (factor * factor) == 0 {
            radicand /= factor * factor;
            outside *= factor;
        }

        factor += 1;
    }

    let root = builtin(Function::Sqrt, number(radicand as f64));

    if outside == 1 {
        root
    } else {
        mul(number(outside as f64), root)
    }
}

// Splits a degree-two polynomial into (square, linear, constant) nodes
fn quadratic_parts(node: &Node, name: char) -> Option<(Node, Node, Node)> {
    if !has_var(node, name) {
        return Some((number(0.0), number(0.0), node.clone()));
    }

    match node {
        Node::Variable(other) if *other == name => Some((number(0.0), number(1.0), number(0.0))),
        Node::Neg(inner) => {
            let (square, linear, constant) = quadratic_parts(inner, name)?;

            Some((neg(square), neg(linear), neg(constant)))
        }
        Node::Add(left, right) => {
            let (ls, ll, lc) = quadratic_parts(left, name)?;
            let (rs, rl, rc) = quadratic_parts(right, name)?;

            Some((add(ls, rs), add(ll, rl), add(lc, rc)))
        }
        Node::Sub(left, right) => {
            let (ls, ll, lc) = quadratic_parts(left, name)?;
            let (rs, rl, rc) = quadratic_parts(right, name)?;

            Some((sub(ls, rs), sub(ll, rl), sub(lc, rc)))
        }
        Node::Mul(left, right) => {
            if !has_var(left, name) {
                let (square, linear, constant) = quadratic_parts(right, name)?;

                Some((
                    mul((**left).clone(), square),
                    mul((**left).clone(), linear),
                    mul((**left).clone(), constant),
                ))
            } else if !has_var(right, name) {
                let (square, linear, constant) = quadratic_parts(left, name)?;

                Some((
                    mul((**right).clone(), square),
                    mul((**right).clone(), linear),
                    mul((**right).clone(), constant),
                ))
            } else {
                None
            }
        }
        Node::Div(numerator, denominator) => {
            if has_var(denominator, name) {
                None
            } else {
                let (square, linear, constant) = quadratic_parts(numerator, name)?;

                Some((
                    div(square, (**denominator).clone()),
                    div(linear, (**denominator).clone()),
                    div(constant, (**denominator).clone()),
                ))
            }
        }
        Node::Pow(base, exponent) => {
            if is_variable(base, name) && as_number(exponent) == Some(2.0) {
                Some((number(1.0), number(0.0), number(0.0)))
            } else if is_variable(base, name) && as_number(exponent) == Some(1.0) {
                Some((number(0.0), number(1.0), number(0.0)))
            } else if !has_var(base, name) {
                Some((number(0.0), number(0.0), node.clone()))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_zero(node: &Node) -> bool {
    matches!(numeric_value(node), Some(value) if value == 0.0)
}

// Reads a folded constant, folding a leading negation
fn numeric_value(node: &Node) -> Option<f64> {
    match node {
        Node::IntNumber(value) => Some(*value as f64),
        Node::FloatNumber(value) => Some(*value),
        Node::Neg(inner) => Some(-numeric_value(inner)?),
        _ => None,
    }
}

fn linear_coefficients(node: &Node, name: char) -> Option<(f64, f64)> {
    match node {
        Node::IntNumber(value) => Some((0.0, *value as f64)),
        Node::FloatNumber(value) => Some((0.0, *value)),
        Node::Variable(other) if *other == name => Some((1.0, 0.0)),
        Node::Variable(_) => None,
        Node::Neg(inner) => {
            let (slope, intercept) = linear_coefficients(inner, name)?;

            Some((-slope, -intercept))
        }
        Node::Add(left, right) => {
            let (left_slope, left_intercept) = linear_coefficients(left, name)?;
            let (right_slope, right_intercept) = linear_coefficients(right, name)?;

            Some((left_slope + right_slope, left_intercept + right_intercept))
        }
        Node::Sub(left, right) => {
            let (left_slope, left_intercept) = linear_coefficients(left, name)?;
            let (right_slope, right_intercept) = linear_coefficients(right, name)?;

            Some((left_slope - right_slope, left_intercept - right_intercept))
        }
        Node::Mul(left, right) => {
            if let Some(value) = as_number(left) {
                let (slope, intercept) = linear_coefficients(right, name)?;

                Some((value * slope, value * intercept))
            } else if let Some(value) = as_number(right) {
                let (slope, intercept) = linear_coefficients(left, name)?;

                Some((value * slope, value * intercept))
            } else {
                None
            }
        }
        Node::Div(numerator, denominator) => {
            let value = as_number(denominator)?;
            let (slope, intercept) = linear_coefficients(numerator, name)?;

            Some((slope / value, intercept / value))
        }
        _ => None,
    }
}

// --- inverses -----------------------------------------------------------

// Inverse of a built-in applied to one argument, as a node in that argument
pub fn builtin_inverse(function: Function, argument: Node) -> Option<Node> {
    let inverse = match function {
        Function::Sin => builtin(Function::Asin, argument),
        Function::Cos => builtin(Function::Acos, argument),
        Function::Tan => builtin(Function::Atan, argument),
        Function::Asin => builtin(Function::Sin, argument),
        Function::Acos => builtin(Function::Cos, argument),
        Function::Atan => builtin(Function::Tan, argument),
        Function::Sinh => builtin(Function::Asinh, argument),
        Function::Cosh => builtin(Function::Acosh, argument),
        Function::Tanh => builtin(Function::Atanh, argument),
        Function::Asinh => builtin(Function::Sinh, argument),
        Function::Acosh => builtin(Function::Cosh, argument),
        Function::Atanh => builtin(Function::Tanh, argument),
        Function::Exp => builtin(Function::Ln, argument),
        Function::Ln => builtin(Function::Exp, argument),
        Function::Log => pow(number(10.0), argument),
        Function::Log2 => pow(number(2.0), argument),
        Function::Sqrt => pow(argument, number(2.0)),
        Function::Cbrt => pow(argument, number(3.0)),
        _ => return None,
    };

    Some(inverse)
}

// Inverse of a user body: affine in its parameter or a scaled pure power
pub fn user_inverse(body: &Node, param: char, argument: Node) -> Option<Node> {
    let body = simplify(body);

    if let Some((slope, intercept)) = linear_coefficients(&body, param) {
        if slope == 0.0 {
            return None;
        }

        let shifted = sub(argument, number(intercept));

        return Some(if slope == 1.0 {
            shifted
        } else {
            div(shifted, number(slope))
        });
    }

    let (coefficient, base) = split_coefficient(&body);
    let exponent = match &base {
        Node::Pow(inner, exponent) if is_variable(inner, param) => as_number(exponent)?,
        _ => return None,
    };

    if coefficient.is_zero() || exponent == 0.0 {
        return None;
    }

    let scaled = if coefficient.is_one() {
        argument
    } else {
        div(argument, coefficient.to_node())
    };

    Some(pow(scaled, number(1.0 / exponent)))
}
