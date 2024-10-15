use std::collections::HashMap;

use crate::{
    graph::{Graph, Node, Uri},
    ir,
};

fn uri_to_id(uri: &Uri) -> ir::ModuleId {
    ir::ModuleId(Box::from(uri.as_str()))
}

#[derive(Debug)]
struct Lowerer {
    graph: Graph,
    modules: HashMap<ir::ModuleId, ir::Module>,
}

impl Lowerer {
    fn module(uri: &Uri, node: &Node) -> Result<(), ()> {
        Ok(())
    }
}

fn lower(graph: Graph) -> Result<Vec<ir::Module>, ()> {
    let mut modules: HashMap<ir::ModuleId, ir::Module> = HashMap::new();
    for (uri, node) in graph.nodes() {
        let id = uri_to_id(uri);
        let imports: Vec<ir::Import> = graph
            .imports(uri)?
            .into_iter()
            .map(|uri| ir::Import {
                id: uri_to_id(&uri),
            })
            .collect();
        let module = ir::Module {
            id,
            imports: imports.into_boxed_slice(),
            typevars: todo!(),
            types: todo!(),
            functions: todo!(),
        };
        modules.insert(id, module).unwrap();
    }
    Ok(modules.into_values().collect())
}
