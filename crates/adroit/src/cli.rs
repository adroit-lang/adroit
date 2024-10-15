use std::{
    collections::HashMap, fs, io, marker::PhantomData, ops::Range, path::PathBuf, sync::Arc,
    time::Instant,
};

use ariadne::{Cache, Color, Label, Report, ReportBuilder, ReportKind, Source};
use clap::{Parser, Subcommand};
use itertools::Itertools;
use serde::Serialize;

use crate::{
    compile::{FullModule, GraphImporter, Printer},
    fetch::fetch,
    graph::{Analysis, Data, Graph, Syntax, Uri},
    lex::{lex, Tokens},
    lsp::language_server,
    parse::{self, ParseError},
    pprint::pprint,
    typecheck,
    util::{Diagnostic, Emitter},
};

fn stdlib() -> Uri {
    let dir = dirs::cache_dir()
        .expect("cache directory should exist")
        .join("adroit/modules");
    Uri::from_directory_path(dir).unwrap()
}

fn rooted_graph(path: PathBuf) -> Result<(Graph, Uri), ()> {
    let mut graph = Graph::new(stdlib());
    let canon = path
        .canonicalize()
        .map_err(|err| eprintln!("error canonicalizing {}: {err}", path.display()))?;
    let uri = Uri::from_file_path(canon).unwrap();
    graph.make_root(uri.clone());
    Ok((graph, uri))
}

#[derive(Debug)]
struct AriadneEmitter<'a, C> {
    cache: C,
    message: String,
    phantom: PhantomData<&'a ()>,
}

impl<'a, C: Cache<&'a str>> AriadneEmitter<'a, C> {
    fn new(cache: C, message: impl ToString) -> Self {
        Self {
            cache,
            message: message.to_string(),
            phantom: PhantomData,
        }
    }
}

impl<'a, C: Cache<&'a str>> Emitter<(&'a str, Range<usize>)> for AriadneEmitter<'a, C> {
    fn diagnostic(
        &mut self,
        span: (&'a str, Range<usize>),
        message: impl ToString,
    ) -> impl Diagnostic<(&'a str, Range<usize>)> {
        let (path, range) = span.clone();
        AriadneDiagnostic {
            cache: &mut self.cache,
            builder: Report::build(ReportKind::Error, path, range.start)
                .with_message(&self.message)
                .with_label(
                    Label::new(span)
                        .with_color(Color::Red)
                        .with_message(message),
                ),
        }
    }
}

#[derive(Debug)]
struct AriadneDiagnostic<'a, 'b, C: Cache<&'a str>> {
    cache: &'b mut C,
    builder: ReportBuilder<'a, (&'a str, Range<usize>)>,
}

impl<'a, 'b, C: Cache<&'a str>> Diagnostic<(&'a str, Range<usize>)>
    for AriadneDiagnostic<'a, 'b, C>
{
    fn related(mut self, span: (&'a str, Range<usize>), message: impl ToString) -> Self {
        self.builder.add_label(
            Label::new(span)
                .with_color(Color::Blue)
                .with_message(message),
        );
        self
    }

    fn finish(self) {
        self.builder.finish().eprint(self.cache).unwrap();
    }
}

fn read<'a>(graph: &'a mut Graph, uri: &Uri) -> Result<&'a Syntax, ()> {
    let text = fetch(graph.stdlib(), uri).map_err(|err| eprintln!("{}", err))?;
    graph.set_text(uri, text);
    let uri_str = uri.as_str();
    match &graph.get(uri).data {
        Data::Read { src, err } => {
            AriadneEmitter::new((uri_str, Source::from(&src.text)), "failed to tokenize")
                .diagnostic((uri_str, err.byte_range()), err.message())
                .finish();
            Err(())
        }
        Data::Lexed { src, toks, err } => {
            let id = match *err {
                ParseError::Expected { id, kinds: _ } => id,
            };
            AriadneEmitter::new((uri_str, Source::from(&src.text)), "failed to parse")
                .diagnostic((uri_str, toks.get(id).byte_range()), err.message())
                .finish();
            Err(())
        }
        Data::Parsed { syn } => Ok(syn),
        _ => unreachable!(),
    }
}

