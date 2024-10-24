use std::collections::HashMap;

use async_trait::async_trait;
use petgraph::{algo::astar, graphmap::DiGraphMap, Direction};
use tracing::instrument;

use super::ThreadEnvelopes;
use crate::{
    envelope::{
        list::ListEnvelopesOptions, Envelopes, SingleId, ThreadedEnvelope, ThreadedEnvelopes,
    },
    maildir::MaildirContextSync,
    AnyResult, Error,
};

#[derive(Clone)]
pub struct ThreadMaildirEnvelopes {
    ctx: MaildirContextSync,
}

impl ThreadMaildirEnvelopes {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ThreadEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ThreadEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ThreadEnvelopes for ThreadMaildirEnvelopes {
    #[instrument(skip(self, opts))]
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir.read().map_err(Error::MaildirsError)?;
        let envelopes = Envelopes::from_mdir_entries(entries, opts.query.as_ref())
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();

        let envelopes = ThreadedEnvelopes::new(envelopes, move |envelopes| {
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
        });

        Ok(envelopes)
    }

    #[instrument(skip(self, opts))]
    async fn thread_envelope(
        &self,
        folder: &str,
        id: SingleId,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir.read().map_err(Error::MaildirsError)?;
        let envelopes = Envelopes::from_mdir_entries(entries, opts.query.as_ref())
            .into_iter()
            .map(|e| (e.id.clone(), e))
            .collect();

        let envelopes = ThreadedEnvelopes::new(envelopes, move |envelopes| {
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

            let mut graph2 = DiGraphMap::<&str, u8>::new();

            for leaf in leafs {
                if let Some((_, path)) = astar(&graph, "0", |n| n == leaf, |_| 0, |_| 0) {
                    if path.contains(&&id.as_str()) {
                        let mut pairs = path.windows(2).enumerate();
                        while let Some((depth, [a, b])) = pairs.next() {
                            graph2.add_edge(*a, *b, depth as u8);
                        }
                    }
                };
            }

            let mut final_graph = DiGraphMap::<ThreadedEnvelope, u8>::new();

            for (a, b, w) in graph2.all_edges() {
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
        });

        Ok(envelopes)
    }
}
