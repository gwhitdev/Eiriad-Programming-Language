import { useState, useEffect, useRef } from "react";

function useFonts() {
  useEffect(() => {
    const l = document.createElement("link");
    l.rel = "stylesheet";
    l.href = "https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,300;0,9..144,600;0,9..144,700;1,9..144,400&family=IBM+Plex+Mono:ital,wght@0,400;0,600;1,400&family=DM+Sans:wght@300;400;500&display=swap";
    document.head.appendChild(l);
  }, []);
}

// ─── Syntax highlight ─────────────────────────────────────────────────────────
function hl(code) {
  // Build a list of [start, end, color] spans over the escaped source,
  // then reconstruct HTML in one pass — no placeholder substitution needed.
  const escaped = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  const rules = [
    { re: /(\/\/[^\n]*)/g,                                                                                 col: "#8fa8a0" },
    { re: /(["'`][^"'`\n]*["'`])/g,                                                                       col: "#a8d5a0" },
    { re: /\b(let|mut|fn|async|await|match|if|else|return|class|signal|effect|import|export|from|in|for|while)\b/g, col: "#e07b54" },
    { re: /\b(Int|Float|Bool|Str|List|Map|Option|Result|self|Self|T|E|K|V)\b/g,                           col: "#7dc4e4" },
    { re: /\b(true|false|null|None|Some|Ok|Err)\b/g,                                                      col: "#c792ea" },
    { re: /\b(\d+\.?\d*)\b/g,                                                                             col: "#f0c36d" },
    { re: /(\|&gt;|->|\.\.\.?|::)/g,                                                                      col: "#f0a830" },
  ];

  // Collect all non-overlapping spans (first match wins)
  const spans = []; // { start, end, col }
  for (const { re, col } of rules) {
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(escaped)) !== null) {
      const start = m.index, end = m.index + m[0].length;
      const overlaps = spans.some(s => start < s.end && end > s.start);
      if (!overlaps) spans.push({ start, end, col });
    }
  }
  spans.sort((a, b) => a.start - b.start);

  // Reconstruct
  let out = "", pos = 0;
  for (const { start, end, col } of spans) {
    if (pos < start) out += escaped.slice(pos, start);
    out += `<span style="color:${col}">${escaped.slice(start, end)}</span>`;
    pos = end;
  }
  out += escaped.slice(pos);
  return out;
}

function Code({ children, inline }) {
  if (inline) return (
    <code style={{ fontFamily:"'IBM Plex Mono',monospace", fontSize:12.5, background:"#f0ede8", color:"#c05c34", padding:"1px 6px", borderRadius:4 }}>
      {children}
    </code>
  );
  return (
    <pre style={{ margin:"12px 0", background:"#1a1d2e", borderRadius:8, padding:"16px 20px", fontFamily:"'IBM Plex Mono',monospace", fontSize:12.5, lineHeight:1.75, color:"#c8d3e8", overflowX:"auto", border:"1px solid #252a3a" }}>
      <code dangerouslySetInnerHTML={{ __html: hl(children) }} />
    </pre>
  );
}

// ─── Pipeline Diagram ─────────────────────────────────────────────────────────
const STAGES = [
  {
    id: "source", label: "Source Text", color: "#e07b54", icon: "①",
    desc: "Raw EIRIAD source code as a UTF-8 string. The entry point for all compilation.",
    detail: "The compiler receives a string of EIRIAD source. No preprocessing or macros occur at this stage. Comments are retained by the lexer for IDE tooling, then discarded before parsing.",
    example: `let x: Int = 40 + 2\nprint(x)`,
  },
  {
    id: "lexer", label: "Lexer / Tokenizer", color: "#f0a830", icon: "②",
    desc: "Splits raw text into a flat stream of typed tokens. No structure yet — just classification.",
    detail: "The lexer scans left-to-right, consuming characters and emitting tokens. Each token carries: kind (Keyword, Ident, Int, Str, Op, Punct, EOF), lexeme (the raw text), and source position (line, col). Whitespace is significant only for newline-as-statement-terminator; other whitespace is discarded.",
    example: `// Token stream for: let x: Int = 40 + 2\n[Keyword:let] [Ident:x] [Punct::] [Ident:Int]\n[Op:=] [Int:40] [Op:+] [Int:2] [EOF]`,
  },
  {
    id: "parser", label: "Parser → AST", color: "#7dc4e4", icon: "③",
    desc: "Consumes the token stream and builds a typed Abstract Syntax Tree using recursive descent.",
    detail: "A hand-written recursive descent parser converts the flat token stream into a tree. Each grammar rule maps to a parse function. The AST nodes carry source spans for error messages. The parser is error-recovering — on a syntax error it emits a diagnostic and resumes at the next statement boundary.",
    example: `// AST for: let x: Int = 40 + 2\nLetDecl {\n  name: "x"\n  type_ann: Ident("Int")\n  value: BinExpr {\n    op: Add\n    left:  IntLit(40)\n    right: IntLit(2)\n  }\n}`,
  },
  {
    id: "checker", label: "Type Checker", color: "#c792ea", icon: "④",
    desc: "Walks the AST, resolves names, infers and verifies types. Errors are collected, not thrown.",
    detail: "Type checking is a single-pass walk over the AST. A TypeEnv (scoped symbol table) tracks variable types. Unresolved type annotations are looked up in scope; inferred types are computed bottom-up. Type errors are collected into a diagnostics list rather than aborting immediately, so you see all errors at once. Generic types are instantiated lazily.",
    example: `// Type environment snapshot\nEnv {\n  x: Int        // resolved from annotation\n}\n// Inferred:\n// BinExpr(40+2) : Int  ✓\n// LetDecl type:  Int  ✓`,
  },
  {
    id: "evaluator", label: "Evaluator", color: "#a8d5a0", icon: "⑤",
    desc: "Tree-walk interpreter. Evaluates each AST node in a runtime environment.",
    detail: "EIRIAD uses a tree-walking interpreter (no bytecode yet). An Env chain handles lexical scoping. Values are tagged runtime variants: VInt, VFloat, VStr, VBool, VList, VMap, VFn, VSome, VNone, VOk, VErr. Tail-call optimisation is applied to direct recursive calls. Async functions return VPromise, backed by a microtask queue.",
    example: `// Runtime env after evaluation\nEnv {\n  x: VInt(42)\n}\n// stdout: 42`,
  },
];

function PipelineNode({ stage, active, onClick }) {
  return (
    <div
      onClick={onClick}
      style={{
        display: "flex", flexDirection: "column", alignItems: "center",
        cursor: "pointer", flex: 1, minWidth: 90,
      }}
    >
      <div style={{
        width: 52, height: 52, borderRadius: "50%",
        background: active ? stage.color : "#f0ede8",
        border: `2px solid ${active ? stage.color : "#ddd8d0"}`,
        display: "flex", alignItems: "center", justifyContent: "center",
        fontSize: 22, transition: "all 0.2s",
        boxShadow: active ? `0 0 0 4px ${stage.color}22` : "none",
        color: active ? "#fff" : "#888",
      }}>
        {stage.icon}
      </div>
      <div style={{
        marginTop: 8, fontSize: 11, fontFamily: "'IBM Plex Mono', monospace",
        color: active ? stage.color : "#888",
        textAlign: "center", letterSpacing: "0.03em",
        fontWeight: active ? 600 : 400,
        transition: "color 0.2s",
      }}>
        {stage.label}
      </div>
    </div>
  );
}

function PipelineArrow() {
  return (
    <div style={{ display:"flex", alignItems:"center", paddingBottom:24, color:"#ccc", fontSize:18, flexShrink:0 }}>
      →
    </div>
  );
}

// ─── Spec Sections ────────────────────────────────────────────────────────────
const SECTIONS = [
  { id: "overview",    label: "Overview" },
  { id: "lexical",     label: "Lexical Rules" },
  { id: "types",       label: "Type System" },
  { id: "variables",   label: "Variables" },
  { id: "functions",   label: "Functions" },
  { id: "control",     label: "Control Flow" },
  { id: "classes",     label: "Classes" },
  { id: "errors",      label: "Error Handling" },
  { id: "async",       label: "Async / Concurrency" },
  { id: "reactive",    label: "Reactivity" },
  { id: "stdlib",      label: "Standard Library" },
  { id: "pipeline",    label: "Interpreter Pipeline" },
  { id: "grammar",     label: "Formal Grammar" },
];

const H2 = ({children}) => (
  <h2 style={{ fontFamily:"'Fraunces',serif", fontSize:28, fontWeight:700, color:"#1a1512", margin:"40px 0 6px", letterSpacing:"-0.02em" }}>
    {children}
  </h2>
);
const H3 = ({children}) => (
  <h3 style={{ fontFamily:"'Fraunces',serif", fontSize:18, fontWeight:600, color:"#2a2420", margin:"28px 0 6px" }}>
    {children}
  </h3>
);
const P = ({children, style}) => (
  <p style={{ fontFamily:"'DM Sans',sans-serif", fontSize:15, color:"#4a4540", lineHeight:1.75, margin:"8px 0", ...style }}>
    {children}
  </p>
);
const Note = ({children}) => (
  <div style={{ background:"#fdf8f0", border:"1px solid #e8d8b0", borderLeft:"3px solid #f0a830", borderRadius:6, padding:"10px 14px", margin:"12px 0" }}>
    <P style={{ margin:0, color:"#705820" }}>{children}</P>
  </div>
);
const Table = ({headers, rows}) => (
  <div style={{ overflowX:"auto", margin:"12px 0" }}>
    <table style={{ width:"100%", borderCollapse:"collapse", fontFamily:"'DM Sans',sans-serif", fontSize:14 }}>
      <thead>
        <tr>{headers.map(h=>(
          <th key={h} style={{ textAlign:"left", padding:"8px 12px", background:"#f0ede8", color:"#4a4540", fontWeight:500, borderBottom:"1px solid #ddd8d0", fontSize:12, letterSpacing:"0.05em", textTransform:"uppercase" }}>{h}</th>
        ))}</tr>
      </thead>
      <tbody>
        {rows.map((r,i)=>(
          <tr key={i} style={{ background: i%2===0?"#fff":"#faf8f5" }}>
            {r.map((c,j)=>(
              <td key={j} style={{ padding:"8px 12px", borderBottom:"1px solid #eee8e0", color:"#2a2420", verticalAlign:"top" }}>
                {typeof c==="string" && c.startsWith("`") ? <Code inline>{c.slice(1,-1)}</Code> : c}
              </td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);

// ─── Main ─────────────────────────────────────────────────────────────────────
export default function EiriadSpec() {
  useFonts();
  const [activeSection, setActiveSection] = useState("overview");
  const [activeStage, setActiveStage] = useState(0);
  const contentRef = useRef(null);

  function scrollTo(id) {
    setActiveSection(id);
    const el = document.getElementById("sec-"+id);
    if (el) el.scrollIntoView({ behavior:"smooth", block:"start" });
  }

  return (
    <div style={{ fontFamily:"'DM Sans',sans-serif", background:"#faf8f5", minHeight:"100vh", display:"flex", flexDirection:"column" }}>

      {/* Header */}
      <header style={{ background:"#1a1512", color:"#f0ede8", padding:"20px 40px", display:"flex", alignItems:"center", gap:20, borderBottom:"3px solid #f0a830" }}>
        <div>
          <div style={{ fontFamily:"'Fraunces',serif", fontSize:26, fontWeight:700, letterSpacing:"-0.03em" }}>
            VEL<span style={{ color:"#f0a830" }}>OX</span>
          </div>
          <div style={{ fontFamily:"'IBM Plex Mono',monospace", fontSize:10, color:"#6a6058", letterSpacing:"0.1em", textTransform:"uppercase", marginTop:2 }}>
            Language Specification · v0.1
          </div>
        </div>
        <div style={{ flex:1 }} />
        <div style={{ fontFamily:"'IBM Plex Mono',monospace", fontSize:11, color:"#6a6058" }}>
          Draft · {new Date().getFullYear()}
        </div>
      </header>

      <div style={{ display:"flex", flex:1, maxHeight:"calc(100vh - 73px)" }}>

        {/* Sidebar */}
        <nav style={{ width:200, flexShrink:0, background:"#f0ede8", borderRight:"1px solid #ddd8d0", overflowY:"auto", padding:"20px 0" }}>
          {SECTIONS.map(s => (
            <button key={s.id} onClick={()=>scrollTo(s.id)} style={{
              display:"block", width:"100%", textAlign:"left",
              padding:"7px 20px", border:"none", background:"none",
              fontFamily:"'DM Sans',sans-serif", fontSize:13,
              color: activeSection===s.id ? "#c05c34" : "#6a6058",
              fontWeight: activeSection===s.id ? 500 : 400,
              cursor:"pointer",
              borderLeft: activeSection===s.id ? "2px solid #f0a830" : "2px solid transparent",
              transition:"all 0.1s",
            }}>
              {s.label}
            </button>
          ))}
        </nav>

        {/* Content */}
        <main ref={contentRef} style={{ flex:1, overflowY:"auto", padding:"40px 48px", maxWidth:860 }}>

          {/* ── Overview ── */}
          <section id="sec-overview">
            <H2>Overview</H2>
            <P>
              EIRIAD is a statically-typed, expression-oriented scripting language designed to run natively in web browsers and server runtimes. It compiles to an intermediate bytecode (future roadmap) or is evaluated via a tree-walking interpreter (current). The syntax is inspired by Rust, Elm, and Python, with deliberate departures from JavaScript's legacy design decisions.
            </P>
            <Table
              headers={["Property", "Value"]}
              rows={[
                ["Paradigm", "Functional-first, OO supported"],
                ["Typing", "Static, inferred, gradual"],
                ["Mutability", "Immutable by default"],
                ["Null safety", "No null/undefined — Option<T>"],
                ["Error model", "Result<T,E> — no throw/catch"],
                ["Async model", "async/await + structured concurrency"],
                ["Reactivity", "Built-in signal/effect primitives"],
                ["Target", "Browser (WASM planned), Node-like runtime"],
              ]}
            />
          </section>

          {/* ── Lexical ── */}
          <section id="sec-lexical">
            <H2>Lexical Rules</H2>
            <H3>Comments</H3>
            <Code>{`// Single-line comment
/* Multi-line comment
   spans multiple lines */`}</Code>

            <H3>Identifiers</H3>
            <P>Identifiers start with a letter or underscore, followed by letters, digits, or underscores. EIRIAD uses <Code inline>snake_case</Code> for variables and functions, <Code inline>PascalCase</Code> for types and classes.</P>
            <Code>{`valid_ident   _private   MyClass   HTTP2Client`}</Code>

            <H3>Literals</H3>
            <Table headers={["Kind","Example","Notes"]}
              rows={[
                ["Integer",   "`42`",          "64-bit signed"],
                ["Float",     "`3.14`",         "64-bit IEEE 754"],
                ["String",    "`\"hello\"`",    "UTF-8, interpolation via {expr}"],
                ["Bool",      "`true / false`", ""],
                ["List",      "`[1, 2, 3]`",    "Heterogeneous allowed"],
                ["Map",       "`{a: 1, b: 2}`", "String keys by default"],
              ]}
            />

            <H3>Operators</H3>
            <Table headers={["Operator","Meaning","Precedence"]}
              rows={[
                ["`|>`",  "Pipe (left-associative)",   "1 (lowest)"],
                ["`||`",  "Logical OR",                 "2"],
                ["`&&`",  "Logical AND",                "3"],
                ["`== !=`","Equality",                  "4"],
                ["`< > <= >=`","Comparison",            "5"],
                ["`+ -`", "Addition / Subtraction",     "6"],
                ["`* / %`","Multiplication / Division", "7"],
                ["`^`",   "Exponentiation (right-assoc)","8"],
                ["`!`",   "Logical NOT (prefix)",       "9 (highest)"],
              ]}
            />

            <H3>Statement Terminators</H3>
            <P>EIRIAD uses newlines as implicit statement terminators (like Go). A trailing backslash <Code inline>\</Code> continues a statement onto the next line. Semicolons are permitted but discouraged.</P>
          </section>

          {/* ── Types ── */}
          <section id="sec-types">
            <H2>Type System</H2>
            <P>EIRIAD has a Hindley-Milner-inspired type inference engine. Annotations are optional — omit them and the type is inferred. When present, annotations are checked and serve as documentation.</P>

            <H3>Primitive Types</H3>
            <Table headers={["Type","Description","Literal"]}
              rows={[
                ["`Int`",   "64-bit signed integer",   "`42`"],
                ["`Float`", "64-bit IEEE float",        "`3.14`"],
                ["`Bool`",  "Boolean",                  "`true`"],
                ["`Str`",   "UTF-8 string",             "`\"hi\"`"],
                ["`()`",    "Unit (no meaningful value)","implicit"],
              ]}
            />

            <H3>Generic / Container Types</H3>
            <Code>{`List<Int>           // [1, 2, 3]
Map<Str, Int>       // {"a": 1}
Option<T>           // Some(value) or None
Result<T, E>        // Ok(value) or Err(error)
fn(Int, Int) -> Int // Function type`}</Code>

            <H3>Type Annotations</H3>
            <Code>{`let x: Int = 5          // explicit
let y = 5               // inferred as Int
let f: fn(Int) -> Bool  // function type annotation

fn greet(name: Str) -> Str {
  "Hello, {name}"
}`}</Code>

            <H3>Generics</H3>
            <Code>{`fn identity<T>(x: T) -> T { x }

fn map_list<T, U>(list: List<T>, f: fn(T) -> U) -> List<U> {
  // built-in map delegates here
}`}</Code>

            <Note>EIRIAD does NOT have implicit type coercion. <Code inline>1 + "2"</Code> is a compile-time type error, not <Code inline>"12"</Code>.</Note>
          </section>

          {/* ── Variables ── */}
          <section id="sec-variables">
            <H2>Variables</H2>
            <Code>{`// Immutable binding (default)
let name = "Alice"
let pi: Float = 3.14159

// Mutable binding
mut count = 0
count = count + 1
count += 1  // shorthand

// Destructuring
let [first, ...rest] = [1, 2, 3]
let { x, y } = point

// Shadowing (creates a new binding)
let x = 5
let x = x * 2  // x is now 10, previous x is dropped`}</Code>
            <Note>Attempting to reassign a <Code inline>let</Code> binding is a compile-time error, not a runtime one.</Note>
          </section>

          {/* ── Functions ── */}
          <section id="sec-functions">
            <H2>Functions</H2>
            <H3>Declaration forms</H3>
            <Code>{`// Named function
fn add(a: Int, b: Int) -> Int {
  a + b          // implicit return — last expression
}

// Arrow shorthand (single expression)
let double = (x: Int) -> x * 2

// Anonymous (assigned to let)
let greet = (name: Str) -> "Hello, {name}"

// Higher-order
fn apply(f: fn(Int) -> Int, x: Int) -> Int {
  f(x)
}`}</Code>

            <H3>Default & Named Arguments</H3>
            <Code>{`fn connect(host: Str, port: Int = 8080, tls: Bool = false) -> Connection {
  // ...
}

connect("localhost")                    // port=8080, tls=false
connect("prod.example.com", tls: true) // named arg, port stays 8080`}</Code>

            <H3>The Pipe Operator</H3>
            <Code>{`// These are equivalent:
let result = sum(map(filter([1,2,3,4,5], is_even), double))

// Pipe: left-to-right, each output feeds the next
let result = [1, 2, 3, 4, 5]
  |> filter(is_even)
  |> map(double)
  |> sum()

// Pipe with a closure inline
let result = "  hello world  "
  |> trim()
  |> split(" ")
  |> map((w) -> capitalize(w))
  |> join(" ")  // => "Hello World"`}</Code>

            <H3>Closures</H3>
            <Code>{`fn make_counter() -> fn() -> Int {
  mut n = 0
  () -> {
    n += 1
    n
  }
}

let counter = make_counter()
counter()  // => 1
counter()  // => 2`}</Code>
          </section>

          {/* ── Control Flow ── */}
          <section id="sec-control">
            <H2>Control Flow</H2>
            <H3>if / else</H3>
            <P>Conditions are expressions — they return a value.</P>
            <Code>{`// As statement
if score > 90 {
  print("A")
} else if score > 75 {
  print("B")
} else {
  print("C")
}

// As expression
let grade = if score > 90 { "A" } else { "B" }`}</Code>

            <H3>match</H3>
            <P>Exhaustive pattern matching. The compiler errors if any variant is unhandled.</P>
            <Code>{`match value {
  0      -> "zero"
  1..10  -> "small"         // range pattern
  n if n < 0 -> "negative" // guard
  _      -> "large"         // wildcard
}

// Matching on Option
match find_user(id) {
  Some(user) -> greet(user.name)
  None       -> "not found"
}

// Matching on Result
match parse_json(input) {
  Ok(data)  -> process(data)
  Err(e)    -> log("Parse failed: {e.message}")
}

// Destructuring in match
match point {
  { x: 0, y } -> "on y-axis at {y}"
  { x, y: 0 } -> "on x-axis at {x}"
  { x, y }    -> "at ({x}, {y})"
}`}</Code>

            <H3>Loops</H3>
            <Code>{`// for-in (iterable)
for item in list { print(item) }
for (i, item) in list.enumerate() { ... }

// Range
for i in 0..10 { ... }   // 0–9
for i in 0..=10 { ... }  // 0–10

// while
mut i = 0
while i < 10 {
  i += 1
}

// loop (infinite, exit with break/return)
loop {
  if done { break }
}`}</Code>
          </section>

          {/* ── Classes ── */}
          <section id="sec-classes">
            <H2>Classes</H2>
            <Code>{`class Rectangle {
  width: Float
  height: Float

  // Constructor is implicit — fields are the constructor params
  // let r = Rectangle { width: 10.0, height: 5.0 }

  fn area() -> Float {
    self.width * self.height
  }

  fn scale(factor: Float) -> Rectangle {
    Rectangle { width: self.width * factor,
                height: self.height * factor }
  }
}

// Inheritance via 'extends'
class Square extends Rectangle {
  // width == height guaranteed
  fn new(side: Float) -> Square {
    Square { width: side, height: side }
  }
}

let sq = Square.new(4.0)
sq.area()  // => 16.0`}</Code>

            <H3>Traits (Interfaces)</H3>
            <Code>{`trait Printable {
  fn to_str() -> Str
}

class Point {
  x: Int
  y: Int

  // Implement a trait
  impl Printable {
    fn to_str() -> Str { "({self.x}, {self.y})" }
  }
}

fn print_thing(item: Printable) {
  print(item.to_str())
}`}</Code>
          </section>

          {/* ── Error Handling ── */}
          <section id="sec-errors">
            <H2>Error Handling</H2>
            <P>EIRIAD has no <Code inline>throw</Code> or <Code inline>try/catch</Code>. All errors are values of type <Code inline>Result&lt;T, E&gt;</Code>.</P>
            <Code>{`fn divide(a: Float, b: Float) -> Result<Float, Str> {
  if b == 0.0 {
    Err("division by zero")
  } else {
    Ok(a / b)
  }
}

// Pattern match to handle
match divide(10.0, 0.0) {
  Ok(n)  -> print("Result: {n}")
  Err(e) -> print("Error: {e}")
}

// Propagate with '?' operator (like Rust)
fn parse_and_divide(a: Str, b: Str) -> Result<Float, Str> {
  let x = parse_float(a)?  // returns Err early if parse fails
  let y = parse_float(b)?
  divide(x, y)
}

// Provide defaults
let val = divide(10.0, 0.0) |> unwrap_or(0.0)

// Chain operations on Ok, skip on Err
let result = parse_float("3.14")
  |> and_then((n) -> divide(n, 2.0))
  |> unwrap_or(0.0)`}</Code>
            <Note>The <Code inline>?</Code> operator can only be used inside a function returning <Code inline>Result</Code>. This makes error propagation explicit but ergonomic.</Note>
          </section>

          {/* ── Async ── */}
          <section id="sec-async">
            <H2>Async / Concurrency</H2>
            <Code>{`// Async functions return Result<T, E> automatically
async fn load_user(id: Int) -> Result<User, HttpError> {
  let res = await fetch("https://api.example.com/users/{id}")
  res |> json<User>()
}

// Parallel execution
let [user, posts] = await all([
  load_user(42),
  load_posts(42),
])

// Race — first to resolve wins
let data = await race([
  load_user(42),
  timeout(5000),   // built-in
])

// Structured concurrency — spawn returns a handle
let task = spawn(load_heavy_data())
// ... do other work ...
let result = await task`}</Code>
          </section>

          {/* ── Reactive ── */}
          <section id="sec-reactive">
            <H2>Reactivity</H2>
            <P>EIRIAD has first-class reactive primitives — no framework required for basic UI.</P>
            <Code>{`// Declare reactive state
signal count = 0
signal items: List<Str> = []

// Derived signal (computed automatically)
signal double_count = count * 2

// Effect — runs when dependencies change
effect {
  document.title = "Items: {len(items)}"
  print("count changed to {count}")
}

// Mutation triggers effects and derived signals
fn add_item(name: Str) {
  items = [...items, name]
  count += 1
}

// In templates (planned syntax)
// <div>{count}</div>  -- auto-updates
// <button onClick={add_item("New")}>Add</button>`}</Code>
            <Note>Signals use fine-grained reactivity — only effects that actually read a signal re-run when it changes. There is no VDOM diffing at this level.</Note>
          </section>

          {/* ── Stdlib ── */}
          <section id="sec-stdlib">
            <H2>Standard Library</H2>
            <Table headers={["Module","Key Functions"]}
              rows={[
                ["core",    "print, assert, panic, typeof, len, range"],
                ["list",    "filter, map, reduce, zip, flat_map, sort, find, any, all, sum, first, last, reverse, chunk"],
                ["map",     "keys, values, entries, merge, get, set, delete, has"],
                ["str",     "split, join, trim, starts_with, ends_with, contains, replace, to_upper, to_lower, pad_start, pad_end, repeat"],
                ["math",    "sqrt, floor, ceil, round, abs, min, max, clamp, sin, cos, tan, log, pow, random"],
                ["option",  "unwrap, unwrap_or, map, and_then, is_some, is_none"],
                ["result",  "unwrap, unwrap_or, map, map_err, and_then, is_ok, is_err"],
                ["io",      "fetch, read_file, write_file, stdin, stdout"],
                ["async",   "all, race, timeout, sleep, spawn"],
                ["json",    "parse, stringify"],
                ["time",    "now, format, parse_date"],
              ]}
            />
          </section>

          {/* ── Pipeline ── */}
          <section id="sec-pipeline">
            <H2>Interpreter Pipeline</H2>
            <P>When you run EIRIAD code, it passes through five distinct stages before producing output. Click each stage to explore what happens.</P>

            {/* Pipeline diagram */}
            <div style={{ background:"#fff", border:"1px solid #ddd8d0", borderRadius:10, padding:"28px 20px", margin:"20px 0" }}>
              <div style={{ display:"flex", alignItems:"center", gap:4, flexWrap:"wrap", justifyContent:"center" }}>
                {STAGES.map((s,i) => (
                  <>
                    <PipelineNode key={s.id} stage={s} active={activeStage===i} onClick={()=>setActiveStage(i)} />
                    {i < STAGES.length-1 && <PipelineArrow key={"arr"+i} />}
                  </>
                ))}
              </div>

              {/* Stage detail */}
              <div style={{ marginTop:24, padding:"20px 24px", background:"#faf8f5", borderRadius:8, border:`1px solid ${STAGES[activeStage].color}44` }}>
                <div style={{ display:"flex", alignItems:"center", gap:10, marginBottom:12 }}>
                  <span style={{ fontSize:20 }}>{STAGES[activeStage].icon}</span>
                  <span style={{ fontFamily:"'Fraunces',serif", fontSize:18, fontWeight:600, color: STAGES[activeStage].color }}>
                    {STAGES[activeStage].label}
                  </span>
                </div>
                <P style={{ marginBottom:8 }}>{STAGES[activeStage].detail}</P>
                <Code>{STAGES[activeStage].example}</Code>
              </div>
            </div>

            <H3>Key Design Decisions</H3>
            <Table headers={["Decision","Rationale"]}
              rows={[
                ["Tree-walking interpreter",     "Simpler to implement and debug. Bytecode VM is planned for v1.0."],
                ["Error recovery in parser",     "Show all errors at once, not just the first."],
                ["Collected type diagnostics",   "Same reason — see all type errors in one pass."],
                ["Env chain for scoping",        "Each scope is a linked Env; lookup walks the chain. O(depth) per lookup."],
                ["Tagged runtime values",        "VInt, VStr etc. are Rust-style enums — safe, exhaustive dispatch."],
                ["Microtask queue for async",    "Mirrors the browser event loop model; async code is familiar."],
                ["Signal dependency tracking",   "Effects register themselves as the 'current observer' during evaluation; signals capture that reference."],
              ]}
            />
          </section>

          {/* ── Grammar ── */}
          <section id="sec-grammar">
            <H2>Formal Grammar (EBNF excerpt)</H2>
            <Code>{`program        = stmt* EOF
stmt           = let_decl | mut_decl | fn_decl | class_decl
               | expr_stmt | return_stmt | for_stmt | while_stmt
let_decl       = "let" IDENT (":" type)? "=" expr
mut_decl       = "mut" IDENT (":" type)? "=" expr
fn_decl        = "async"? "fn" IDENT type_params? "(" params? ")" ("->" type)? block
block          = "{" stmt* expr? "}"
expr           = pipe_expr
pipe_expr      = or_expr ("|>" call_expr)*
or_expr        = and_expr ("||" and_expr)*
and_expr       = eq_expr ("&&" eq_expr)*
eq_expr        = cmp_expr (("==" | "!=") cmp_expr)*
cmp_expr       = add_expr (("<" | ">" | "<=" | ">=") add_expr)*
add_expr       = mul_expr (("+" | "-") mul_expr)*
mul_expr       = exp_expr (("*" | "/" | "%") exp_expr)*
exp_expr       = unary_expr ("^" exp_expr)?
unary_expr     = ("!" | "-") unary_expr | call_expr
call_expr      = primary ("(" args? ")" | "." IDENT | "[" expr "]")*
primary        = INT | FLOAT | STR | BOOL | "None"
               | IDENT | list_lit | map_lit
               | "(" expr ")" | lambda | match_expr | if_expr
lambda         = "(" params? ")" "->" (expr | block)
match_expr     = "match" expr "{" match_arm+ "}"
match_arm      = pattern ("->" expr | block) ","?
pattern        = "_" | literal | IDENT | "Some" "(" pattern ")"
               | "Ok" "(" pattern ")" | "Err" "(" pattern ")"
               | "[" pattern* ("..." IDENT)? "]"
               | "{" field_pattern* "}"
type           = "Int" | "Float" | "Bool" | "Str" | "()"
               | IDENT ("<" type ("," type)* ">")?
               | "fn" "(" type* ")" "->" type`}</Code>
            <Note>This is a simplified excerpt. The full grammar includes operator precedence climbing, generic bounds, trait declarations, and module syntax.</Note>
          </section>

          <div style={{ height: 80 }} />
        </main>
      </div>
    </div>
  );
}
