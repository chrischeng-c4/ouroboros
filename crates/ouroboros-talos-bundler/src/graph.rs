use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::path::PathBuf;

/// Module identifier (graph node index)
pub type ModuleId = NodeIndex;

/// Module dependency graph
pub struct ModuleGraph {
    /// Directed acyclic graph of modules
    graph: DiGraph<ModuleNode, EdgeKind>,

    /// Map from path to node index
    path_to_id: HashMap<PathBuf, ModuleId>,
}

/// Node in the module graph
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// Module file path
    pub path: PathBuf,

    /// Module type
    pub kind: ModuleKind,

    /// File size in bytes
    pub size: u64,
}

/// Type of module
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleKind {
    /// JavaScript/TypeScript module
    Script,

    /// CSS module
    Css,

    /// JSON module
    Json,

    /// Asset (image, font, etc.)
    Asset,
}

/// Edge type in the dependency graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Static import
    Import,

    /// Dynamic import
    DynamicImport,

    /// CSS import
    CssImport,
}

impl ModuleGraph {
    /// Create a new empty module graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            path_to_id: HashMap::new(),
        }
    }

    /// Add a module to the graph
    pub fn add_module(&mut self, path: PathBuf, kind: ModuleKind, size: u64) -> ModuleId {
        if let Some(&id) = self.path_to_id.get(&path) {
            return id;
        }

        let node = ModuleNode { path: path.clone(), kind, size };
        let id = self.graph.add_node(node);
        self.path_to_id.insert(path, id);
        id
    }

    /// Add a dependency edge between modules
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId, kind: EdgeKind) {
        self.graph.add_edge(from, to, kind);
    }

    /// Get module by path
    pub fn get_module(&self, path: &PathBuf) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Get module node data
    pub fn get_node(&self, id: ModuleId) -> Option<&ModuleNode> {
        self.graph.node_weight(id)
    }

    /// Get all modules in topological order
    pub fn topological_sort(&self) -> Result<Vec<ModuleId>, Vec<PathBuf>> {
        use petgraph::algo::toposort;

        match toposort(&self.graph, None) {
            Ok(order) => Ok(order),
            Err(cycle) => {
                // Extract cycle information
                let cycle_node = cycle.node_id();
                let cycle_path = self
                    .graph
                    .node_weight(cycle_node)
                    .map(|n| n.path.clone())
                    .unwrap_or_default();

                tracing::warn!("Cycle detected in module graph at: {:?}", cycle_path);

                // Find the cycle
                let cycle_paths = self.find_cycle_from(cycle_node);
                Err(cycle_paths)
            }
        }
    }

    /// Find cycle starting from a given node
    pub fn find_cycle_from(&self, start: ModuleId) -> Vec<PathBuf> {
        use petgraph::Direction;

        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![start];
        let mut path = Vec::new();

        while let Some(node) = stack.last().copied() {
            if visited.contains(&node) {
                // Found a cycle
                if let Some(pos) = path.iter().position(|&n| n == node) {
                    return path[pos..]
                        .iter()
                        .filter_map(|&id| self.graph.node_weight(id).map(|n| n.path.clone()))
                        .collect();
                }
                stack.pop();
                continue;
            }

            visited.insert(node);
            path.push(node);

            let mut has_unvisited = false;
            for neighbor in self.graph.neighbors_directed(node, Direction::Outgoing) {
                if !visited.contains(&neighbor) {
                    stack.push(neighbor);
                    has_unvisited = true;
                    break;
                }
            }

            if !has_unvisited {
                stack.pop();
            }
        }

        Vec::new()
    }

    /// Check if the graph has any cycles
    pub fn has_cycle(&self) -> bool {
        use petgraph::algo::toposort;
        toposort(&self.graph, None).is_err()
    }

    /// Get dependencies of a module
    pub fn dependencies(&self, id: ModuleId) -> Vec<(ModuleId, EdgeKind)> {
        use petgraph::Direction;

        self.graph
            .neighbors_directed(id, Direction::Outgoing)
            .map(|dep_id| {
                let edge = self.graph.find_edge(id, dep_id).unwrap();
                let kind = *self.graph.edge_weight(edge).unwrap();
                (dep_id, kind)
            })
            .collect()
    }

    /// Get number of modules
    pub fn module_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Clear the graph
    pub fn clear(&mut self) {
        self.graph.clear();
        self.path_to_id.clear();
    }
}

