pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use std::collections::HashMap;

use async_trait::async_trait;
use petgraph::{algo::astar, graphmap::DiGraphMap, Direction};

use super::{
    list::ListEnvelopesOptions, Envelope, SingleId, ThreadedEnvelope, ThreadedEnvelopes,
};
use crate::AnyResult;

#[async_trait]
pub trait ThreadEnvelopes: Send + Sync {
    /// Thread all available envelopes from the given folder matching
    /// the given pagination.
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes>;

    async fn thread_envelope(
        &self,
        _folder: &str,
        _id: SingleId,
        _opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        unimplemented!()
    }
}

/// Build a thread graph from envelopes using In-Reply-To / Message-ID
/// relationships. This is a client-side threading algorithm that works
/// regardless of whether the server supports IMAP THREAD.
pub fn build_thread_graph_all(
    envelopes: &HashMap<String, Envelope>,
) -> DiGraphMap<ThreadedEnvelope<'_>, u8> {
    let msg_id_mapping: HashMap<_, _> = envelopes
        .values()
        .map(|e| (e.message_id.as_str(), e.id.as_str()))
        .collect();

    let mut graph = DiGraphMap::<&str, u8>::new();

    for envelope in envelopes.values() {
        match envelope.in_reply_to.as_ref() {
            Some(msg_id) => {
                if let Some(id) = msg_id_mapping.get(msg_id.as_str()) {
                    graph.add_edge(*id, envelope.id.as_str(), 0);
                }
            }
            None => {
                graph.add_edge("0", envelope.id.as_str(), 0);
            }
        };
    }

    let leafs: Vec<_> = graph
        .nodes()
        .filter(|node| graph.neighbors_directed(node, Direction::Outgoing).count() == 0)
        .collect();

    for leaf in leafs {
        if let Some((_, path)) = astar(&graph, "0", |n| n == leaf, |_| 0, |_| 0) {
            let mut pairs = path.windows(2).enumerate();
            while let Some((depth, [a, b])) = pairs.next() {
                graph[(*a, *b)] = depth as u8;
            }
        };
    }

    build_final_graph(envelopes, &graph)
}

/// Build a thread graph filtered to only the thread containing the
/// given envelope ID.
pub fn build_thread_graph_for_id<'a>(
    envelopes: &'a HashMap<String, Envelope>,
    id: &str,
) -> DiGraphMap<ThreadedEnvelope<'a>, u8> {
    let msg_id_mapping: HashMap<_, _> = envelopes
        .values()
        .map(|e| (e.message_id.as_str(), e.id.as_str()))
        .collect();

    let mut graph = DiGraphMap::<&str, u8>::new();

    for envelope in envelopes.values() {
        match envelope.in_reply_to.as_ref() {
            Some(msg_id) => {
                if let Some(parent_id) = msg_id_mapping.get(msg_id.as_str()) {
                    graph.add_edge(*parent_id, envelope.id.as_str(), 0);
                }
            }
            None => {
                graph.add_edge("0", envelope.id.as_str(), 0);
            }
        };
    }

    let leafs: Vec<_> = graph
        .nodes()
        .filter(|node| graph.neighbors_directed(node, Direction::Outgoing).count() == 0)
        .collect();

    let mut filtered = DiGraphMap::<&str, u8>::new();

    for leaf in leafs {
        if let Some((_, path)) = astar(&graph, "0", |n| n == leaf, |_| 0, |_| 0) {
            if path.contains(&id) {
                let mut pairs = path.windows(2).enumerate();
                while let Some((depth, [a, b])) = pairs.next() {
                    filtered.add_edge(*a, *b, depth as u8);
                }
            }
        };
    }

    build_final_graph(envelopes, &filtered)
}

fn build_final_graph<'a>(
    envelopes: &'a HashMap<String, Envelope>,
    graph: &DiGraphMap<&str, u8>,
) -> DiGraphMap<ThreadedEnvelope<'a>, u8> {
    let mut final_graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

    for (a, b, w) in graph.all_edges() {
        let eb = envelopes.get(&b.to_string()).unwrap();
        match envelopes.get(&a.to_string()) {
            Some(ea) => {
                final_graph.add_edge(ea.as_threaded(), eb.as_threaded(), *w);
            }
            None => {
                let ea = ThreadedEnvelope {
                    id: "0",
                    message_id: "0",
                    subject: "",
                    from: "",
                    date: Default::default(),
                };
                final_graph.add_edge(ea, eb.as_threaded(), *w);
            }
        }
    }

    final_graph
}
