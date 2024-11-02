use proc_macro2::{Group, Literal, TokenStream, TokenTree};
use quote::{quote, ToTokens, TokenStreamExt};

enum Arg {
    Literal(String),
    Expr(TokenStream),
}

impl ToTokens for Arg {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Arg::Literal(s) => tokens.append(Literal::string(s)),
            Arg::Expr(e) => tokens.append_all(e.into_token_stream()),
        }
    }
}

enum ParseState {
    Cmd,
    Args,
    SetSink,
    DoneSetSink,
    SetSource,
    DoneSetSource,
}

enum Sink {
    File(String),
    Expr(TokenStream),
}

enum Source {
    File(String),
    Expr(TokenStream),
}

struct CmdParser {
    state: ParseState,
    cmd: Option<String>,
    args: Vec<Arg>,
    sink: Option<Sink>,
    source: Option<Source>,
}

struct Cmd {
    cmd: String,
    args: Vec<Arg>,
    sink: Option<Sink>,
    source: Option<Source>,
}

#[derive(Debug)]
enum CmdTokenTree {
    Value(String),
    EndOfLine,
    Expr(Group),
    Sink,
    Source,
}

impl From<TokenTree> for CmdTokenTree {
    fn from(value: TokenTree) -> Self {
        match value {
            TokenTree::Group(g) => CmdTokenTree::Expr(g),
            TokenTree::Ident(value) => CmdTokenTree::Value(value.to_string()),
            TokenTree::Punct(c) if c.as_char() == ';' => CmdTokenTree::EndOfLine,
            TokenTree::Punct(c) if c.as_char() == '>' => CmdTokenTree::Sink,
            TokenTree::Punct(c) if c.as_char() == '<' => CmdTokenTree::Source,
            TokenTree::Punct(c) => panic!("Unexpected punctuation character: {c}"),
            TokenTree::Literal(value) => {
                let literal = litrs::Literal::from(value);
                let value = match literal {
                    litrs::Literal::Bool(b) => b.to_string(),
                    litrs::Literal::Integer(i) => i.to_string(),
                    litrs::Literal::Float(f) => f.to_string(),
                    litrs::Literal::Char(_c) => {
                        unimplemented!("Character literals are not implemented")
                    }
                    litrs::Literal::String(s) => s.into_value().into_owned(),
                    litrs::Literal::Byte(_b) => unimplemented!("Byte literals are not implemented"),
                    litrs::Literal::ByteString(_s) => {
                        unimplemented!("Byte literals are not implemented")
                    }
                };
                ShTokenTree::Value(value)
            }
        }
    }
}

#[must_use]
enum ParseResult {
    KeepGoing,
    Done(Cmd),
}

impl Default for CmdParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CmdParser {
    pub fn new() -> Self {
        Self {
            state: ParseState::Cmd,
            cmd: None,
            args: Vec::new(),
            sink: None,
            source: None,
        }
    }

    pub fn feed(&mut self, token: TokenTree) -> ParseResult {
        let token = ShTokenTree::from(token);
        match self.state {
            ParseState::Cmd => {
                let CmdTokenTree::Value(value) = token else {
                    panic!("Unexpected command: {token:?}");
                };
                self.cmd = Some(value);
                self.state = ParseState::Args;
            }
            ParseState::Args => match token {
                CmdTokenTree::Value(v) => self.args.push(Arg::Literal(v)),
                CmdTokenTree::EndOfLine => return ParseResult::Done(self.take()),
                CmdTokenTree::Expr(g) => self.args.push(Arg::Expr(g.stream())),
                CmdTokenTree::Sink => self.state = ParseState::SetSink,
                CmdTokenTree::Source => self.state = ParseState::SetSource,
            },
            ParseState::SetSink => {
                assert!(self.sink.is_none(), "Can't set the sink more than once");
                match token {
                    CmdTokenTree::Value(v) => self.sink = Some(Sink::File(v)),
                    CmdTokenTree::Expr(g) => self.sink = Some(Sink::Expr(g.stream())),
                    other => panic!("Unexpected token: {other:?}"),
                }
                self.state = ParseState::DoneSetSink;
            }
            ParseState::SetSource => {
                assert!(self.source.is_none(), "Can't set the source more than once");
                match token {
                    CmdTokenTree::Value(v) => self.source = Some(Source::File(v)),
                    CmdTokenTree::Expr(g) => {
                        self.source = Some(Source::Expr(g.stream()));
                    }
                    other => panic!("Unexpected token: {other:?}"),
                }
                self.state = ParseState::DoneSetSource;
            }
            ParseState::DoneSetSink => match token {
                CmdTokenTree::EndOfLine => return ParseResult::Done(self.take()),
                CmdTokenTree::Source => self.state = ParseState::SetSource,
                other => panic!("Unexpected token: {other:?}"),
            },
            ParseState::DoneSetSource => match token {
                CmdTokenTree::EndOfLine => return ParseResult::Done(self.take()),
                CmdTokenTree::Sink => self.state = ParseState::SetSink,
                other => panic!("Unexpected token: {other:?}"),
            },
        }
        ParseResult::KeepGoing
    }