fn analyze(graph: &mut Graph, job: Analysis) -> Result<(), ()> {
    let (uri, syn, deps) = &job;
    let (module, errs) = typecheck::typecheck(
        &syn.src.text,
        &syn.toks,
        &syn.tree,
        deps.iter().map(|(_, dep)| dep.as_ref()).collect(),
    );
    let sem = Arc::new(module);
    if errs.is_empty() {
        graph.supply_semantic(job, sem, errs);
        Ok(())
    } else {
        let uri_str = uri.as_str();
        let uris: Vec<Uri> = deps.iter().map(|(import, _)| import.clone()).collect();
        let full = FullModule {
            source: &syn.src.text,
            tokens: &syn.toks,
            tree: &syn.tree,
            module: sem,
        };
        let printer = Printer::new(full, GraphImporter { graph, uris: &uris });
        let mut emitter = AriadneEmitter::new(
            (uri_str, Source::from(&syn.src.text)),
            "failed to typecheck",
        );
        for err in errs {
            printer.emit_type_error(&mut emitter, uri_str, err);
        }
        Err(())
    }
}

fn exhaust(graph: &mut Graph) -> Result<(), ()> {
    loop {
        let pending = graph.pending();
        if pending.is_empty() {
            break;
        }
        for uri in pending {
            read(graph, &uri)?;
        }
    }
    loop {
        let analysis = graph.analysis();
        if analysis.is_empty() {
            break;
        }
        for job in analysis {
            analyze(graph, job)?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct FullNode<'a> {
    pub source: &'a str,
    pub tokens: &'a Tokens,
    pub tree: &'a parse::Module,
    pub imports: Vec<Uri>,
    pub module: Arc<typecheck::Module>,
}

#[derive(Debug, Serialize)]
struct Modules<'a> {
    root: Uri,
    modules: HashMap<&'a Uri, FullNode<'a>>,
}

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Lex a source file
    Lex {
        /// Number of times to lex
        #[arg(short)]
        n: usize,
        file: PathBuf,
    },

    /// Print the reformatted source code of a module
    Fmt { file: PathBuf },

    /// Print the typed IR of a module as JSON
    Json { file: PathBuf },

    /// Start a language server over stdio
    Lsp,
}

pub fn cli() -> Result<(), ()> {
    match Cli::parse().command {
        Commands::Lex { n, file } => {
            let source = fs::read_to_string(&file).map_err(|err| {
                eprintln!("error reading {}: {err}", file.display());
            })?;
            let bytes = source.len();
            println!("{n} iterations");
            println!("{n} * {bytes} = {} bytes", n * bytes);
            let lines = source.lines().count();
            println!("{n} * {lines} = {} lines", n * lines);
            let tokens = lex(&source).map_err(|_| eprintln!("failed to lex"))?.len();
            println!("{n} * {tokens} = {} tokens", n * tokens);
            let start = Instant::now();
            for _ in 0..n {
                lex(&source).unwrap();
            }
            let end = Instant::now();
            let elapsed = end - start;
            println!("{elapsed:?}");
            println!("{:?} per byte", elapsed / (n * bytes) as u32);
            println!("{:?} per line", elapsed / (n * lines) as u32);
            println!("{:?} per token", elapsed / (n * tokens) as u32);
            let seconds = elapsed.as_secs_f64();
            println!("{} bytes per second", (n * bytes) as f64 / seconds);
            println!("{} lines per second", (n * lines) as f64 / seconds);
            println!("{} tokens per second", (n * tokens) as f64 / seconds);
            Ok(())
        }
        Commands::Fmt { file } => {
            let (mut graph, _) = rooted_graph(file)?;
            let (uri,) = graph.pending().into_iter().collect_tuple().unwrap();
            let syn = read(&mut graph, &uri)?;
            pprint(&mut io::stdout(), &syn.src.text, &syn.toks, &syn.tree)
                .map_err(|err| eprintln!("error formatting module: {err}"))
        }
        Commands::Json { file } => {
            let (mut graph, root) = rooted_graph(file)?;
            exhaust(&mut graph)?;
            let modules = graph
                .nodes()
                .map(|(uri, node)| match &node.data {
                    Data::Analyzed { syn, sem, errs } => {
                        assert!(errs.is_empty());
                        let full = FullNode {
                            source: &syn.src.text,
                            tokens: &syn.toks,
                            tree: &syn.tree,
                            imports: graph.imports(uri)?,
                            module: sem.clone(),
                        };
                        Ok((uri, full))
                    }
                    _ => Err(()),
                })
                .collect::<Result<HashMap<&Uri, FullNode>, ()>>()
                .map_err(|()| {
                    eprintln!("cyclic import");
                })?;
            serde_json::to_writer(io::stdout(), &Modules { root, modules })
                .map_err(|err| eprintln!("error serializing modules: {err}"))?;
            println!();
            Ok(())
        }
        Commands::Lsp => language_server(stdlib()),
    }
}