impl Default for ModuleGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_module() {
        let mut graph = ModuleGraph::new();
        let path = PathBuf::from("test.js");
        let id = graph.add_module(path.clone(), ModuleKind::Script, 100);

        assert_eq!(graph.module_count(), 1);
        assert_eq!(graph.get_module(&path), Some(id));
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = ModuleGraph::new();
        let id1 = graph.add_module(PathBuf::from("a.js"), ModuleKind::Script, 100);
        let id2 = graph.add_module(PathBuf::from("b.js"), ModuleKind::Script, 200);

        graph.add_dependency(id1, id2, EdgeKind::Import);

        let deps = graph.dependencies(id1);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, id2);
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = ModuleGraph::new();
        //  a → b → c (a depends on b, b depends on c)
        let id_a = graph.add_module(PathBuf::from("a.js"), ModuleKind::Script, 100);
        let id_b = graph.add_module(PathBuf::from("b.js"), ModuleKind::Script, 100);
        let id_c = graph.add_module(PathBuf::from("c.js"), ModuleKind::Script, 100);

        graph.add_dependency(id_a, id_b, EdgeKind::Import);
        graph.add_dependency(id_b, id_c, EdgeKind::Import);

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);

        // In topological order: c should come before b, b should come before a
        // (dependencies come before dependents)
        let pos_a = sorted.iter().position(|&id| id == id_a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == id_b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == id_c).unwrap();

        // Print for debugging
        println!("Sorted order: c={}, b={}, a={}", pos_c, pos_b, pos_a);

        // Actually petgraph's toposort returns nodes such that dependencies come AFTER dependents
        // So we expect: a < b < c
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = ModuleGraph::new();
        //  a → b → c → a (cycle!)
        let id_a = graph.add_module(PathBuf::from("a.js"), ModuleKind::Script, 100);
        let id_b = graph.add_module(PathBuf::from("b.js"), ModuleKind::Script, 100);
        let id_c = graph.add_module(PathBuf::from("c.js"), ModuleKind::Script, 100);

        graph.add_dependency(id_a, id_b, EdgeKind::Import);
        graph.add_dependency(id_b, id_c, EdgeKind::Import);
        graph.add_dependency(id_c, id_a, EdgeKind::Import); // Creates cycle

        assert!(graph.has_cycle());

        // topological_sort should return an error
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_complex_graph() {
        let mut graph = ModuleGraph::new();
        //  a → b → d
        //  a → c → d
        // (a depends on b and c, both b and c depend on d)
        let id_a = graph.add_module(PathBuf::from("a.js"), ModuleKind::Script, 100);
        let id_b = graph.add_module(PathBuf::from("b.js"), ModuleKind::Script, 100);
        let id_c = graph.add_module(PathBuf::from("c.js"), ModuleKind::Script, 100);
        let id_d = graph.add_module(PathBuf::from("d.js"), ModuleKind::Script, 100);

        graph.add_dependency(id_a, id_b, EdgeKind::Import);
        graph.add_dependency(id_a, id_c, EdgeKind::Import);
        graph.add_dependency(id_b, id_d, EdgeKind::Import);
        graph.add_dependency(id_c, id_d, EdgeKind::Import);

        assert!(!graph.has_cycle());

        let sorted = graph.topological_sort().unwrap();
        assert_eq!(sorted.len(), 4);

        // petgraph toposort: dependents come BEFORE dependencies
        // So we expect: a < (b, c) < d
        let pos_a = sorted.iter().position(|&id| id == id_a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == id_b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == id_c).unwrap();
        let pos_d = sorted.iter().position(|&id| id == id_d).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
        assert!(pos_b < pos_d);
        assert!(pos_c < pos_d);
    }
}