    fn finish(mut self) -> Option<Cmd> {
        if self.cmd.is_some() {
            Some(self.take())
        } else {
            None
        }
    }

    fn take(&mut self) -> Cmd {
        let mut parser = std::mem::take(self);
        Cmd {
            cmd: parser.cmd.take().expect("Missing command"),
            args: parser.args,
            sink: parser.sink,
            source: parser.source,
        }
    }
}

/// A command-running macro.
///
/// `cmd` is a macro for running external commands. It provides functionality to
/// pipe the input and output to/from variables as well as using rust expressions
/// as arguments to the program.
///
/// The format of a `cmd` call is like so:
///
/// ```ignore
/// lex!( [prog] [arg]* [< {in_expr}]? [> {out_expr}]? [;]? )
/// ```
///
/// Or you can create multiple commands on a single block
///
/// ```ignore
/// lex! {
///   [prog] [arg]* [< {in_expr}]? [> {out_expr}]? ;
///   [prog] [arg]* [< {in_expr}]? [> {out_expr}]? ;
///   [prog] [arg]* [< {in_expr}]? [> {out_expr}]? [;]?
/// }
/// ```
///
/// Arguments are allowed to take the form of identifiers (i.e. plain text),
/// literals (numbers, quoted strings, characters, etc.), or rust expressions
/// delimited by braces.
///
/// This macro doesn't execute the commands. It returns an iterator of `shx::Cmd` which
/// can be executed. Alternatively, see `shx::sh` which executes the commands sequentially.
///
/// # Examples
///
/// ```ignore
/// # use shx_macros::lex;
/// # #[cfg(target_os = "linux")]
/// # fn run() {
/// let world = "world";
/// let mut out = String::new();
/// lex!(echo hello {world} > {&mut out}).for_each(|cmd| cmd.exec().unwrap());
/// assert_eq!(out, "hello world\n");
/// # }
/// # run();
/// ```
///
/// ```ignore
/// # use shx_macros::lex;
/// # #[cfg(target_os = "linux")]
/// # fn run() {
/// lex! {
///   echo hello;
///   sleep 5;
///   echo world;
/// }.for_each(|cmd| cmd.exec().unwrap()); // prints hello, waits 5 seconds, prints world.
/// # }
/// # run();
/// ```
///
/// You can also use string literals as needed
///
/// ```ignore
/// # use shx_macros::lex;
/// # #[cfg(target_os = "linux")]
/// # fn run() {
/// let mut out = String::new();
/// lex!(echo "hello world" > {&mut out}).for_each(|cmd| cmd.exec().unwrap());
/// assert_eq!(out, "hello world\n");
/// # }
/// # run();
/// ```
#[proc_macro]
pub fn lex(stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let stream: TokenStream = stream.into();
    let stream = stream.into_iter();
    let mut cmd_list: Vec<Cmd> = Vec::new();

    let mut parser = CmdParser::new();
    for token in stream {
        match parser.feed(token) {
            ParseResult::KeepGoing => {}
            ParseResult::Done(sh) => cmd_list.push(sh),
        }
    }
    if let Some(cmd) = parser.finish() {
        cmd_list.push(cmd);
    }

    quote!(
        {
            let mut __commands = Vec::new();
            #(
                __commands.push({
                    #cmd_list
                });
            )*
            __commands.into_iter()
        }
    )
    .into()
}

impl ToTokens for Cmd {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            cmd,
            args,
            sink,
            source,
            ..
        } = self;

        tokens.append_all(quote! {
            let mut __cmd = ::std::process::Command::new(#cmd);
            #(
                __cmd.arg({ #args });
            )*
            let mut __builder = ::shx::CmdBuilder::new(__cmd);
        });

        match sink {
            Some(Sink::File(path)) => {
                unimplemented!("Writing command output to file is not yet implemented: {path:?}")
            }
            Some(Sink::Expr(expr)) => tokens.append_all(quote! {
                __builder.sink({ #expr });
            }),
            None => {}
        }
        match source {
            Some(Source::File(path)) => {
                unimplemented!("Reading command input from file is not yet implemented: {path:?}");
            }
            Some(Source::Expr(expr)) => tokens.append_all(quote! {
                __builder.source({ #expr });
            }),
            None => {}
        }

        tokens.append_all(quote! {
            __builder.build()
        })
    }
}
